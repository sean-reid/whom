import { expect, test, type Page } from "@playwright/test";
import pool from "../fixtures/pool.json" with { type: "json" };
import { gotoLaunchDay } from "./helpers";

async function openToday(page: Page) {
  await gotoLaunchDay(page);
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator("#guess-form")).toBeVisible();
  await expect
    .poll(() => page.locator("#portrait").evaluate((img) => (img as HTMLImageElement).naturalWidth))
    .toBeGreaterThan(0);
}

test("plays today's puzzle through to the result", async ({ page, context }, info) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await openToday(page);
  await expect(page.locator("#remaining")).toHaveText("8 guesses left");

  await page.fill("#guess", "ala");
  await expect(page.locator("#suggest li")).toContainText(["Alain", "Alan"]);
  await page.keyboard.press("Escape");
  await expect(page.locator("#suggest")).toBeHidden();

  await page.fill("#guess", "Zzyzx");
  await page.keyboard.press("Enter");
  await expect(page.locator("#notice")).toHaveText("Not a name I know.");
  await expect(page.locator("#guesses li, #latest li")).toHaveCount(0);

  // Two names no fixture person carries, so the sorted list and the pinned row both appear
  // before the win whichever person today's puzzle is.
  const misses = ["Pierre", "Maria"];
  let rows = 0;
  for (const display of [...misses, ...pool.people.map((p) => p.display)]) {
    await page.fill("#guess", display);
    await page.keyboard.press("Enter");
    rows += 1;
    await expect(page.locator("#guesses li, #latest li")).toHaveCount(rows);
    if (rows === 1) {
      const phrase = await page.locator("#latest li .line > *").first().textContent();
      await expect(page.locator("#announce")).toHaveText(
        new RegExp(`^${phrase}, .*\\. 7 guesses left$`),
      );
    }
    if (rows === 2) {
      await expect(page.locator("#facts li").first()).toHaveText(/^Born in the \d{4}s$/);
      await expect(page.locator("#latest li .name")).toHaveText(display, {
        ignoreCase: true,
      });
      await page.reload();
      await expect(page.locator("#guesses li")).toHaveCount(1);
      await expect(page.locator("#latest li")).toHaveCount(1);
      await page.screenshot({ path: info.outputPath("mid.png"), fullPage: true });
    }
    if (await page.locator("#end").isVisible()) break;
  }
  await expect(page.locator("#end")).toBeVisible();
  await expect(page.locator("#end")).toBeFocused();
  await expect(page.locator("#guess-form")).toBeHidden();
  const winner = pool.people[rows - misses.length - 1];
  await expect(page.locator("#result")).toContainText(winner?.label ?? "");
  await expect(page.locator("#credit")).toContainText("via Wikimedia Commons");
  await expect(page.locator("#figures dd").first()).toHaveText("1");
  await expect(page.locator("#dist-note")).toContainText("everyone today");
  await expect(page.locator("#dist .count .all")).toHaveCount(9);
  await expect(page.locator("#dist .visually-hidden").nth(rows - 1)).toHaveText(
    new RegExp(`^Won in ${rows}: 1 of your games, [1-9]\\d* of everyone's$`),
  );
  await page.screenshot({ path: info.outputPath("result.png"), fullPage: true });

  await page.click("#share");
  await expect(page.locator("#share-done")).toHaveText("Copied");
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied).toMatch(/^WHOM\? #\d+ \d\/8 whom\.dwainosaur\.com$/);

  await page.reload();
  await expect(page.locator("#end")).toBeVisible();
  await expect(page.locator("#guesses li")).toHaveCount(rows);
  await expect(page.locator("#latest li")).toHaveCount(0);
  await expect(page.locator("#guesses li .name").first()).toHaveText(winner?.display ?? "", {
    ignoreCase: true,
  });
});
