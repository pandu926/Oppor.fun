# Contract security boundaries

## Authority and trust

The factory is permissionless and has no owner. Each campaign's creator is established by the factory caller. Creator-only actions are funding, activation, one-time finalization, and draft cancellation. Sweep is permissionless but pays only the configured refund recipient. Claim must be called by the proof-bound recipient. There is no platform key, generic external-call facility, admin payout, root replacement, proxy, or upgrade authority.

Creator attestation remains a trust boundary: a ordinary Merkle root proves membership, not correct eligibility, raffle fairness, the sum of quantities, or uniqueness of NFT assignments. Aggregate payout and inventory checks prevent overpayment, but an invalid or malicious manifest can leave some recipients unable to claim. The backend validates the manifest before preparation; users still depend on the creator signing the correct root. Seed commitments and reproducible raffle transcripts do not make creator-controlled finalization trustless.

Reward tokens and NFT collections are external dependencies. Mutable issuers can pause transfers, blacklist a wallet, confiscate balances, or upgrade token logic. Balance/ownership checks reject inconsistent transfers, not a token that dishonestly reports its own state. Rebasing, fee-on-transfer, reflection, and sender/recipient-tax behavior are unsupported. Existing token policy can make claim/refund fail even after valid funding.

## Implemented controls

- Fixed configuration, domain-bound double-hashed leaves, exact asset/token-ID validation, and immutable finalization.
- Bitmap protection per claim index; ERC-721 deposited/delivered checks and payout limits.
- Checks/effects before payout, plus OpenZeppelin storage-based `ReentrancyGuard` on every lifecycle mutation and factory creation. This does not require EIP-1153.
- Safe ERC-20 wrappers, exact source/destination balance deltas for funding and payout, and NFT ownership checks.
- NFT receivers accept only an in-flight creator funding transfer with the exact collection, operator, sender, ID, and quantity. Batch deposits are rejected. Receiver callbacks do not attempt to reacquire the funding guard.
- Failed transfers revert claim bits, counters, terminal states, and NFT flags atomically.
- No early sweep, including after all declared claims complete. Deadline boundaries are explicit and tested. The backend's confirmed-chain policy remains necessary for displaying transactions.
- At most 100 NFTs in funding/refund loops, at most 100,000 leaves, and at most 32 proof hashes. No participant loop at finalization.

The libraries are pinned to OpenZeppelin Contracts 5.4.0. See [SafeERC20 documentation](https://docs.openzeppelin.com/contracts/5.x/api/token/erc20), [ReentrancyGuard documentation](https://docs.openzeppelin.com/contracts/5.x/api/utils), and [MerkleProof documentation](https://docs.openzeppelin.com/contracts/5.x/api/utils/cryptography). Library reuse does not audit the custom contracts.

## Static-analysis review

Slither 0.11.5 inspected the production sources with dependency/test/script findings excluded from the report. It produced zero high findings, two medium findings, 32 low findings, three informational findings, and one optimization suggestion. These are recorded honestly; the scan does not claim zero findings.

The two medium `reentrancy-no-eth` reports concern `fundERC721` and `_refund`. Slither observes writes in a subsequent loop iteration after an earlier token call. Both operations execute under the single escrow reentrancy guard. Each NFT's inventory/delivery state is set before its own transfer; refund enters a terminal state before any external call. The sole unguarded receivers can only consume the exact outstanding deposit context. The creator-collection test attempts a nested `activate` from an address that passes `onlyCreator` and verifies the guard's precise revert selector. Recipient callback tests cover nested claims; rejecting recipients verify atomic rollback.

These findings have explicit source-pinned explanations in [security-review.json](security-review.json). `scripts/security-check.py` rejects new medium/high findings, changes to the reviewed production-source hashes, or a changed medium-finding baseline. It does not globally disable reentrancy detectors.

The remaining reports are bounded external-call loops (25 low), intended timestamp deadline checks (six low), the same guarded funding loop (one low), three bounded storage-write-loop notices, and a suggestion to make `configHash` immutable. Storage configuration is intentional: a per-campaign embedded immutable would break the backend's uniform runtime hash. NFT calls can still revert or consume excessive gas; the 100-item cap bounds iteration count, not arbitrary collection code execution. The local maximum-inventory test succeeds with standard NFT behavior. Token/receiver liveness remains an external trust boundary.

## Recovery and operational limits

The [internal audit](AUDIT.md) reproduces the case where one frozen ERC-721 ID prevents refund of otherwise transferable inventory. Refund uses safe NFT transfer. A refund wallet must support the relevant receiver interface and continue accepting transfers. A blocked token or rejecting refund wallet can prevent cancellation/sweep; the configured address cannot be replaced. Test that recipient and token before funding. No privileged bypass is provided.

Unrecorded assets can be trapped; only approved funding methods belong in the frontend. Failed creator finalization permits refund after review expiry, and unclaimed official inventory is refunded after claim expiry. There is no contract fee or early extraction of unused allocated rewards.

These contracts have not been independently audited. Tests, fuzzing, invariants, code-size checks, static analysis, and backend compatibility checks support review; they do not prove universal safety. Live Arc token behavior, deployed bytecode, finality, gas capacity, source verification, and operational monitoring must be validated for the intended environment.
