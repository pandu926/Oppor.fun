# Contract verification

The release passed **68 Solidity tests**, including audit regressions and stateful invariants, before deployment to Arc mainnet.

## Reproducible checks

```sh
bash scripts/bootstrap.sh
forge fmt --check
forge test
python3 scripts/deployment-smoke.py
python3 scripts/export-artifacts.py --check
python3 scripts/test-security-gates.py
```

Static analysis requires `scripts/requirements-static-analysis.txt` and runs through `scripts/security-check.py`. The reviewed detector baseline is pinned to source hashes. Source changes or unreviewed medium/high findings fail the gate. Regression checks also exercise stale-review and stale-export failure paths under optimized Python.

The backend integration suite deploys these contracts to Anvil and exercises real contract interactions. Live funded campaign-to-claim verification was performed on Arc testnet. Mainnet deployment checks verified creation input and runtime bytecode against the release artifact.

See [project verification](../docs/VERIFICATION.md), [deployment](DEPLOYMENT.md), and [security](SECURITY.md). These checks are internal validation, not an independent security audit or penetration test.
