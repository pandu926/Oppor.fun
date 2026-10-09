# Verification record

Date: 2026-10-09. Local Linux workspace, Foundry v1.5.1, Solidity 0.8.30, Paris EVM target, optimizer 200 runs, IR compilation, no metadata bytecode hash. Dependencies are pinned to OpenZeppelin Contracts 5.4.0 and forge-std 1.9.7 by full Git commit.

| Check | Result |
| --- | --- |
| Solidity compilation and runtime-size checks | Passed; escrow 10,387 bytes, factory 13,595 bytes, both below 24,576-byte EIP-170 limit |
| Foundry tests | 68 tests passed across four suites; zero failures |
| Unit/fuzz coverage | 66 tests, including three fuzz tests configured for 512 runs each |
| Stateful invariant coverage | Two invariants, each 128 runs × 64 actions = 8,192 actions; no action reverts |
| Configuration/root invariant | Passed across randomized claims, time changes, and sweep |
| Accounting invariant | Claimed ≤ allocated ≤ funded; bitmap count matches paid wallets; escrow + claims + refund conserves funded inventory |
| Backend ABI check | Every factory/escrow function and event in the Rust client matches compiled Solidity inputs and indexed event fields |
| Deployment smoke | Successful local simulation; wrong-chain and zero-deployer rejection; temporary node unchanged, no broadcast |
| Verification-tool regressions | Two tests passed; stale source/artifact checks remain enforced under optimized Python |
| Generated integration artifacts | Factory/escrow ABIs, compiler settings, uniform runtime hashes, source hashes, and code sizes exported and verified |
| Slither 0.11.5 | Zero high findings; two reviewed medium, 32 low, three informational, one optimization; explicit review and source-change gate |
| Rust unit/property suite | 16 passed, one explicit performance test ignored |
| Rust strict lint | `cargo clippy --locked --all-targets -- -D warnings` passed |
| Release API-to-chain integration | Passed using actual factory/escrow on temporary Anvil, PostgreSQL, Redis, and S3 protocol fixture |
| OpenAPI | 43 paths, 47 operations; 115 captured real responses validated |

Tests cover registry uniqueness and sender binding; config hashing and uniform code across different assets/configs; deadline ordering; minimum claim window; partial/full funding; activation restrictions; unsupported fee behavior; tokens without return values; false-return/paused tokens; single finalization; empty results; wrong or forged claims; cross-chain/cross-escrow proof rejection; bitmap word boundaries; aggregate allocation caps; cancel and permissionless sweep; full 100-NFT inventory refund; NFT recipient rejection; unsolicited/batch NFT deposits; native-payment rejection; and token/recipient/creator-collection reentrancy.

The creator-collection test intentionally makes the collection itself the creator, so a callback passes the ownership check and specifically encounters the reentrancy guard. NFT tests use standard OpenZeppelin-backed ERC-721/ERC-1155 mocks and the actual escrow. Failing payouts/refunds preserve retry rights by reverting state atomically.

The backend's complete ERC-20 scenario deploys these contracts, prepares and executes wallet transactions, reconciles confirmed events, builds real Rust Merkle leaves, checks the Solidity leaf, claims the reward, and verifies token balances. It also exercises wallet auth, admin security, creator review, raffle reproducibility, and API schema contracts. Local fixture tokens and Anvil do not validate live Arc issuers or network behavior.

A [gas report](gas-report.txt) is included for reproducibility. It mixes supported and reverting paths and warm/cold test state; its function summaries are not production fee quotes. Obtain deployment-specific `eth_estimateGas` and run representative proof/inventory sizes against the actual intended token and RPC before quoting transaction costs.

## Reproduce

```bash
cd contracts
bash scripts/bootstrap.sh
forge fmt --check
forge test --gas-report
python3 scripts/deployment-smoke.py
python3 scripts/export-artifacts.py --check
# Install the pinned analysis requirements in a virtual environment, then:
PATH="$PWD/.analysis-venv/bin:$PATH" python3 scripts/security-check.py
cd ../backend
bash scripts/test.sh
```

The CI workflow includes contract formatting/tests, source-pinned static review, ABI/artifact compatibility, and backend integration. Hosted CI has not been executed from this workspace. No live deployment, mainnet transaction, independent audit, or independent penetration test was performed. See [SECURITY.md](SECURITY.md) and [DEPLOYMENT.md](DEPLOYMENT.md).
