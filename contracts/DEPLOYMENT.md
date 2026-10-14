# Contract deployment

The public application uses the verified factory on **Arc mainnet, chain ID 5042**. See [deployment details](../docs/MAINNET_DEPLOYMENT.md) and the [manifest](deployments/arc-mainnet.json).

## Prepare

1. Run the checks in [README.md](README.md), including tests, artifact consistency, and source-pinned static analysis.
2. Confirm the intended network, RPC, chain ID, deployer, fee estimate, and balance. Do not rely on a contract address alone to identify its network.
3. Verify the reward token, asset units, issuer behavior, and refund recipient before funding a campaign.
4. Use an encrypted keystore or hardware wallet. Keep credentials and signing keys outside Git and shell history.

The factory deploys individual campaign escrows on demand. Deploying a factory does not itself fund or activate a campaign.

## Simulate

The Foundry script guards the expected chain and deployer. Run from `contracts` with verified environment values:

```sh
forge script script/Deploy.s.sol:Deploy \
  --sig 'run(uint256,address)' "$EXPECTED_CHAIN_ID" "$EXPECTED_DEPLOYER" \
  --rpc-url "$ARC_RPC_URL"
```

Simulation does not publish the deployment. Inspect its output before broadcasting.

## Broadcast

Select an encrypted Foundry keystore using its local account alias:

```sh
forge script script/Deploy.s.sol:Deploy \
  --sig 'run(uint256,address)' "$EXPECTED_CHAIN_ID" "$EXPECTED_DEPLOYER" \
  --rpc-url "$ARC_RPC_URL" \
  --account "$DEPLOYER_ACCOUNT" \
  --broadcast
```

Do not repeat a broadcast blindly if a receipt is delayed. Inspect the transaction hash and sender nonce before deciding whether another transaction is required.

## Verify and integrate

Check the successful creation receipt, chain ID, sender, deployment address, and creation input. Compare runtime bytecode with the release hashes in `deployment-artifacts.json`, then record the creation transaction and block in a public manifest.

Configure the backend's chain ID, RPC, factory, deployment block, runtime hashes, and confirmation depth. Configure the frontend for the same chain and explorer. Start indexing and require readiness before exposing funds-related operations.

Bytecode verification and explorer source verification are different checks. The deployed release has verified bytecode; its explorer source-verification status is recorded in the deployment guide. Independent audit remains outstanding.
