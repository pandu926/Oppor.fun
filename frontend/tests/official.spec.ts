import { test, expect } from "@playwright/test";
import { samples } from "../src/lib/demo";
import { earlyAdopter } from "../src/lib/official";
const official = {
  ...samples[0],
  id: earlyAdopter.campaignId,
  chain_id: "5042",
  creator_wallet: earlyAdopter.creator,
  title: "Oppor.fun Early Adopter — Genesis NFT Raffle",
  description:
    "Complete four X tasks for a chance to receive one of 1,000 Early Adopter NFTs.",
  status: "ACTIVE" as const,
  reward: {
    asset_kind: "ERC1155" as const,
    token_address: earlyAdopter.collection,
    amount_base_units: "1000",
    token_id: "1",
    nft_inventory: [],
  },
  distribution: {
    ...samples[0].distribution,
    mode: "RAFFLE" as const,
    winner_count: 1000,
    capacity: 0,
    registration_limit: 100000,
    allocation_policy: "FIXED_REWARD" as const,
    reward_per_recipient: "1",
  },
};
for (const width of [320, 360, 768, 1440]) {
  test(`official campaign remains the headline at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    const other = {
      ...samples[1],
      status: "ACTIVE",
      title: "Another active campaign",
    };
    await page.route("**/v1/campaigns?*", (route) =>
      route.fulfill({ json: { items: [other, official], next_cursor: null } }),
    );
    await page.route(`**/v1/campaigns/${earlyAdopter.campaignId}`, (route) =>
      route.fulfill({ json: official }),
    );
    await page.goto("http://127.0.0.1:4311/");
    await expect(page.locator(".featured h2")).toHaveText(official.title);
    await expect(page.locator(".featured")).toContainText(
      "Official Oppor.fun campaign",
    );
    await expect(page.locator(".featured")).toContainText(
      "1,000 Early Adopter NFTs",
    );
    await expect(page.locator(".official-featured-art")).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page
      .locator(".featured")
      .getByRole("link", { name: "View campaign" })
      .click();
    await expect(page.locator(".campaign-summary")).toContainText(
      "Created by Oppor.fun",
    );
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
  });
}
