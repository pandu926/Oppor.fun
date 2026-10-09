import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
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
