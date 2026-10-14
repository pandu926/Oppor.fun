# Mainnet application

The public application at [oppor.fun](https://oppor.fun) uses **Arc mainnet, chain ID 5042**. Frontend wallet configuration, API authentication, transaction preparation, worker allocation, and chain indexing use the same network and verified factory.

## Activation checks

Production verification on 14 October 2026 checked:

- Mainnet RPC chain ID, factory runtime bytecode, and the USDC ERC-20 interface.
- Readiness after the indexer caught up with confirmed blocks.
- Rejection of testnet SIWE requests and acceptance of a real mainnet wallet signature.
- Authenticated operator access and creator listings isolated from testnet data.
- Saving and locking an unpublished campaign containing all 18 task types.
- Mainnet transaction preparation and successful gas estimation for campaign creation, without broadcasting a campaign transaction.
- RainbowKit browser-wallet connection and domain-bound mainnet authentication in Chrome.
- Correct explorer links, community-task configuration, mobile containment at 360 px, and absence of uncaught JavaScript errors.

The mainnet checks passed alongside 32 browser regression tests and six frontend unit tests. Production data is separate from retained testnet records. The operational recovery procedure was also exercised using an encrypted backup and an isolated database restore.

## Wallet support

Injected browser wallets are enabled. The code also supports WalletConnect when a valid Reown project ID is configured; QR relay is not enabled in the current deployment.

## Scope

These checks establish application configuration and connectivity. They do not claim that a funded mainnet campaign or mainnet reward claim has been executed. The separate funded E2E run is documented as [testnet verification](COMPREHENSIVE_E2E_2026-10-13.md).

See [mainnet deployment](MAINNET_DEPLOYMENT.md), [verification](VERIFICATION.md), and [security](../SECURITY.md).
