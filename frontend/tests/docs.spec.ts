import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test("documentation has its own layout and supports navigation, search and section links", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/docs");
  await expect(
    page.getByRole("heading", { name: "Introduction", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".docs-sidebar")).toBeVisible();
  await expect(page.locator(".sidebar")).toHaveCount(0);
  await expect(page.locator(".docs-toc")).toBeVisible();
  await page
    .locator(".docs-sidebar")
    .getByRole("link", { name: "Creating a campaign", exact: true })
    .click();
  await expect(page).toHaveURL(/\/docs\/creating-a-campaign$/);
  await expect(page).toHaveTitle("Creating a campaign · Oppor");
  await page
    .locator(".docs-toc")
    .getByRole("link", { name: "Set the schedule", exact: true })
    .click();
  await expect(page).toHaveURL(/#schedule$/);
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByRole("textbox", { name: "Search guides" }).fill("versioning");
  await expect(page.locator(".docs-search-result")).toHaveCount(1);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "API overview", exact: true }),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/docs\/api-overview#concurrency$/);
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(errors).toEqual([]);
});
test("code snippets copy exact content and search provides an empty state", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/docs/api-overview");
  await page
    .getByRole("button", { name: "Copy code", exact: true })
    .first()
    .click();
  await expect(page.getByRole("button", { name: "Copied code" })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain(
    "GET /v1/campaigns?limit=25&sort=ending",
  );
  await page
    .getByRole("button", { name: "Search documentation", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Search guides" })
    .fill("zzzz-no-document");
  await expect(
    page.getByText("No matching guides.", { exact: false }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
test("documentation remains readable on mobile and navigation closes after selection", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/docs");
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    390,
  );
  await page.getByRole("button", { name: "Open documentation menu" }).click();
  await page
    .locator(".docs-sidebar")
    .getByRole("link", { name: "Funding & launch", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Funding & launch", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Open documentation menu" }),
  ).toHaveAttribute("aria-expanded", "false");
  await page.getByRole("button", { name: "On this page", exact: true }).click();
  await page
    .getByRole("navigation", { name: "Article sections" })
    .getByRole("link", { name: "Activate the campaign", exact: true })
    .click();
  await expect(page).toHaveURL(/#activate$/);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    390,
  );
});
test("documentation has no serious accessibility violations or invented launch claims", async ({
  page,
}) => {
  await page.goto("/docs");
  const results = await new AxeBuilder({ page }).analyze();
  expect(
    results.violations.filter((v) =>
      ["serious", "critical"].includes(v.impact || ""),
    ),
  ).toEqual([]);
  await expect(page.locator(".docs-main")).not.toContainText(
    /simulation|demo mode|sample campaigns|deployed on mainnet|independently audited/i,
  );
  await page.goto("/docs/missing-guide");
  await expect(
    page.getByRole("heading", { name: "Page not found" }),
  ).toBeVisible();
});
