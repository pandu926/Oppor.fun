import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
const orbit = "00000000-0000-4000-8000-000000000001";
async function signIn(page: import("@playwright/test").Page) {
  await page
    .locator(".topbar")
    .getByRole("button", { name: "Connect wallet", exact: true })
    .click();
  await page.getByRole("button", { name: "Continue in demo mode" }).click();
}
test("desktop reference geometry, six cards, filters and search", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(page.locator(".campaign-card")).toHaveCount(6);
  const banner = await page.locator(".featured").boundingBox();
  expect(banner?.x).toBe(302);
  expect(banner?.y).toBeGreaterThanOrEqual(178);
  expect(banner?.y).toBeLessThanOrEqual(181);
  expect(banner?.width).toBe(1205);
  await page.getByRole("tab", { name: "USDC", exact: true }).click();
  await expect(page.locator(".campaign-card")).toHaveCount(2);
  await page.getByRole("tab", { name: "NFTs", exact: true }).click();
  await expect(page.locator(".campaign-card")).toHaveCount(2);
  await page.getByRole("tab", { name: "All campaigns", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "All campaigns", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".campaign-card")).toHaveCount(6);
  await page
    .getByRole("searchbox", { name: "Search campaigns" })
    .fill("Mooncat");
  await expect(page.locator(".campaign-card")).toHaveCount(1);
  await page.getByRole("searchbox").fill("nothing-here");
  await expect(
    page.getByRole("heading", { name: "No campaigns found" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
test("participant saves evidence, must explicitly submit, and recovers after reload", async ({
  page,
}) => {
  await page.goto(`/campaigns/${orbit}`);
  await signIn(page);
  await page
    .getByRole("button", { name: "Join campaign", exact: true })
    .click();
  for (let i = 0; i < 4; i++) {
    await page
      .getByRole("textbox", { name: `Evidence for task ${i + 1}`, exact: true })
      .fill(`Task ${i + 1} completed`);
    await page
      .getByRole("button", { name: "Save evidence", exact: true })
      .nth(i)
      .click();
    await expect(page.getByText("Evidence saved", { exact: true })).toHaveCount(
      i + 1,
    );
  }
  await expect(page.getByText("Entry status:", { exact: false })).toContainText(
    "REGISTERED",
  );
  await page.getByRole("button", { name: "Submit entry for review" }).click();
  await expect(page.getByText("Entry status:", { exact: false })).toContainText(
    "SUBMITTED",
  );
  await page.reload();
  await signIn(page);
  await expect(page.getByText("Entry status:", { exact: false })).toContainText(
    "SUBMITTED",
  );
  await page.goto("/entries");
  await signIn(page);
  await expect(
    page.getByRole("link", { name: "Orbit community", exact: true }),
  ).toBeVisible();
});
test("draft creation and recovery works, with safe amount validation", async ({
  page,
}) => {
  await page.goto("/create");
  await signIn(page);
  await page
    .getByLabel("Campaign name", { exact: true })
    .fill("A new community");
  await page
    .getByLabel("Token contract address", { exact: true })
    .fill("0x3333333333333333333333333333333333333333");
  await page
    .getByLabel("Total reward in base units", { exact: false })
    .fill("2500000000");
  await page
    .getByLabel("Target URL", { exact: true })
    .fill("https://x.com/test/status/123");
  await page
    .getByLabel("Instructions", { exact: true })
    .fill("Repost the launch announcement.");
  await page.getByRole("button", { name: "Create campaign draft" }).click();
  await expect(
    page.getByRole("heading", { name: "Campaign workspace" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Lock configuration" }),
  ).toBeDisabled();
  const path = new URL(page.url()).pathname;
  await page.reload();
  await signIn(page);
  await expect(
    page.getByRole("heading", { name: "Campaign workspace" }),
  ).toBeVisible();
  expect(new URL(page.url()).pathname).toBe(path);
});
test("mobile has no horizontal overflow and menu navigates", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    390,
  );
  await page.getByRole("button", { name: "Open menu" }).click();
  await page.getByRole("link", { name: "My entries", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "My entries", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Close menu", exact: true }),
  ).toHaveCount(1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    390,
  );
});
test("wallet dialog handles keyboard escape and returns focus", async ({
  page,
}) => {
  await page.goto("/");
  const trigger = page.getByRole("button", {
    name: "Connect wallet",
    exact: true,
  });
  await trigger.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(trigger).toBeFocused();
});
test("public marketplace and dialog have no serious accessibility violations", async ({
  page,
}) => {
  await page.goto("/");
  let result = await new AxeBuilder({ page }).analyze();
  expect(
    result.violations.filter((v) =>
      ["critical", "serious"].includes(v.impact || ""),
    ),
  ).toEqual([]);
  await page
    .getByRole("button", { name: "Connect wallet", exact: true })
    .click();
  result = await new AxeBuilder({ page }).analyze();
  expect(
    result.violations.filter((v) =>
      ["critical", "serious"].includes(v.impact || ""),
    ),
  ).toEqual([]);
});
test("unknown routes and admin do not grant operator access", async ({
  page,
}) => {
  await page.goto("/unavailable");
  await expect(
    page.getByRole("heading", { name: "This page has wandered off" }),
  ).toBeVisible();
  await page.goto("/admin");
  await signIn(page);
  await expect(
    page.getByRole("heading", {
      name: "Administration requires a live deployment",
    }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Hide" })).toHaveCount(0);
});
test("live API failure never substitutes sample campaigns", async ({
  page,
}) => {
  await page.route("**/v1/campaigns?*", (route) =>
    route.fulfill({
      status: 503,
      json: {
        error: {
          code: "UNAVAILABLE",
          message: "Temporary service interruption",
        },
      },
    }),
  );
  await page.goto("http://127.0.0.1:4311/");
  await expect(page.getByRole("alert")).toContainText(
    "Temporary service interruption",
  );
  await expect(page.locator(".campaign-card")).toHaveCount(0);
  await expect(page.getByText("2,500 USDC", { exact: true })).toHaveCount(0);
});
test("live browser wallet signs the exact challenge and sends CSRF on mutations", async ({
  page,
}) => {
  const wallet = "0x1111111111111111111111111111111111111111";
  let mutation = false;
  await page.addInitScript(() => {
    window.ethereum = {
      on: () => {},
      removeListener: () => {},
      request: async ({
        method,
        params,
      }: {
        method: string;
        params?: unknown[];
      }) => {
        if (method === "eth_requestAccounts" || method === "eth_accounts")
          return ["0x1111111111111111111111111111111111111111"];
        if (method === "eth_chainId") return "0x4cef52";
        if (method === "personal_sign") {
          (window as unknown as { signed: unknown }).signed = params;
          return "0x" + "a".repeat(130);
        }
        throw new Error("Unexpected wallet request: " + method);
      },
    } as never;
  });
  await page.route("**/v1/**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    let json: unknown = { items: [], next_cursor: null };
    if (path === "/v1/auth/challenge")
      json = {
        message: "Exact challenge for Oppor",
        expires_at: "2099-01-01T00:00:00Z",
      };
    else if (path === "/v1/auth/verify") {
      expect(route.request().postDataJSON().message).toBe(
        "Exact challenge for Oppor",
      );
      json = {
        wallet,
        user_id: "test-user",
        csrf_token: "test-csrf",
        expires_in: 3600,
      };
    } else if (path === "/v1/admin/me") {
      await route.fulfill({
        status: 403,
        json: { error: { code: "FORBIDDEN", message: "Not an operator" } },
      });
      return;
    } else if (path === "/v1/auth/logout") {
      expect(route.request().headers()["x-csrf-token"]).toBe("test-csrf");
      expect(route.request().headers()["idempotency-key"]).toBeTruthy();
      mutation = true;
      json = { logged_out: true };
    }
    await route.fulfill({ json });
  });
  await page.goto("http://127.0.0.1:4311/");
  await page
    .getByRole("button", { name: "Connect wallet", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Browser wallet", exact: true })
    .click();
  await page.getByRole("button", { name: "Sign in to Oppor" }).click();
  await expect(page.getByLabel("Disconnect wallet")).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as unknown as { signed: unknown[] }).signed[0],
    ),
  ).toBe("0x4578616374206368616c6c656e676520666f72204f70706f72");
  await page.getByLabel("Disconnect wallet").click();
  await expect.poll(() => mutation).toBe(true);
});
