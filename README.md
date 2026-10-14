# Oppor

Community reward campaigns on Arc, with creator-funded escrow and wallet-based claims.

[Live application](https://oppor.fun) · [Product documentation](https://oppor.fun/docs) · [Mainnet deployment](docs/MAINNET_DEPLOYMENT.md)

Oppor gives token creators and communities a single workflow for configuring tasks, funding rewards, reviewing participation evidence, and distributing allocations. Creators choose the reward asset and distribution method; eligible participants claim their rewards directly from a dedicated campaign escrow.

## Features

- **Configurable tasks.** Choose from 18 task types across X, Discord, Telegram, YouTube, TikTok, Instagram, websites, and custom actions. Combine up to 20 required or optional tasks.
- **Flexible rewards.** Fund campaigns with USDC, other ERC-20 tokens, ERC-721 NFTs, or ERC-1155 assets.
- **Distribution options.** Use all-eligible or raffle distribution, with equal-pool and supported fixed-reward policies.
- **Evidence and review.** Participants submit text, links, or screenshots. Creators review entries after cutoff and lock the eligibility set.
- **Onchain settlement.** Lock campaign rules, fund an escrow, finalize an allocation, and let recipients claim using Merkle proofs.
- **Defined deadlines.** Campaign contracts enforce funding, review, claim, cancellation, and refund conditions.

## How it works

1. The creator configures rewards, tasks, limits, and deadlines.
2. The creator locks the rules, deploys a campaign escrow, funds it, and activates the campaign.
3. Participants connect a wallet, complete tasks, and submit evidence before cutoff.
4. The creator reviews entries and locks eligibility.
5. The backend publishes allocation artifacts; the creator finalizes the distribution onchain.
6. Recipients claim their allocations from escrow using their wallets.

Social verification is manual. The creator controls eligibility, and raffle selection uses a reproducible server draw. These trust boundaries are described in the [security documentation](SECURITY.md).

## Architecture

| Component | Stack | Source |
| --- | --- | --- |
| Frontend | React, Vite, TypeScript, RainbowKit, wagmi, viem | [frontend](frontend/) |
| API and worker | Rust, Axum, SQLx, PostgreSQL, Redis | [backend](backend/) |
| Smart contracts | Solidity, Foundry, OpenZeppelin | [contracts](contracts/) |
| API contract | OpenAPI 3.1 with generated frontend types | [OpenAPI](backend/openapi.json) |

The backend prepares unsigned transactions and indexes confirmed events. Creators and participants authorize transactions with their own wallets. See [architecture](docs/ARCHITECTURE.md) for component responsibilities.

## Development

Start with the component guides for environment variables, dependencies, and local services:

- [Frontend setup](frontend/README.md)
- [Backend setup](backend/README.md)
- [Contract setup](contracts/README.md)
- [Development and verification commands](docs/DEVELOPMENT.md)

Production deployments require their own secrets, infrastructure, storage policies, and verified network configuration. Environment examples contain placeholders; signing keys and credentials must remain outside the repository.

## Deployment

The application runs on **Arc mainnet, chain ID 5042**.

```text
Factory: 0x3fe542ab4ca297eb8936692b04a619a9f5935561
```

[View the factory](https://explorer.arc.io/address/0x3fe542ab4ca297eb8936692b04a619a9f5935561) · [Deployment manifest](contracts/deployments/arc-mainnet.json)

The factory creates a separate escrow for each campaign. It has no owner or upgrade mechanism. Read [deployment details](docs/MAINNET_DEPLOYMENT.md) and [verification scope](docs/VERIFICATION.md) before integrating.

## Security and verification

The repository includes contract tests, backend integration tests, frontend browser tests, and source-pinned static-analysis checks. A complete funded campaign-to-claim verification run was performed on Arc testnet; mainnet deployment and application checks are recorded separately in [verification](docs/VERIFICATION.md).

An independent security audit has not been completed. Token issuer restrictions, creator decisions, RPC availability, and the raffle trust model remain relevant. See [security boundaries](SECURITY.md) and the [internal contract review](contracts/AUDIT.md).
