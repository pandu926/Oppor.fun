#!/usr/bin/env python3
"""Fail on advisories in the actual normal/build dependency graph, without a blanket ignore list."""
import json
import re
import subprocess
import sys

report = subprocess.run(["cargo", "audit", "--json"], capture_output=True, text=True)
if report.returncode not in (0, 1):
    sys.stderr.write(report.stderr)
    sys.exit(report.returncode)
try:
    audit = json.loads(report.stdout)
except json.JSONDecodeError:
    sys.stderr.write(report.stderr)
    sys.exit(2)
tree = subprocess.run(
    ["cargo", "tree", "--locked", "--target", "all", "--edges", "normal,build", "--prefix", "none", "--format", "{p}"],
    capture_output=True, text=True, check=True,
)
active = set()
for line in tree.stdout.splitlines():
    match = re.match(r"^(\S+) v(\S+)", line)
    if match:
        active.add(match.groups())
failures = []
for item in audit.get("vulnerabilities", {}).get("list", []):
    package, advisory = item["package"], item["advisory"]
    used = (package["name"], package["version"]) in active
    print(f'{"ACTIVE" if used else "LOCKFILE ONLY"}: {advisory["id"]} {package["name"]} {package["version"]}: {advisory["title"]}')
    if used:
        failures.append(item)
for category, items in audit.get("warnings", {}).items():
    for item in items:
        package = item["package"]
        used = (package["name"], package["version"]) in active
        print(f'{"ACTIVE" if used else "LOCKFILE ONLY"} {category}: {package["name"]} {package["version"]}')
        if used and category in ("unsound", "yanked"):
            failures.append(item)
print(f"Active vulnerable/unsound/yanked dependencies: {len(failures)}")
sys.exit(bool(failures))
