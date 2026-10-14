import {
  RainbowKitProvider,
  connectorsForWallets,
  lightTheme,
  type Wallet,
} from "@rainbow-me/rainbowkit";
import {
  injectedWallet,
  metaMaskWallet,
  rabbyWallet,
  rainbowWallet,
  walletConnectWallet,
} from "@rainbow-me/rainbowkit/wallets";
import { createConfig, createConnector, http, WagmiProvider } from "wagmi";
import { injected } from "wagmi/connectors";
import type { ReactNode } from "react";
import { chain, siteUrl } from "./config";
const projectId = (import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || "").trim();
if (projectId && !/^[a-f0-9]{32}$/i.test(projectId))
  throw new Error(
    "VITE_WALLETCONNECT_PROJECT_ID must be a valid WalletConnect project ID.",
  );
export const walletConnectEnabled = !!projectId;
// This connector uses injected wallets only; no invented relay project ID is used.
const browserWallet = (): Wallet => ({
  id: "browser-wallet",
  name: "Browser wallet",
  iconUrl: "/favicon.svg",
  iconBackground: "#eef2ff",
  installed: typeof window !== "undefined" && !!window.ethereum,
  createConnector: (details) =>
    createConnector((config) => ({
      ...injected({ shimDisconnect: true })(config),
      ...details,
    })),
});
const connectors = connectorsForWallets(
  [
    {
      groupName: "Wallets",
      wallets: projectId
        ? [
            metaMaskWallet,
            rabbyWallet,
            rainbowWallet,
            injectedWallet,
            walletConnectWallet,
          ]
        : [browserWallet],
    },
  ],
  {
    appName: "Oppor",
    appDescription: "Community campaigns and creator-funded rewards on Arc",
    appUrl: siteUrl,
    appIcon: siteUrl.replace(/\/$/, "") + "/favicon.svg",
    projectId,
  },
);
export function createWalletConfig() {
  return createConfig({
    chains: [chain],
    connectors,
    transports: {
      [chain.id]: http(chain.rpcUrls.default.http[0], { timeout: 15000 }),
    },
    ssr: true,
  });
}
export const walletConfig = createWalletConfig();
export function WalletProvider({
  children,
  config = walletConfig,
}: {
  children: ReactNode;
  config?: ReturnType<typeof createWalletConfig>;
}) {
  return <WagmiProvider config={config}>{children}</WagmiProvider>;
}
export function WalletUIProvider({ children }: { children: ReactNode }) {
  return (
    <RainbowKitProvider
      locale="en-US"
      initialChain={chain}
      modalSize="compact"
      theme={lightTheme({
        accentColor: "#4338ca",
        accentColorForeground: "#ffffff",
        borderRadius: "medium",
        fontStack: "system",
        overlayBlur: "small",
      })}
      appInfo={{ appName: "Oppor", learnMoreUrl: "/docs" }}
    >
      {children}
    </RainbowKitProvider>
  );
}
