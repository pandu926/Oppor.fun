# Deployment procedure

No Arc deployment has been performed. The deployment script is ready for simulation; it contains no private key or hardcoded live address. Simulation and broadcast are separate steps.

## Prepare

1. Run the checks in [README.md](README.md), including source-pinned static analysis and the backend integration suite.
2. Confirm the intended network chain ID and RPC against the network's official documentation and `cast chain-id --rpc-url "$ARC_RPC_URL"`. The repository does not assert a mainnet chain ID or current endpoint.
3. Verify the reward token interface, base units, issuer policy, NFT support, and refund receiver. Supported labels do not establish that every token implementation behaves correctly.
4. Set `ARC_RPC_URL`, `EXPECTED_CHAIN_ID`, and `EXPECTED_DEPLOYER` to verified values. These are network/address values, not secrets. Keep signing keys in an encrypted keystore or hardware wallet; do not put a raw private key in shell history or repository files.

## Simulate

From `contracts`:

```bash
forge script script/Deploy.s.sol:Deploy \
  --rpc-url "$ARC_RPC_URL" \
  --sig 'run(uint256,address)' "$EXPECTED_CHAIN_ID" "$EXPECTED_DEPLOYER"
```

The script rejects a different chain and a zero deployer. It creates only the factory; individual escrows are created by campaign creators. Review simulation output and the intended sender before a separate broadcast using the approved wallet configuration and `--broadcast`. No live broadcast is included in automated tests or CI.

## Verify and configure backend

After deployment, verify the source with the exact compiler/optimizer/IR/EVM settings from [deployment-artifacts.json](deployment-artifacts.json). Obtain the factory address and deployment block from the confirmed receipt. Fetch the actual runtime bytecode and compare its Keccak hash to the manifest; do not substitute a creation-code hash.

```bash
FACTORY_RUNTIME=$(cast code "$FACTORY_ADDRESS" --rpc-url "$ARC_RPC_URL")
cast keccak "$FACTORY_RUNTIME"
```

Populate the backend's `ARC_CHAIN_ID`, `ARC_RPC_URL`, `FACTORY_ADDRESS`, `FACTORY_CODE_HASH`, `ESCROW_CODE_HASH`, and `FACTORY_DEPLOYMENT_BLOCK` with verified values. `FACTORY_CODE_HASH` and `ESCROW_CODE_HASH` correspond to the manifest's `runtime_code_hash` fields. Inspect `backend/.env.example` for the actual configuration names and finality settings.

The escrow stores campaign configuration without Solidity immutables, so independently created escrows share one runtime hash. Verify a sample deployed escrow's runtime and `configHash`, funding events, indexer admission, recipient claim, and deadline/refund behavior. The backend intentionally fails closed on bytecode/configuration mismatch.

Complete a small, approved end-to-end campaign on the intended environment before accepting public funds. The Anvil suite validates ERC-20 API-to-chain behavior and Solidity tests validate all three asset kinds locally; neither constitutes a real Arc deployment or independent audit.
