#!/usr/bin/env python3
"""Regression checks: optimized Python must not disable deployment verification gates."""
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

class VerificationGateTests(unittest.TestCase):
    def test_stale_review_fails_under_optimized_python(self):
        with tempfile.TemporaryDirectory(prefix="oppor-audit-") as temporary:
            root = Path(temporary) / "contracts"
            (root / "scripts").mkdir(parents=True)
            shutil.copytree(ROOT / "src", root / "src")
            script = root / "scripts/security-check.py"
            shutil.copy(ROOT / "scripts/security-check.py", script)
            review = json.loads((ROOT / "security-review.json").read_text())
            review["source_sha256"]["CampaignEscrow.sol"] = "00" * 32
            (root / "security-review.json").write_text(json.dumps(review))
            result = subprocess.run([sys.executable, "-O", str(script)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Sources changed", result.stderr)

    def test_stale_export_fails_under_optimized_python(self):
        with tempfile.TemporaryDirectory(prefix="oppor-audit-") as temporary:
            root = Path(temporary) / "contracts"
            (root / "scripts").mkdir(parents=True)
            (root / "out").mkdir()
            shutil.copytree(ROOT / "src", root / "src")
            shutil.copytree(ROOT / "abi", root / "abi")
            for name in ("CampaignFactory", "CampaignEscrow"):
                shutil.copytree(ROOT / "out" / f"{name}.sol", root / "out" / f"{name}.sol")
            (root / "abi/CampaignFactory.json").write_text("[]\n")
            script = root / "scripts/export-artifacts.py"
            shutil.copy(ROOT / "scripts/export-artifacts.py", script)
            result = subprocess.run([sys.executable, "-O", str(script), "--check"], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Stale exported artifact", result.stderr)

if __name__ == "__main__":
    unittest.main()
