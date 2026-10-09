import { useQuery } from "@tanstack/react-query";
import { erc20Abi, formatUnits, type Address } from "viem";
import { demo, usdcAddress } from "./config";
import { publicClient } from "./wallet";
import type { Campaign } from "./types";
import { samples, sampleRewards } from "./demo";
export function staticReward(c: Campaign) {
  const i = samples.findIndex((s) => s.id === c.id);
  if (demo && i >= 0) return sampleRewards[i];
  if (c.reward.asset_kind === "ERC721")
    return `${c.reward.nft_inventory?.length || c.reward.amount_base_units} NFTs`;
  if (
    c.reward.asset_kind === "ERC20" &&
    usdcAddress &&
    c.reward.token_address.toLowerCase() === usdcAddress.toLowerCase()
  )
    return `${decimalLabel(c.reward.amount_base_units, 6)} USDC`;
  return `${c.reward.amount_base_units} base units`;
}
export function decimalLabel(amount: string, decimals: number) {
  const value = formatUnits(BigInt(amount), decimals);
  const [whole, fraction] = value.split(".");
  if (whole === "0" && fraction && Number(fraction.slice(0, 6)) === 0)
    return "<0.000001";
  return `${whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",")}${fraction ? "." + fraction.slice(0, 6) : ""}`;
}
export function Reward({ campaign: c }: { campaign: Campaign }) {
  const known =
    c.reward.token_address.toLowerCase() === usdcAddress.toLowerCase();
  const metadata = useQuery({
    queryKey: ["token", c.chain_id, c.reward.token_address],
    enabled: !demo && !known && c.reward.asset_kind === "ERC20",
    staleTime: 3600000,
    retry: 0,
    queryFn: async () => {
      const address = c.reward.token_address as Address;
      const [decimals, symbol] = await Promise.all([
        publicClient.readContract({
          address,
          abi: erc20Abi,
          functionName: "decimals",
        }),
        publicClient.readContract({
          address,
          abi: erc20Abi,
          functionName: "symbol",
        }),
      ]);
      if (decimals > 77 || symbol.length > 24)
        throw new Error("Unsupported token metadata.");
      return { decimals, symbol };
    },
  });
  return (
    <span
      title={`${c.reward.amount_base_units} base units · ${c.reward.token_address}`}
    >
      {metadata.data
        ? `${decimalLabel(c.reward.amount_base_units, metadata.data.decimals)} ${metadata.data.symbol}`
        : staticReward(c)}
    </span>
  );
}
