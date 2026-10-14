# Arc mainnet deployment

Oppor's application and contracts use **Arc mainnet, chain ID 5042**.

| Field | Value |
| --- | --- |
| Application | https://oppor.fun |
| Product documentation | https://oppor.fun/docs |
| Network | Arc mainnet |
| Chain ID | 5042 |
| RPC | `https://rpc.mainnet.arc.io` |
| Explorer | https://explorer.arc.io |
| Factory | `0x3fe542ab4ca297eb8936692b04a619a9f5935561` |
| Deployment block | 25932535 |
| Deployment date | 14 October 2026 |

[Factory contract](https://explorer.arc.io/address/0x3fe542ab4ca297eb8936692b04a619a9f5935561) · [Creation transaction](https://explorer.arc.io/tx/0xd1baf1ac2e16c6da0306fbacc97171548497f8327268a4544ebed30bf12c0ecb)

The [deployment manifest](../contracts/deployments/arc-mainnet.json) records the creation transaction and bytecode hashes. Runtime bytecode was checked against the [release artifacts](../contracts/deployment-artifacts.json). This is bytecode verification; explorer source-code verification has not been submitted.

## Contract model

The permissionless factory creates a dedicated escrow for each campaign. It has no owner, upgrade mechanism, or platform withdrawal capability. Campaign creators fund their escrows and authorize finalization; recipients claim allocations with their own wallets.

Individual escrows enforce immutable campaign configuration, deadlines, reward accounting, and recipient-bound Merkle proofs. Onchain settlement does not establish whether a social action occurred or whether a creator's eligibility decisions were fair. See [security](../SECURITY.md).

## Verification scope

All 68 contract tests passed before mainnet deployment. Mainnet activation checks verified wallet authentication, application configuration, factory bytecode, and a simulated campaign-creation call. The funded campaign-to-claim E2E run was performed on testnet. See [verification](VERIFICATION.md).

For deployment to another environment, follow [the contract deployment guide](../contracts/DEPLOYMENT.md), configure the matching frontend and backend network, and provide your own infrastructure and secrets.
