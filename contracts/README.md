# Oppor campaign contracts

Implemented, non-upgradeable Solidity contracts for creator-funded promotion campaigns. The backend prepares unsigned transactions; creators and recipients sign with their own wallets. No platform wallet holds rewards or has withdrawal privileges.

- [CampaignFactory](src/CampaignFactory.sol): permissionless deployment and `(creator, campaignKey)` registry.
- [CampaignEscrow](src/CampaignEscrow.sol): one reward asset, fixed deadlines, funding, activation, single finalization, Merkle claims, cancellation, and expired-campaign refund.
- [Types](src/CampaignTypes.sol): field order and numeric enum values shared with the Rust backend.
- [Security boundaries and static-analysis review](SECURITY.md).
- [Verification record](VERIFICATION.md).
- [Internal security audit and remaining risks](AUDIT.md).
- [Deployment procedure](DEPLOYMENT.md).
- [Exported ABIs](abi/) and [reproducible bytecode manifest](deployment-artifacts.json).

## Build and verify

Use Foundry v1.5.1, Git, and Python 3. Solidity 0.8.30 is downloaded by Foundry. Dependencies are pinned by full commit in `scripts/bootstrap.sh`; bootstrap refuses modified tracked dependency files or a different revision.

```bash
cd contracts
bash scripts/bootstrap.sh
forge fmt --check
forge build --sizes
forge test
python3 scripts/deployment-smoke.py
python3 scripts/export-artifacts.py --check
python3 scripts/test-security-gates.py
```

To reproduce static analysis in an isolated Python environment:

```bash
python3 -m venv .analysis-venv
.analysis-venv/bin/pip install -r scripts/requirements-static-analysis.txt
PATH="$PWD/.analysis-venv/bin:$PATH" python3 scripts/security-check.py
```

The analyzer reports reviewed findings rather than hiding whole detector classes. The check fails on new medium/high findings and any production-source change without a refreshed review. Generated ABIs and bytecode hashes must also match the compiled sources and every function/event used by `backend/src/chain.rs`.

Run the complete API-to-chain scenario from `backend` with `bash scripts/test.sh`. It deploys the actual factory/escrow implementation on temporary Anvil, alongside test-only token and wallet mocks. Nothing in these commands deploys to Arc or spends live funds.

## Lifecycle

1. Creator calls `createCampaign(config)`. Factory requires the caller to be `config.creator` and rejects a reused key for that creator.
2. Creator approves the reward token/collection and funds the escrow before `startsAt`. Partial funding is allowed. Activation requires the exact target quantity.
3. The active campaign accepts participants through the backend. Task evidence and review remain offchain and manual.
4. Between `cutoffAt` inclusive and `reviewDeadline` exclusive, creator finalizes a Merkle root, allocation-manifest hash, eligibility hash, allocated quantity, and leaf count. Empty results are supported. These values cannot be replaced.
5. Each recipient calls `claim(allocation, proof)` before `claimDeadline`. The payout address is bound to the proof and must be the caller. ERC-721 uses one leaf and one claim per NFT.
6. A draft can be cancelled by its creator. Anyone can execute an expired-campaign sweep; the immutable configured refund recipient receives the remaining officially funded inventory.

State values are `DRAFT=0`, `ACTIVE=1`, `FINALIZED=2`, `CANCELLED=3`, `CLOSED=4`. Asset kinds are `ERC20=0`, `ERC721=1`, `ERC1155=2`. Distribution modes are `ALL_ELIGIBLE=0`, `RAFFLE=1`. The contract records the mode; eligibility, the draw, and allocation policy are attested by the creator using backend-generated artifacts.

## Configuration and limits

Configuration is set once in the constructor and has no setter. It is stored instead of embedded as Solidity immutables so every campaign has identical runtime bytecode for the backend's exact code-hash check. `configHash = keccak256(abi.encode(config))` binds every field.

Required order: `deployment time < startsAt < cutoffAt < reviewDeadline < claimDeadline`. The claim window after review must be at least 24 hours. Maximum leaf count/participant capacity/winner count is 100,000; ERC-721 inventory is capped at 100. Proofs are bounded to 32 hashes. Funding and refund loops exist only for bounded NFT inventory; finalize and individual claim do not iterate over participants.

ERC-20 and ERC-1155 quantities use integer base units. ERC-721 target quantity counts unique deposited IDs, each claim quantity is one, and ERC-1155 campaigns have one configured token ID. Arc USDC rewards use its ERC-20 interface and six-decimal base units; native gas balances are not a second reward asset or escrow funding path. See the original [asset specification](../docs/SMART_CONTRACT.md).

Only creator-initiated funding calls count toward funded inventory. Unsolicited safe NFT transfers and ERC-1155 batch deposits are rejected. Direct ERC-20 transfers and unsafe ERC-721 transfers do not increase funding and may be stranded. Ordinary native transfers are rejected. There is no arbitrary rescue, root override, platform pause, proxy upgrade, or platform fee in this implementation.

An escrow verifies transfers and blocks duplicate claims, but does not prove fairness or guarantee token issuer behavior. The implementation and local tests are complete; an independent audit and live deployment verification have not been performed.
