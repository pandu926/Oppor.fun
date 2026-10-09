import { test, expect, type Page } from "@playwright/test";
import { samples } from "../src/lib/demo";
const wallet = "0x1111111111111111111111111111111111111111";
const id = samples[0].id;
async function fixture(page: Page, admin = false) {
  await page.addInitScript(() => {
    const listeners = new Map<string, Set<(...args: unknown[]) => void>>();
    let account = "0x1111111111111111111111111111111111111111";
    let chainId = "0x4cef52";
    (
      window as unknown as {
        walletEvent: (event: string, value: unknown) => void;
      }
    ).walletEvent = (event, value) => {
      if (event === "accountsChanged") account = (value as string[])[0];
      if (event === "chainChanged") chainId = value as string;
      listeners.get(event)?.forEach((fn) => fn(value));
    };
    window.ethereum = {
      on: (event: string, fn: (...args: unknown[]) => void) => {
        if (!listeners.has(event)) listeners.set(event, new Set());
        listeners.get(event)!.add(fn);
      },
      removeListener: (event: string, fn: (...args: unknown[]) => void) => {
        listeners.get(event)?.delete(fn);
      },
      request: async ({ method }: { method: string }) => {
        if (method === "eth_requestAccounts" || method === "eth_accounts")
          return account ? [account] : [];
        if (method === "eth_chainId") return chainId;
        if (method === "personal_sign") return "0x" + "a".repeat(130);
        throw new Error(method);
      },
    } as never;
  });
  await page.route("**/v1/auth/challenge", (r) =>
    r.fulfill({
      json: { message: "Sign in to test", expires_at: "2099-01-01T00:00:00Z" },
    }),
  );
  await page.route("**/v1/auth/verify", (r) =>
    r.fulfill({
      json: {
        wallet,
        user_id: "fixture-user",
        csrf_token: "fixture-csrf",
        expires_in: 3600,
      },
    }),
  );
  await page.route("**/v1/admin/me", (r) =>
    r.fulfill(
      admin
        ? { json: { role: "ADMIN" } }
        : {
            status: 403,
            json: { error: { code: "FORBIDDEN", message: "Not an operator" } },
          },
    ),
  );
}
async function login(page: Page) {
  await page
    .locator(".topbar")
    .getByRole("button", { name: "Connect wallet" })
    .click();
  await page
    .getByRole("button", { name: "Browser wallet", exact: true })
    .click();
  await page.getByRole("button", { name: "Sign in to Oppor" }).click();
  await expect(page.getByLabel("Disconnect wallet")).toBeVisible();
}
test("creator review reads evidence and submits versioned decisions", async ({
  page,
}) => {
  await fixture(page);
  const campaign = {
    ...samples[0],
    creator_wallet: wallet,
    status: "REVIEWING",
    cutoff_at: "2026-01-02T00:00:00Z",
  };
  const entry = {
    id: "entry-1",
    campaign_id: id,
    payout_wallet: "0x2222222222222222222222222222222222222222",
    slot_number: 1,
    status: "SUBMITTED",
    version: 3,
    submitted_at: "2026-01-01T00:00:00Z",
  };
  let decision = false;
  await page.route(`**/v1/campaigns/${id}`, (r) =>
    r.fulfill({ json: campaign }),
  );
  await page.route(`**/v1/campaigns/${id}/entries?*`, (r) =>
    r.fulfill({ json: { items: [entry], next_cursor: null } }),
  );
  await page.route(`**/v1/campaigns/${id}/entries/entry-1/evidence`, (r) =>
    r.fulfill({
      json: {
        entry,
        x_username_declared: "tester",
        discord_username_declared: null,
        evidence: [
          {
            task_id: "task-1",
            revision: 1,
            evidence: {
              text: "My task evidence",
              url: "https://x.com/test/status/123",
              upload_id: null,
            },
            image_url: null,
          },
        ],
      },
    }),
  );
  await page.route(`**/v1/campaigns/${id}/entries/entry-1/review`, (r) => {
    expect(r.request().postDataJSON()).toEqual({
      expected_version: 3,
      decision: "ELIGIBLE",
      reason: "Evidence meets the task rules",
    });
    expect(r.request().headers()["x-csrf-token"]).toBe("fixture-csrf");
    decision = true;
    return r.fulfill({
      json: { entry_id: entry.id, status: "ELIGIBLE", version: 4 },
    });
  });
  await page.goto(`http://127.0.0.1:4311/campaigns/${id}/manage`);
  await login(page);
  await page.getByRole("button", { name: /0x2222.*Review/ }).click();
  await expect(page.getByText("My task evidence")).toBeVisible();
  await page
    .getByLabel("Reason", { exact: true })
    .fill("Evidence meets the task rules");
  await page.getByRole("button", { name: "Save review decision" }).click();
  await expect.poll(() => decision).toBe(true);
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
test("operator moderation sends separate administration version and audit reason", async ({
  page,
}) => {
  await fixture(page, true);
  await page.route("**/v1/admin/stats", (r) =>
    r.fulfill({
      json: {
        counts: { users: 2, campaigns: 1, entries: 1, claimed_allocations: 0 },
        indexer: null,
        generated_at: "2026-01-01T00:00:00Z",
      },
    }),
  );
  await page.route("**/v1/admin/campaigns?*", (r) =>
    r.fulfill({
      json: {
        items: [
          {
            campaign: samples[0],
            moderation: {
              hidden: false,
              version: 7,
              reason: null,
              updated_at: null,
            },
          },
        ],
        next_cursor: null,
      },
    }),
  );
  let moderated = false;
  await page.route(`**/v1/admin/campaigns/${id}/moderation`, (r) => {
    expect(r.request().postDataJSON()).toEqual({
      expected_version: 7,
      reason: "Investigating community report",
      hidden: true,
    });
    moderated = true;
    return r.fulfill({ json: { campaign_id: id, hidden: true, version: 8 } });
  });
  await page.goto("http://127.0.0.1:4311/admin");
  await login(page);
  await page.getByRole("button", { name: "Hide", exact: true }).click();
  await page
    .getByLabel("Reason", { exact: true })
    .fill("Investigating community report");
  await page.getByRole("button", { name: "Confirm action" }).click();
  await expect.poll(() => moderated).toBe(true);
});
test("claim preparation rejects an unexpected contract before wallet send", async ({
  page,
}) => {
  await fixture(page);
  const campaign = {
    ...samples[0],
    status: "CLAIM_OPEN",
    escrow_address: "0x4444444444444444444444444444444444444444",
  };
  await page.route(`**/v1/campaigns/${id}`, (r) =>
    r.fulfill({ json: campaign }),
  );
  await page.route(`**/v1/campaigns/${id}/my-entry`, (r) =>
    r.fulfill({
      status: 404,
      json: { error: { code: "NOT_FOUND", message: "No entry" } },
    }),
  );
  await page.route(`**/v1/campaigns/${id}/allocations/${wallet}`, (r) =>
    r.fulfill({
      json: {
        campaign_id: id,
        escrow: campaign.escrow_address,
        claim_deadline: "2099-01-01T00:00:00Z",
        allocations: [
          {
            index: "0",
            recipient: wallet,
            token_id: "0",
            quantity: "1000000",
            proof: [],
            claim_tx_hash: null,
            claimed_at: null,
          },
        ],
      },
    }),
  );
  await page.route(`**/v1/campaigns/${id}/prepare-claim`, (r) =>
    r.fulfill({
      json: {
        chain_id: "5042002",
        to: wallet,
        data: "0x1234",
        value: "0",
        expected_sender: wallet,
        intent: "CLAIM",
        expires_at: "2099-01-01T00:00:00Z",
      },
    }),
  );
  await page.goto(`http://127.0.0.1:4311/campaigns/${id}`);
  await login(page);
  await page.getByRole("button", { name: "Claim reward", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Transaction target does not match",
  );
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

for (const event of [
  "accountsChanged",
  "chainChanged",
  "disconnect",
] as const) {
  test(`wallet ${event} revokes the authenticated session`, async ({
    page,
  }) => {
    await fixture(page);
    await page.route("**/v1/campaigns?*", (r) =>
      r.fulfill({ json: { items: [], next_cursor: null } }),
    );
    let revoked = false;
    await page.route("**/v1/auth/logout", (r) => {
      expect(r.request().headers()["x-csrf-token"]).toBe("fixture-csrf");
      revoked = true;
      return r.fulfill({ status: 204 });
    });
    await page.goto("http://127.0.0.1:4311/");
    await login(page);
    await page.evaluate(
      (event) =>
        (
          window as unknown as {
            walletEvent: (event: string, value: unknown) => void;
          }
        ).walletEvent(
          event,
          event === "accountsChanged"
            ? ["0x2222222222222222222222222222222222222222"]
            : event === "chainChanged"
              ? "0x1"
              : { code: 4900 },
        ),
      event,
    );
    await expect.poll(() => revoked).toBe(true);
    await expect(page.getByLabel("Disconnect wallet")).toHaveCount(0);
    await expect(
      page.locator(".topbar").getByRole("button", { name: "Connect wallet" }),
    ).toBeVisible();
  });
}
test("rejected wallet signature grants no session", async ({ page }) => {
  await fixture(page);
  await page.route("**/v1/campaigns?*", (r) =>
    r.fulfill({ json: { items: [], next_cursor: null } }),
  );
  await page.goto("http://127.0.0.1:4311/");
  await page.evaluate(() => {
    const original = window.ethereum.request;
    window.ethereum.request = async (args: { method: string }) => {
      if (args.method === "personal_sign")
        throw Object.assign(new Error("User rejected the request."), {
          code: 4001,
        });
      return original(args);
    };
  });
  await page
    .locator(".topbar")
    .getByRole("button", { name: "Connect wallet" })
    .click();
  await page
    .getByRole("button", { name: "Browser wallet", exact: true })
    .click();
  await page.getByRole("button", { name: "Sign in to Oppor" }).click();
  await expect(page.getByRole("alert")).toContainText(/rejected/i);
  await expect(page.getByLabel("Disconnect wallet")).toHaveCount(0);
});
