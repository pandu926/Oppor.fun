import { defineConfig } from "vite";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [
    react(),
    {
      name: "documentation-preview-html",
      configurePreviewServer(server) {
        server.middlewares.use((req, _res, next) => {
          const url = new URL(req.url || "/", "http://localhost");
          if (/^\/docs(?:\/[a-z-]+)?\/?$/.test(url.pathname)) {
            const route = url.pathname.replace(/\/$/, "");
            if (existsSync(resolve("dist", route.slice(1), "index.html")))
              req.url = `${route}/index.html${url.search}`;
          }
          next();
        });
      },
    },
  ],
  server: {
    port: 3000,
    strictPort: true,
    proxy: { "/v1": { target: "http://127.0.0.1:8080", changeOrigin: false } },
  },
  preview: { port: 3000, strictPort: true },
  build: {
    sourcemap: false,
    chunkSizeWarningLimit: 600,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            { name: "wallet", test: /node_modules\/(viem|ox|@noble|@scure)/ },
            {
              name: "react",
              test: /node_modules\/(react|react-dom|scheduler|react-router)/,
            },
          ],
        },
      },
    },
  },
});
