import {
  decodeFunctionData,
  encodeAbiParameters,
  keccak256,
  erc20Abi,
  erc721Abi,
  erc1155Abi,
  createPublicClient,
  createWalletClient,
  custom,
  http,
  isAddress,
  type EIP1193Provider,
  type Hex,
  type Address,
} from "viem";
import { factoryAbi, escrowAbi } from "./abi";
import { chain, factoryAddress } from "./config";
import type { PreparedTransaction, Campaign } from "./types";
export type Provider = EIP1193Provider & {
  on?: (event: string, callback: (...args: unknown[]) => void) => void;
  removeListener?: (
    event: string,
    callback: (...args: unknown[]) => void,
  ) => void;
};
declare global {
  interface Window {
    ethereum?: Provider;
  }
}
export const publicClient = createPublicClient({
  chain,
  transport: http(chain.rpcUrls.default.http[0], { timeout: 15000 }),
});
export function walletClient(provider: Provider) {
  return createWalletClient({ chain, transport: custom(provider) });
}
export async function switchChain(provider: Provider) {
  const client = walletClient(provider);
  try {
    await client.switchChain({ id: chain.id });
  } catch (error) {
    if (
      (error as { code?: number }).code !== 4902 &&
      !String(error).includes("4902")
    )
      throw error;
    await client.addChain({ chain });
    await client.switchChain({ id: chain.id });
  }
}
export function validatePrepared(
  tx: PreparedTransaction,
  wallet: string,
  campaign: Campaign,
  kind: string,
) {
  if (
    kind === "APPROVE"
      ? ![
          "APPROVE",
          "RESET_APPROVAL",
          "APPROVE_NFT",
          "APPROVE_COLLECTION",
        ].includes(tx.intent)
      : tx.intent !== kind
  )
    throw new Error("Transaction intent does not match the requested action.");
  if (
    kind === "CREATE" &&
    (!campaign.config_hash || tx.config_hash !== campaign.config_hash)
  )
    throw new Error(
      "Configuration commitment does not match the locked campaign.",
    );
  if (
    tx.chain_id !== String(chain.id) ||
    campaign.chain_id !== String(chain.id)
  )
    throw new Error(
      "Transaction network does not match the configured network.",
    );
  if (tx.expected_sender.toLowerCase() !== wallet.toLowerCase())
    throw new Error("Transaction sender does not match your wallet.");
  if (
    !isAddress(tx.to) ||
    !/^0x(?:[a-fA-F0-9]{2})*$/.test(tx.data) ||
    !/^\d+$/.test(tx.value) ||
    BigInt(tx.value) !== 0n
  )
    throw new Error("Invalid transaction payload.");
  if (
    !Number.isFinite(Date.parse(tx.expires_at)) ||
    Date.parse(tx.expires_at) <= Date.now()
  )
    throw new Error("This transaction has expired. Prepare it again.");
  const target =
    kind === "CREATE"
      ? factoryAddress
      : kind === "APPROVE"
        ? campaign.reward.token_address
        : campaign.escrow_address;
  if (
    !target ||
    !isAddress(target) ||
    target.toLowerCase() !== tx.to.toLowerCase()
  )
    throw new Error(
      "Transaction target does not match the configured contract.",
    );
  const data = tx.data as Hex;
  if (kind === "APPROVE") {
    const abi =
      campaign.reward.asset_kind === "ERC20"
        ? erc20Abi
        : campaign.reward.asset_kind === "ERC721"
          ? erc721Abi
          : erc1155Abi;
    const decoded = decodeFunctionData({ abi, data });
    if (
      !["approve", "setApprovalForAll"].includes(decoded.functionName) ||
      !decoded.args
    )
      throw new Error("Unexpected approval function.");
    const [spender, quantity] = decoded.args;
    if (
      String(spender).toLowerCase() !== campaign.escrow_address?.toLowerCase()
    )
      throw new Error("Approval spender does not match this escrow.");
    if (
      campaign.reward.asset_kind === "ERC20" &&
      (typeof quantity !== "bigint" ||
        quantity > BigInt(campaign.reward.amount_base_units))
    )
      throw new Error("Approval exceeds the campaign reward pool.");
    if (
      campaign.reward.asset_kind === "ERC721" &&
      !campaign.reward.nft_inventory?.includes(String(quantity))
    )
      throw new Error("NFT approval is outside the configured inventory.");
    if (campaign.reward.asset_kind === "ERC1155" && quantity !== true)
      throw new Error("Unexpected collection approval.");
  } else if (kind === "CREATE") {
    const decoded = decodeFunctionData({ abi: factoryAbi, data });
    if (
      keccak256(encodeAbiParameters(factoryAbi[0].inputs, decoded.args)) !==
      campaign.config_hash
    )
      throw new Error(
        "Creation calldata differs from the locked configuration.",
      );
  } else {
    const decoded = decodeFunctionData({ abi: escrowAbi, data });
    const expected: Record<string, string> = {
      FUND:
        campaign.reward.asset_kind === "ERC20"
          ? "fundERC20"
          : campaign.reward.asset_kind === "ERC721"
            ? "fundERC721"
            : "fundERC1155",
      ACTIVATE: "activate",
      FINALIZE: "finalize",
      CLAIM: "claim",
      CANCEL: "cancel",
      SWEEP: "sweepRemaining",
    };
    if (decoded.functionName !== expected[kind])
      throw new Error("Unexpected escrow function.");
    if (
      decoded.functionName === "claim" &&
      (decoded.args[0].recipient.toLowerCase() !== wallet.toLowerCase() ||
        decoded.args[0].quantity <= 0n)
    )
      throw new Error("Claim recipient or quantity is invalid.");
    if (
      decoded.functionName === "fundERC20" ||
      decoded.functionName === "fundERC1155"
    )
      if (
        decoded.args[0] <= 0n ||
        decoded.args[0] > BigInt(campaign.reward.amount_base_units)
      )
        throw new Error("Funding amount is outside the campaign reward pool.");
    if (
      decoded.functionName === "fundERC721" &&
      decoded.args[0].some(
        (id) => !campaign.reward.nft_inventory?.includes(String(id)),
      )
    )
      throw new Error("Funding includes an unconfigured NFT.");
    if (
      decoded.functionName === "finalize" &&
      (decoded.args[0] !== tx.distribution_root ||
        decoded.args[1] !== tx.manifest_hash)
    )
      throw new Error("Finalization commitments do not match the preview.");
  }
}
export async function executePrepared(
  provider: Provider,
  tx: PreparedTransaction,
  wallet: string,
  campaign: Campaign,
  kind: string,
  onHash?: (hash: Hex) => void,
) {
  validatePrepared(tx, wallet, campaign, kind);
  const client = walletClient(provider);
  const [accounts, id] = await Promise.all([
    client.getAddresses(),
    client.getChainId(),
  ]);
  if (id !== chain.id || accounts[0]?.toLowerCase() !== wallet.toLowerCase())
    throw new Error("Wallet or network changed. Reconnect before continuing.");
  const hash = await client.sendTransaction({
    account: wallet as Address,
    to: tx.to as Address,
    data: tx.data as Hex,
    value: BigInt(tx.value),
    chain,
  });
  onHash?.(hash);
  const receipt = await publicClient.waitForTransactionReceipt({
    hash,
    confirmations: 1,
    timeout: 120000,
  });
  if (receipt.status !== "success")
    throw new Error(`Transaction reverted: ${hash}`);
  return hash;
}
export function uint256(value: string) {
  if (!/^(0|[1-9]\d*)$/.test(value) || BigInt(value) > 2n ** 256n - 1n)
    throw new Error("Enter a valid unsigned integer in base units.");
  return value;
}
export function shorten(value: string) {
  return value.length > 14 ? `${value.slice(0, 6)}…${value.slice(-4)}` : value;
}
