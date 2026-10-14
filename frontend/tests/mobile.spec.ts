import { test, expect } from "@playwright/test";
import { samples } from "../src/lib/demo";
const longTitle =
  "Community rewards for supporters across the entire ecosystem with a very long campaign name";
const unbrokenTitle = "Community".repeat(11);
const campaign = {
  ...samples[0],
  title: longTitle,
  description:
    "A creator-funded community campaign with transparent reward rules.",
  reward: {
    ...samples[0].reward,
    token_address: "0x3600000000000000000000000000000000000000",
    amount_base_units: "2500000000",
  },
};
for (const width of [320, 360, 390, 430, 768]) {
  test(`live campaign titles remain contained at ${width}px on a mobile browser`, async ({
    browser,
  }) => {
    const context = await browser.newContext({
      viewport: { width, height: 844 },
      isMobile: true,
      hasTouch: true,
    });
    const page = await context.newPage();
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.route("**/v1/campaigns?*", (route) =>
      route.fulfill({
        json: {
          items: [
            campaign,
            {
              ...campaign,
              id: samples[1].id,
              title: unbrokenTitle,
              description: "Description".repeat(30),
            },
          ],
          next_cursor: null,
        },
      }),
    );
    await page.route(`**/v1/campaigns/${campaign.id}`, (route) =>
      route.fulfill({ json: { ...campaign, title: unbrokenTitle } }),
    );
    await page.goto("http://127.0.0.1:4311/");
    await expect(page.locator(".campaign-card")).toHaveCount(2);
    await expect(
      page.locator(".campaign-card").nth(1).locator("h3"),
    ).toHaveText(unbrokenTitle);
    async function contained() {
      const dimensions = await page.evaluate(() => ({
        width: innerWidth,
        scroll: document.documentElement.scrollWidth,
        client: document.documentElement.clientWidth,
      }));
      expect(dimensions.width).toBe(width);
      expect(dimensions.client).toBe(width);
      expect(dimensions.scroll).toBeLessThanOrEqual(width);
      for (const selector of [
        ".featured",
        ".campaign-card",
        ".page-heading",
        ".filter-bar",
        ".topbar",
      ]) {
        for (const element of await page.locator(selector).all()) {
          const box = await element.boundingBox();
          expect(box!.x).toBeGreaterThanOrEqual(0);
          expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
        }
      }
    }
    await contained();
    await page.getByRole("button", { name: "Open menu", exact: true }).click();
    await expect(
      page.getByRole("link", { name: "Discover", exact: true }),
    ).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("button", { name: "Open menu", exact: true }),
    ).toBeFocused();
    await page.locator(".campaign-card").first().click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      unbrokenTitle,
    );
    await contained();
    expect(errors).toEqual([]);
    await context.close();
  });
}
test("an empty live marketplace shows no sample campaigns or featured content", async ({
  page,
}) => {
  await page.route("**/v1/campaigns?*", (route) =>
    route.fulfill({ json: { items: [], next_cursor: null } }),
  );
  await page.goto("http://127.0.0.1:4311/");
  await expect(
    page.getByRole("heading", { name: "No campaigns yet", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".campaign-card")).toHaveCount(0);
  await expect(page.locator(".featured")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Clear filters", exact: true }),
  ).toHaveCount(0);
  await expect(page.getByText("Orbit community", { exact: true })).toHaveCount(
    0,
  );
});
