import { defineChain } from "viem";
const env = import.meta.env || {};
export const demo = env.VITE_DATA_MODE !== "live";
export const siteUrl = env.VITE_SITE_URL || "https://oppai.fun";
export const apiBase = env.VITE_API_BASE_URL || "/v1";
export function secureUrl(value: string, local = false): string {
  const u = new URL(value);
  if (
    u.username ||
    u.password ||
    (u.protocol !== "https:" &&
      !(
        local &&
        u.protocol === "http:" &&
        ["localhost", "127.0.0.1"].includes(u.hostname)
      ))
  )
    throw new Error("An HTTPS URL is required.");
  return u.href;
}
export const chain = defineChain({
  id: Number(env.VITE_CHAIN_ID || 5042002),
  name: "Arc",
  nativeCurrency: { name: "USDC", symbol: "USDC", decimals: 18 },
  rpcUrls: {
    default: {
      http: [
        secureUrl(env.VITE_RPC_URL || "https://rpc.testnet.arc.io", env.DEV),
      ],
    },
  },
  blockExplorers: {
    default: {
      name: "Arc Explorer",
      url: secureUrl(
        env.VITE_EXPLORER_URL || "https://explorer.testnet.arc.io",
        env.DEV,
      ),
    },
  },
});
if (!Number.isSafeInteger(chain.id) || chain.id <= 0)
  throw new Error("Invalid configured chain ID.");
export const factoryAddress = env.VITE_FACTORY_ADDRESS || "";

export const usdcAddress =
  env.VITE_USDC_ADDRESS ||
  (chain.id === 5042002 ? "0x3600000000000000000000000000000000000000" : "");
