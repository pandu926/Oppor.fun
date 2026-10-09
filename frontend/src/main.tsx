import { createRoot, hydrateRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import {
  QueryClient,
  QueryClientProvider,
  HydrationBoundary,
  type DehydratedState,
} from "@tanstack/react-query";
import { App } from "./app";
import { AppProvider } from "./lib/context";
import "@fontsource-variable/inter";
import "./styles.css";
import { demo } from "./lib/config";
import { loadDemo } from "./lib/demo";
if (demo) loadDemo();
declare global {
  interface Window {
    __OPPOR__?: { path: string; state: DehydratedState };
  }
}
const query = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, staleTime: 15000, refetchOnWindowFocus: false },
    mutations: { retry: false },
  },
});
const bootstrap = window.__OPPOR__;
const content = (
  <QueryClientProvider client={query}>
    <HydrationBoundary
      state={
        bootstrap?.path === location.pathname ? bootstrap.state : undefined
      }
    >
      <BrowserRouter>
        <AppProvider>
          <App />
        </AppProvider>
      </BrowserRouter>
    </HydrationBoundary>
  </QueryClientProvider>
);
const root = document.getElementById("root")!;
if (bootstrap?.path === location.pathname && root.hasChildNodes())
  hydrateRoot(root, content);
else createRoot(root).render(content);
