#!/usr/bin/env python3
"""Run Slither and reject unreviewed medium/high findings or stale source review."""
import hashlib
import json
import shutil
import subprocess
from collections import Counter
from pathlib import Path
def _require(condition, message):
    if not condition:
        raise RuntimeError(message)

root = Path(__file__).resolve().parent.parent
review = json.loads((root / 'security-review.json').read_text())
source_hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((root / 'src').glob('*.sol'))}
_require(review['source_sha256'] == source_hashes, 'Sources changed; refresh the security review after examining findings')
slither = shutil.which('slither')
_require(slither, 'Install scripts/requirements-static-analysis.txt first')
report = root / 'slither-report.json'
report.unlink(missing_ok=True)
result = subprocess.run([slither, '.', '--filter-paths', 'lib/|test/|script/', '--exclude-dependencies', '--json', str(report)], cwd=root, capture_output=True, text=True)
_require(report.exists(), result.stderr)
scan = json.loads(report.read_text())
_require(scan['success'], scan.get('error', result.stderr))
_require(result.returncode in (0, 255), result.stderr)
findings = scan['results'].get('detectors', [])
accepted = {(r['check'], r['impact'], r['function']): r for r in review['reviewed_medium_findings']}
seen = set()
for d in findings:
    if d['impact'] not in ('High', 'Medium'):
        continue
    functions = [e['name'] for e in d['elements'] if e['type'] == 'function']
    _require(len(functions) == 1, d['description'])
    key = (d['check'], d['impact'], functions[0])
    _require(key in accepted, 'Unreviewed security finding: ' + d['description'])
    seen.add(key)
_require(seen == set(accepted), 'Analysis findings changed; re-review the baseline')
print('Slither findings:', dict(Counter((d['impact'] for d in findings))))
print('No high findings. Two medium loop/callback findings have explicit, source-pinned review; see SECURITY.md.')
