import { test, expect, type Page } from "@playwright/test";
async function fixture(page: Page, rpcChain = "0x13b2") {
  await page.addInitScript(() => {
    let chain = "0x4cef52";
    let address = "0x1111111111111111111111111111111111111111";
    const listeners = new Map<string, Set<(...args: unknown[]) => void>>();
    (
      window as unknown as { changeDeploymentWallet: () => void }
    ).changeDeploymentWallet = () => {
      address = "0x2222222222222222222222222222222222222222";
      listeners.get("accountsChanged")?.forEach((fn) => fn([address]));
    };
    window.ethereum = {
      on: (name: string, fn: (...args: unknown[]) => void) => {
        if (!listeners.has(name)) listeners.set(name, new Set());
        listeners.get(name)!.add(fn);
      },
      removeListener: (name: string, fn: (...args: unknown[]) => void) =>
        listeners.get(name)?.delete(fn),
      request: async ({
        method,
        params,
      }: {
        method: string;
        params?: Array<{ chainId: string }>;
      }) => {
        if (["eth_requestAccounts", "eth_accounts"].includes(method))
          return [address];
        if (method === "eth_chainId") return chain;
        if (method === "wallet_switchEthereumChain") {
          chain = params![0].chainId;
          listeners.get("chainChanged")?.forEach((fn) => fn(chain));
          return null;
        }
        if (method === "eth_sendTransaction") {
          (window as unknown as { deploymentSent: boolean }).deploymentSent =
            true;
          throw Object.assign(new Error("User rejected deployment."), {
            code: 4001,
          });
        }
        throw new Error(method);
      },
    } as never;
  });
  await page.route("https://rpc.mainnet.arc.io/**", (r) => {
    const body = r.request().postDataJSON();
    const answer = (q: { id: number; method: string }) => ({
      jsonrpc: "2.0",
      id: q.id,
      result:
        q.method === "eth_chainId"
          ? rpcChain
          : q.method === "eth_estimateGas"
            ? "0x2710"
            : q.method === "eth_gasPrice"
              ? "0x3b9aca00"
              : q.method === "eth_getBalance"
                ? "0xde0b6b3a7640000"
                : "0x1",
    });
    return r.fulfill({
      json: Array.isArray(body) ? body.map(answer) : answer(body),
    });
  });
  await page.goto("/setup/deploy");
  await page
    .getByRole("button", { name: "Connect wallet", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Browser wallet", exact: true })
    .click();
}
test("deployment tool rejects an RPC on the wrong network", async ({
  page,
}) => {
  await fixture(page, "0x1");
  await page.getByRole("button", { name: "Prepare deployment" }).click();
  await expect(page.getByRole("alert")).toContainText("expected Arc mainnet");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
test("wallet changes after review block contract deployment", async ({
  page,
}) => {
  await fixture(page);
  await page.getByRole("button", { name: "Prepare deployment" }).click();
  await expect(page.getByRole("dialog")).toContainText(
    "Review factory deployment",
  );
  await page.evaluate(() =>
    (
      window as unknown as { changeDeploymentWallet: () => void }
    ).changeDeploymentWallet(),
  );
  await page.getByRole("button", { name: "Deploy from wallet" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Wallet or network changed",
  );
  expect(
    await page.evaluate(
      () => (window as unknown as { deploymentSent?: boolean }).deploymentSent,
    ),
  ).toBeUndefined();
});
test("rejected deployment signature never reports a deployed factory", async ({
  page,
}) => {
  await fixture(page);
  await page.getByRole("button", { name: "Prepare deployment" }).click();
  await page.getByRole("button", { name: "Deploy from wallet" }).click();
  await expect(page.getByRole("alert")).toContainText(/rejected/i);
  await expect(
    page.getByRole("button", { name: "Download deployment manifest" }),
  ).toHaveCount(0);
});
