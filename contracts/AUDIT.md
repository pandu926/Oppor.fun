# Internal security audit

Date: 2026-10-09. Scope: `CampaignFactory`, `CampaignEscrow`, shared configuration/claim types, dependency/compiler choices, ABI/artifact tooling, and the Rust backend's configuration hashing, transaction preparation, event admission, allocation finalization, and payout reconciliation. This is an internal review by the implementing agent, not an independent audit or certification. It is not a comprehensive penetration test of the entire backend/infrastructure.

## Findings

| ID | Severity | Finding | Status |
| --- | --- | --- | --- |
| OPPOR-01 | Low | Divisible-asset finalization accepted mathematically impossible leaf counts | Fixed and regression-tested |
| OPPOR-02 | Medium, conditional availability | One frozen NFT blocks the entire atomic inventory refund | Reproduced; unresolved design limitation |
| OPPOR-03 | Informational, trust model | Creator can commit an unfair or internally inconsistent Merkle root | Documented trust boundary; not eliminated |
| OPPOR-04 | Informational, maintenance | Compiler pin predates subsequently published compiler fixes | Trigger review completed; no applicable trigger identified in reviewed production sources |
| OPPOR-05 | Low, verification tooling | Optimized Python could disable source/artifact security checks | Fixed; two optimized-mode regressions passed |

No outsider fund theft, root replacement, cross-campaign claim replay, or successful lifecycle reentrancy exploit was identified in the reviewed scope. This statement is limited to the examined code and tests; it is not proof that every vulnerability has been found.

## OPPOR-01: impossible divisible allocation count

Location: [finalize](src/CampaignEscrow.sol#L195).

Every valid claim requires a positive integer quantity. Therefore the declared allocated quantity must be at least the declared number of leaves, regardless of asset kind. Before this review, ERC-20/ERC-1155 finalization could accept `allocatedQuantity=2` and `leafCount=3`. Such a declaration cannot pay three positive-quantity claims within the declared payout cap. Finalization is irreversible, so a mistaken creator declaration could leave promised claims unpayable until expiry/refund.

The creator is the only party able to finalize; outsiders cannot use this path to alter another campaign or steal funds. The Rust allocation builder emits positive quantities, so the normal prepared distribution is already consistent. Severity is Low because this is additional onchain input validation of a creator-attested commitment, not an outsider escalation or an assertion that an unfair root is otherwise impossible.

Evidence: `testRejectImpossibleDivisibleAllocationCount` failed before the patch with “next call did not revert as expected.” The patch rejects `declaredLeafCount > declaredAllocatedQuantity`. Regression tests cover ERC-20 and ERC-1155. Existing bitmap fuzz tests now use feasible declared budgets and exercise indices throughout 0–99,999. No ABI, authority, deadline, or valid allocation behavior changed.

## OPPOR-02: atomic NFT refund couples transfer availability

Location: [refund loop](src/CampaignEscrow.sol#L374).

Refund transfers all undelivered deposited NFTs in one transaction. If the collection freezes just one deposited token, or the refund receiver rejects it, that transfer reverts the entire operation, including transfers of otherwise transferable NFTs. Neither the creator nor an unrelated sweep caller can recover the unaffected inventory using a subset transfer. A permanently frozen token can thus leave the remaining inventory in escrow indefinitely.

Evidence: `testOneFrozenNFTBlocksAtomicRefundUntilIssuerRestoresTransfers` funds IDs 7 and 8 with an otherwise standard ERC-721 collection, activates the campaign, freezes only ID 7, and attempts an expired-campaign sweep. The test verifies revert, unchanged active state, both tokens still held by escrow, and rolled-back delivery flags. Once the issuer restores ID 7 transfers, the same sweep succeeds.

This is a conditional availability risk under the already documented external-token trust model. It does not redirect funds or allow early withdrawal. It remains unresolved: atomic refund is the specified baseline, and an escrow cannot override a collection's freeze policy. A future per-item refund design could isolate failures for unaffected NFTs, but would need explicit per-item accounting, repeatable refund events, compatible backend projection, and a revised terminal-state policy. That change would still not recover a frozen token universally.

Before live use, constrain supported collections and validate refund receivers. This reduces likelihood; it cannot guarantee future issuer/receiver behavior. Existing NFT claim rights remain governed by the deadline and collection transfer policy.

## OPPOR-03: creator-attested results are not trustless

A normal Merkle root does not prove that all eligible entrants appear, that leaf quantities sum to the declared amount, or that every ERC-721 assignment is unique. The contract bounds cumulative payout, rejects repeated indices and delivered NFTs, binds recipient/token/chain/escrow/campaign, and freezes the root. These controls stop overspending and replay; they do not establish correct creator decisions. A hostile creator can still produce a root with missing or unpayable claims, or finalize an empty result despite eligible participants.

The backend validates generated allocations and compares finalization events to the published artifacts, marking inconsistent campaigns as integrity errors. It cannot undo an already finalized onchain commitment. Eligibility review and reproducible server raffle continue to require creator/backend trust. No trustless-fairness claim should appear in product copy.

## OPPOR-04: compiler and dependency review

Production uses Solidity 0.8.30, optimized IR, Paris target, and a storage-based guard. The official [known-bugs registry](https://docs.soliditylang.org/en/latest/bugs.html) now lists later fixes affecting that compiler family. Applicability review is an inference from trigger conditions and the actual sources: no mutual internal recursion, custom storage placement near the storage-space end, transient storage, memory-byte-element deletion in the legacy pipeline, or named-argument custom errors inside `require` were identified. Paris also excludes the Cancun-only transient-storage condition. Pinning 0.8.30 is not a claim that the compiler has no known bugs; recheck the registry whenever source or settings change.

OpenZeppelin's published [advisory index](https://github.com/OpenZeppelin/openzeppelin-contracts/security/advisories) was reviewed. The [Bytes.lastIndexOf advisory](https://github.com/OpenZeppelin/openzeppelin-contracts/security/advisories/GHSA-9rcw-c2f9-2j55) concerns a utility outside the production import closure; the contracts do not use it. The older multiproof advisory concerns an older version/feature; this implementation uses single proofs in pinned 5.4.0. Dependency versions and source commits remain pinned. This is source applicability review, not an independent audit of OpenZeppelin or the compiler.

## OPPOR-05: optimization bypassed verification assertions

The artifact exporter and static-analysis gate used Python `assert` for source hashes, stale ABIs, compiler settings, code-size checks, and reviewed finding validation. Running Python with `-O` or `PYTHONOPTIMIZE` disables those statements. Under that explicit environment condition, the tooling could report success despite a failed verification condition. The ordinary CI commands did not enable optimized Python, and this is not an onchain exploit; severity is Low.

These deployment/security gates now use explicit checks that raise exceptions independently of optimization mode. Two isolated regression tests deliberately provide a stale source-review hash and a stale exported ABI, respectively, and run the gates under `python -O`. Both must fail with the intended validation message. The tests use temporary copies and do not tamper with actual source/dependency/artifact files. CI runs these regressions alongside the normal positive gate checks.

## Method and evidence

- Manual review of the authority matrix, full state lifecycle, immutable configuration, uint256 bounds, bitmap word/index isolation, leaf domain, finalization constraints, token balance deltas, receiver deposit context, and refund caller/recipient binding.
- Review of backend factory/escrow admission, config hashing, creator transaction preparations, NFT inventory checks, manifest matching, onchain claim reconciliation, and permissionless refund events. No broad backend security certification is implied.
- Adversarial Foundry regressions, existing fuzz/invariant suites, static analysis, compilation/code-size validation, artifact/ABI consistency, deployment simulation, and the actual-contract backend integration scenario.
- Slither's two medium loop/callback reports were re-examined after the patch. The guard spans entire funding/refund operations, each NFT is marked before its own transfer, and receiver context cannot grant general mutation authority. These reviewed detector reports are separate from the conditional refund availability finding above.

Before-patch escrow source SHA-256: `e80cfbf0bb3e92fc6e59b7252e40911bbd0fd442be0e80787a5c821b05c91824`. The patched source SHA-256 and new runtime hashes are recorded in [deployment-artifacts.json](deployment-artifacts.json); [security-review.json](security-review.json) pins the updated reviewed source. Both factory and escrow code hashes change because factory bytecode embeds escrow creation code. Backend configuration must use the regenerated hashes for a deployment of the patched version. There is no live deployment to upgrade, and existing immutable deployments could not be patched in place.

Post-fix validation: all 68 Solidity tests passed, including three audit regressions and the two stateful invariants. The optimized-mode tooling checks and deployment simulation passed; the actual-contract Rust integration passed in both debug and release profiles, with 115 captured responses validated against OpenAPI. See [VERIFICATION.md](VERIFICATION.md) for executed checks. Mainnet/Arc deployment, real token behavior, external RPC trust, issuer upgrades, adversarial gas consumption, comprehensive backend penetration testing, and independent human review remain outside this audit's evidence.
