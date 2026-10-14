import type { Campaign } from "./types";
export const earlyAdopter = {
  campaignId: "dc379018-5da0-4fe4-a3c4-c14188ca0c1c",
  creator: "0x88513627a28851d4aa80fe7d9dca000ec38c6852",
  chainId: "5042",
  collection: "0x890870ce940dbbb74c2962d3b1de5f2e0858c2c9",
  artwork: "/collectibles/oppor-early-adopter-v1.webp",
} as const;
export function isEarlyAdopter(c: Campaign): boolean {
  return (
    c.id === earlyAdopter.campaignId &&
    c.chain_id === earlyAdopter.chainId &&
    c.creator_wallet.toLowerCase() === earlyAdopter.creator &&
    c.reward.token_address.toLowerCase() === earlyAdopter.collection
  );
}
