# Verification

Oppor's verification combines contract tests, backend integration, browser regression coverage, and live network checks. Results below describe executed checks, not guarantees of security or future availability.

## Recorded results

| Scope | Result |
| --- | --- |
| Solidity contracts | 68 tests passed, including regression cases and stateful invariants |
| Frontend | 32 Playwright browser tests and six unit tests passed |
| Mainnet application | 11 live checks passed for chain configuration, wallet authentication, campaign configuration, and factory-call simulation |
| Public pages | Six HTML, metadata, and hydration checks passed |
| Complete testnet lifecycle | 68 confirmed transactions and 15 successful reward claims |
| Independent testnet reconciliation | Manifest hashes, Merkle proofs, raffle replay, final balances, and escrow balances matched |
| Recovery procedure | Encrypted database backup restored successfully into an isolated environment |

The [testnet lifecycle summary](COMPREHENSIVE_E2E_2026-10-13.md) describes the funded scenarios. [Mainnet application checks](MAINNET_ACTIVATION.md) describe the production activation scope. Mainnet checks did not broadcast or fund a campaign transaction.

## Reproduce checks

- [Development commands](DEVELOPMENT.md)
- [Contract verification](../contracts/VERIFICATION.md)
- [Backend verification](../backend/VERIFICATION.md)
- [GitHub Actions](../.github/workflows/)

Contract tests use the pinned Solidity compiler and OpenZeppelin dependency revision. Backend integration runs against isolated PostgreSQL, Redis, object-storage fixtures, Anvil, and the real contract implementation. Browser regression tests use explicit fixtures; they do not move mainnet funds.

The contract source-pinned static-analysis baseline, ABI exports, generated API schemas, and frontend types have corresponding consistency checks in CI. A stale source review or interface export must be addressed rather than silently accepted.

## Limits

No independent security audit or independent penetration test has been completed. Creators control eligibility, the raffle depends on a server seed, and supported token standards do not guarantee compatible behavior for every token. See [security](../SECURITY.md) and the [internal contract review](../contracts/AUDIT.md).
