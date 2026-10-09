import { renderToString } from "react-dom/server";
import { StaticRouter } from "react-router-dom";
import {
  QueryClient,
  QueryClientProvider,
  dehydrate,
} from "@tanstack/react-query";
import {
  WalletProvider,
  WalletUIProvider,
  createWalletConfig,
} from "./lib/rainbow";
import { App, seo } from "./app";
import { AppProvider } from "./lib/context";
import type { Campaign } from "./lib/types";
export function render(path: string, campaigns: Campaign[] = []) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: 15000 } },
  });
  if (campaigns.length) {
    client.setQueryData(["campaigns", false, "ending", undefined], {
      pages: [{ items: campaigns, next_cursor: null }],
      pageParams: [null],
    });
    campaigns.forEach((c) => client.setQueryData(["campaign", c.id], c));
  }
  const html = renderToString(
    <WalletProvider config={createWalletConfig()}>
      <QueryClientProvider client={client}>
        <WalletUIProvider>
          <StaticRouter location={path}>
            <AppProvider>
              <App />
            </AppProvider>
          </StaticRouter>
        </WalletUIProvider>
      </QueryClientProvider>
    </WalletProvider>,
  );
  return {
    html,
    state: dehydrate(client),
    meta: seo(
      path,
      campaigns.find((c) => path === `/campaigns/${c.id}`),
    ),
  };
}

export { docPages } from "./docs/content";
