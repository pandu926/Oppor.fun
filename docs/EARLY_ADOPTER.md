# Oppor.fun Early Adopter

The official Genesis NFT campaign celebrates the early community supporting Oppor.fun on Arc.

[Open the campaign](https://oppor.fun/campaigns/dc379018-5da0-4fe4-a3c4-c14188ca0c1c) · [View the collection](https://explorer.arc.io/address/0x890870ce940dbbb74c2962d3b1de5f2e0858c2c9) · [View the escrow](https://explorer.arc.io/address/0x22fa96188576f0e7b5fc5653dbdcfcc9b4493333)

## Campaign rules

| Setting | Value |
| --- | --- |
| Network | Arc mainnet, chain 5042 |
| Creator | Oppor.fun's operator wallet |
| Reward | Oppor.fun Early Adopter NFT, ERC-1155 token ID 1 |
| Funded pool | 1,000 collectibles |
| Distribution | Raffle, up to 1,000 eligible recipients |
| Reward per selected recipient | Exactly one NFT |
| Registration limit | 100,000 wallets |
| Start | 2026-10-14T15:10:29.000Z |
| Submission cutoff | 2026-10-28T15:10:29.000Z |
| Review deadline | 2026-10-31T15:10:29.000Z |
| Claim deadline | 2026-11-30T15:10:29.000Z |

Times are UTC. Participants must save evidence for all four required tasks and explicitly submit their entry before cutoff. The creator reviews evidence after cutoff and determines eligibility. If fewer than 1,000 entries are eligible, the raffle selects the available eligible entries; each selected recipient still receives one NFT. Unallocated and unclaimed inventory follows the contract's refund conditions.

## Required tasks

1. Follow [@opporfun](https://x.com/opporfun).
2. Like [the official post](https://x.com/opporfun/status/2110384268778278935).
3. Leave a relevant, original comment on that post.
4. Repost that post.

Submit the evidence requested by each task. Social verification is manual; the application does not automatically check X engagement. Wallet registration alone does not establish a unique human.

## Collection

[OpporEarlyAdopter.sol](../contracts/collections/OpporEarlyAdopter.sol) mints exactly 1,000 units at deployment. It has no owner, external mint function, metadata setter, pause, or upgrade mechanism. Its metadata JSON is embedded in the onchain URI; the artwork is hosted at a versioned public URL. This is a commemorative collectible, without a promise of returns, token allocations, or future benefits.

The collection and the mainnet escrow were funded with real transactions. The [campaign manifest](../contracts/deployments/oppor-early-adopter.json) records their addresses, deadlines, transaction hashes, and artwork digest.

## Official placement

The website identifies the official campaign using its campaign ID, chain ID, creator wallet, and collection address together. The headline prioritizes that campaign during its participation, review, allocation, and claim phases. Another creator cannot obtain this official label by copying its title.

The collection passed four targeted tests for fixed supply, immutable metadata, transfer accounting, and invalid constructor arguments. The complete contract suite passed 72 tests after adding the collection. This is internal validation, not an independent audit.
