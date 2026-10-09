import { defineConfig } from "@playwright/test";
import { existsSync } from "node:fs";
const executable =
  process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);
export default defineConfig({
  testDir: "./tests",
  testMatch: "*.spec.ts",
  fullyParallel: true,
  timeout: 30000,
  retries: process.env.CI ? 1 : 0,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:3000",
    viewport: { width: 1536, height: 1024 },
    launchOptions: { executablePath: executable, args: ["--no-sandbox"] },
    trace: "retain-on-failure",
  },
  webServer: [
    {
      command: "npm run dev",
      url: "http://127.0.0.1:3000",
      reuseExistingServer: !process.env.CI,
    },
    {
      command: "VITE_DATA_MODE=live npm run dev -- --port 4311",
      url: "http://127.0.0.1:4311",
      reuseExistingServer: !process.env.CI,
    },
  ],
});
