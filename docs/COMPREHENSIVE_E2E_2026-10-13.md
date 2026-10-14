# Campaign lifecycle verification

A complete campaign-to-claim run was performed on **Arc testnet, chain ID 5042002**, on 13 October 2026. The run used the live application, actual object storage, newly created test wallets, and test-only reward assets. Chain timestamps were not modified.

## Results

- **68 confirmed blockchain transactions.**
- **15 successful reward claims.**
- **18 API-submitted entries** and one browser-submitted entry.
- **All 18 community task types** exercised through evidence submission and creator review.
- **49 API checks**, **14 browser checks**, and **10 independent reconciliation checks** passed.

## Reward scenarios

| Scenario | Submitted entries | Successful claims |
| --- | ---: | ---: |
| Custom ERC-20, all 18 tasks | 3 | 2 |
| Custom ERC-20, fixed rewards | 3 | 3 |
| USDC, all eligible | 3 | 2 |
| USDC, raffle | 3 | 1 |
| ERC-721 NFTs | 3 | 3 |
| ERC-1155, fixed rewards | 3 | 3 |
| Browser-operated USDC campaign | 1 | 1 |

One participant was deliberately disqualified in the equal-pool ERC-20 cases. The fixed-reward case paid the configured amount to every accepted participant. The raffle paid its selected winner. All six API campaign escrows were independently checked to have zero remaining rewards after claims.

## Browser workflow

Chrome exercised real wallet authentication, task selection, campaign creation, escrow deployment, approvals, funding, activation, entry registration, screenshot upload, evidence saving, explicit submission, creator review, eligibility locking, finalization, and claim confirmation after reload. Desktop and 360 px mobile layouts were inspected.

A funded draft was cancelled and its reward pool returned to the configured refund recipient. An earlier expired campaign was refunded through the browser. Test campaigns were removed from public discovery while preserving their records and participant claim access.

## Negative cases

- Wrong-chain authentication and unauthorized administration were rejected.
- Missing CSRF, unauthorized configuration locking, and locked-rule edits were rejected.
- Submission without all required evidence and another participant's private evidence access were rejected.
- Upload completion by another user, early review, late evidence edits, incomplete eligibility locking, and stale review versions were rejected.
- Wrong-recipient claims, forged quantities, invalid token IDs, and repeated claims were rejected by the relevant API or live-contract checks.

## Independent reconciliation

Published manifest and snapshot hashes matched their onchain commitments. Claim leaves and Merkle proofs were independently reconstructed. The raffle was replayed from its published seed and eligibility snapshot; its commitment, counter, selected wallet, and claimed allocation matched. Participant token balances, NFT ownership, ERC-1155 quantities, native-USDC funding, and transaction gas reconciled.

Social evidence was test data reviewed by the creator. This run validates the workflow and settlement; it does not assert real external social engagement. The recorded 15 claims occurred on testnet, not mainnet. See [mainnet verification scope](MAINNET_ACTIVATION.md).
