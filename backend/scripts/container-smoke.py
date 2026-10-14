#!/usr/bin/env python3
"""Smoke-test the built image against the isolated Compose test services only."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parent.parent
os.chdir(root)
image = sys.argv[1] if len(sys.argv) > 1 else "oppor-backend:verification"
suffix = uuid.uuid4().hex
schema = "container_" + suffix
names = ["oppor-api-smoke-" + suffix, "oppor-worker-smoke-" + suffix]
env_file = root / "target" / ("container-" + suffix + ".env")
def command(args):
    return subprocess.run(args, capture_output=True, text=True, check=True, timeout=60).stdout
def sql(statement):
    command(["docker", "compose", "-f", "compose.test.yml", "exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1", "-U", "oppor", "-d", "oppor", "-c", statement])
def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]
port = free_port()
unused_rpc = socket.socket()
unused_rpc.bind(("127.0.0.1", 0))  # Reserved but not listening: no unrelated RPC is contacted.
environment = {
    "APP_ENV": "local", "HTTP_BIND": f"127.0.0.1:{port}",
    "DATABASE_URL": f"postgres://oppor:oppor-local-tests@127.0.0.1:55491/oppor?options=-csearch_path%3D{schema}",
    "REDIS_URL": "redis://127.0.0.1:56389/0", "PUBLIC_ORIGIN": "http://localhost:3000",
    "ARC_CHAIN_ID": "5042", "ARC_RPC_URL": f"http://127.0.0.1:{unused_rpc.getsockname()[1]}",
    "FACTORY_ADDRESS": "0x" + "11" * 20, "FACTORY_CODE_HASH": "0x" + "22" * 32,
    "ESCROW_CODE_HASH": "0x" + "33" * 32, "RAFFLE_SEED_ENCRYPTION_KEY": "44" * 32,
    "RATE_LIMIT_KEY": "55" * 32, "OBJECT_STORAGE_ENDPOINT": "http://127.0.0.1:59009",
    "OBJECT_STORAGE_BUCKET": "oppor-container-smoke", "OBJECT_STORAGE_REGION": "us-east-1",
    "OBJECT_STORAGE_ACCESS_KEY": "oppor-local", "OBJECT_STORAGE_SECRET_KEY": "oppor-local-storage-tests",
}
try:
    sql(f"CREATE SCHEMA {schema}")
    env_file.parent.mkdir(exist_ok=True)
    descriptor = os.open(env_file, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    with os.fdopen(descriptor, "w") as handle:
        handle.write("".join(f"{key}={value}\n" for key, value in environment.items()))
    user = command(["docker", "image", "inspect", "--format", "{{.Config.User}}", image]).strip()
    assert user == "10001:10001", user
    production = subprocess.run(["docker", "run", "--rm", "-e", "PUBLIC_ORIGIN=http://localhost:3000", image], capture_output=True, text=True, timeout=30)
    assert production.returncode != 0 and "Production requires HTTPS" in production.stderr
    common = ["--network", "host", "--env-file", str(env_file), "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true"]
    command(["docker", "run", "--rm", *common, image, "migrate"])
    command(["docker", "run", "-d", "--name", names[0], *common, image])
    command(["docker", "run", "-d", "--name", names[1], *common, "--entrypoint", "oppor-worker", image])
    live = f"http://127.0.0.1:{port}/v1/health/live"
    for attempt in range(100):
        try:
            with urllib.request.urlopen(live, timeout=2) as response:
                assert response.status == 200
                assert json.load(response)["status"] == "ok"
            break
        except (urllib.error.URLError, ConnectionError):
            if attempt == 99:
                raise
            time.sleep(0.1)
    try:
        urllib.request.urlopen(f"http://127.0.0.1:{port}/v1/health/ready", timeout=3)
        raise AssertionError("Readiness must reject the unavailable indexer")
    except urllib.error.HTTPError as error:
        assert error.code == 503
    try:
        urllib.request.urlopen(f"http://127.0.0.1:{port}/v1/admin/me", timeout=3)
        raise AssertionError("Administrative access must require authentication")
    except urllib.error.HTTPError as error:
        assert error.code == 401
    for name in names:
        command(["docker", "stop", "--time", "35", name])
        assert command(["docker", "inspect", "--format", "{{.State.ExitCode}}", name]).strip() == "0"
    print("Container smoke passed: non-root, read-only, migration, API liveness, fail-closed readiness, protected admin routing, API/worker graceful shutdown, and secure production defaults.")
finally:
    for name in names:
        subprocess.run(["docker", "rm", "-f", name], capture_output=True, timeout=40)
    env_file.unlink(missing_ok=True)
    unused_rpc.close()
    sql(f"DROP SCHEMA IF EXISTS {schema} CASCADE")
