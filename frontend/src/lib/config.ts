import { defineChain } from "viem";
const env = import.meta.env || {};
// Sample data is available only in an explicitly selected development fixture.
export const demo =
  import.meta.env?.DEV === true && import.meta.env?.VITE_DATA_MODE === "demo";
export const siteUrl = env.VITE_SITE_URL || "https://oppor.fun";
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
const configuredChainId = Number(
  env.VITE_CHAIN_ID || (env.PROD ? 5042 : 5042002),
);
export const chain = defineChain({
  id: configuredChainId,
  name: configuredChainId === 5042002 ? "Arc Testnet" : "Arc",
  nativeCurrency: { name: "USDC", symbol: "USDC", decimals: 18 },
  rpcUrls: {
    default: {
      http: [
        secureUrl(
          env.VITE_RPC_URL ||
            (configuredChainId === 5042
              ? "https://rpc.mainnet.arc.io"
              : "https://rpc.testnet.arc.io"),
          env.DEV,
        ),
      ],
    },
  },
  blockExplorers: {
    default: {
      name: "Arc Explorer",
      url: secureUrl(
        env.VITE_EXPLORER_URL ||
          (configuredChainId === 5042
            ? "https://explorer.arc.io"
            : "https://explorer.testnet.arc.io"),
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
  ([5042, 5042002].includes(chain.id)
    ? "0x3600000000000000000000000000000000000000"
    : "");
