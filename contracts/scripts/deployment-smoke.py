#!/usr/bin/env python3
"""Exercise deployment simulation and network/sender guards on an owned temporary node."""
import json
import socket
import subprocess
import time
import urllib.request
from pathlib import Path

root = Path(__file__).resolve().parent.parent
with socket.socket() as reservation:
    reservation.bind(("127.0.0.1", 0))
    port = reservation.getsockname()[1]
url = f"http://127.0.0.1:{port}"
node = subprocess.Popen(["anvil", "--silent", "--host", "127.0.0.1", "--port", str(port), "--chain-id", "5042"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

def rpc(method):
    request = urllib.request.Request(url, data=json.dumps({"jsonrpc":"2.0", "id":1, "method":method, "params":[]}).encode(), headers={"Content-Type":"application/json"})
    with urllib.request.urlopen(request, timeout=2) as response:
        result = json.load(response)
    assert "error" not in result, result
    return result["result"]

try:
    for _ in range(100):
        if node.poll() is not None:
            raise RuntimeError("Owned Anvil process exited before becoming ready")
        try:
            accounts = rpc("eth_accounts")
            break
        except OSError:
            time.sleep(0.05)
    else:
        raise RuntimeError("Anvil did not become ready")
    base = ["forge", "script", "script/Deploy.s.sol:Deploy", "--rpc-url", url, "--sig", "run(uint256,address)"]
    for chain, sender, failure in (("1", accounts[0], "WrongChain"), ("5042", "0x" + "00"*20, "InvalidDeployer")):
        result = subprocess.run(base + [chain, sender], cwd=root, capture_output=True, text=True, timeout=120)
        assert result.returncode != 0 and failure in result.stdout + result.stderr, result.stdout + result.stderr
    result = subprocess.run(base + ["5042", accounts[0]], cwd=root, capture_output=True, text=True, timeout=120)
    assert result.returncode == 0, result.stdout + result.stderr
    assert rpc("eth_blockNumber") == "0x0", "Simulation unexpectedly changed node state"
    print("Deployment smoke passed: wrong-chain rejection, zero-sender rejection, successful simulation, no broadcast.")
finally:
    node.terminate()
    try:
        node.wait(timeout=5)
    except subprocess.TimeoutExpired:
        node.kill()
        node.wait()
