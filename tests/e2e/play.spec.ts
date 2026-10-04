import { expect, test, type Page } from "@playwright/test";
import { GAME_KEY } from "../../src/state";
import pool from "../fixtures/pool.json" with { type: "json" };
import { gotoLaunchDay, launchNumber } from "./helpers";

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
  await expect(page.locator("#suggest li").filter({ hasText: /^Alan$/ })).toHaveCount(1);
  await expect(page.locator("#suggest li").filter({ hasText: /^Alain$/ })).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(page.locator("#suggest")).toBeHidden();

  await page.fill("#guess", "Zzyzx");
  await page.keyboard.press("Enter");
  await expect(page.locator("#notice")).toHaveText("Not a name I know.");
  await expect(page.locator("#guesses li, #latest li")).toHaveCount(0);

  // Tab fills the highlighted option and leaves the submit to Enter.
  await page.fill("#guess", "pie");
  await expect(page.locator("#suggest li")).toContainText(["Pierre"]);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Tab");
  await expect(page.locator("#guess")).toHaveValue("Pierre");
  await expect(page.locator("#suggest")).toBeHidden();
  await expect(page.locator("#guess")).toBeFocused();
  await expect(page.locator("#guesses li, #latest li")).toHaveCount(0);
  await page.keyboard.press("Enter");
  await expect(page.locator("#guesses li")).toHaveCount(0);
  await expect(page.locator("#latest li")).toHaveCount(1);
  await expect(page.locator("#latest li .name")).toHaveText("Pierre", { ignoreCase: true });
  const phrase = await page.locator("#latest li .line > *").first().textContent();
  await expect(page.locator("#announce")).toHaveText(
    new RegExp(`^${phrase}, .*\\. 7 guesses left$`),
  );

  // Two ArrowDowns highlight the second option and Enter submits it. No fixture person is
  // called Pierre or any Mar- name, so both lists appear before the win whoever today is.
  await page.fill("#guess", "mar");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  const second = page.locator("#suggest li").nth(1);
  await expect(second).toHaveAttribute("aria-selected", "true");
  const secondName = (await second.textContent()) ?? "";
  await page.keyboard.press("Enter");
  await expect(page.locator("#guesses li")).toHaveCount(2);
  await expect(page.locator("#latest li .name")).toHaveText(secondName, { ignoreCase: true });
  await expect(page.locator("#guesses li.current .name")).toHaveText(secondName, {
    ignoreCase: true,
  });
  await expect(page.locator("#facts li").first()).toHaveText(/^Born in the \d{4}s$/);
  await page.reload();
  await expect(page.locator("#guesses li")).toHaveCount(2);
  await expect(page.locator("#latest li")).toHaveCount(1);
  await page.screenshot({ path: info.outputPath("mid.png"), fullPage: true });

  const misses = 2;
  let rows = misses;
  for (const display of pool.people.map((p) => p.display)) {
    await page.fill("#guess", display);
    await page.keyboard.press("Enter");
    rows += 1;
    await expect(
      page.locator("#guesses li .name").filter({ hasText: new RegExp(`^${display}$`, "i") }),
    ).toHaveCount(1);
    if (await page.locator("#end").isVisible()) break;
  }
  await expect(page.locator("#end")).toBeVisible();
  await expect(page.locator("#end")).toBeFocused();
  await expect(page.locator("#guess-form")).toBeHidden();
  const winner = pool.people[rows - misses - 1];
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
  await page.evaluate(() => {
    navigator.clipboard.writeText = () => Promise.reject(new Error("denied"));
  });
  await page.click("#share");
  await expect(page.locator("#share-done")).toHaveText(copied);

  await page.reload();
  await expect(page.locator("#end")).toBeVisible();
  await expect(page.locator("#guesses li")).toHaveCount(rows);
  await expect(page.locator("#latest li")).toHaveCount(0);
  await expect(page.locator("#guesses li .name").first()).toHaveText(winner?.display ?? "", {
    ignoreCase: true,
  });
});

const refusals = [
  { status: 422, error: "already guessed", notice: "You already tried that one." },
  { status: 429, error: "slow down", notice: "Slow down a little." },
  { status: 409, error: "game over", notice: "This game is over. Reload for today's result." },
  {
    status: 500,
    error: "answer has no name record",
    notice: "That guess did not get checked, the server answered 500. Try again.",
  },
];

// The game is seeded into storage with a token the Worker never sees: every guess here is
// answered by a route, so none of them spends the local rate limit.
test("a refused or lost guess leaves the board unchanged and the input ready", async ({ page }) => {
  const game = {
    n: launchNumber(),
    token: "unsigned",
    rows: [],
    facts: [],
    done: false,
    won: false,
  };
  await page.addInitScript(([key, value]) => localStorage.setItem(key, value), [
    GAME_KEY,
    JSON.stringify(game),
  ] as const);
  await openToday(page);
  const board = page.locator("#guesses li, #latest li");

  for (const refusal of refusals) {
    let sent: unknown = null;
    await page.route("**/api/guess", (route) => {
      sent = route.request().postDataJSON();
      return route.fulfill({
        status: refusal.status,
        contentType: "application/json",
        body: JSON.stringify({ error: refusal.error }),
      });
    });
    await page.fill("#guess", "Pierre");
    await page.keyboard.press("Enter");
    await expect(page.locator("#notice")).toHaveText(refusal.notice);
    expect(sent).toEqual({ token: "unsigned", name: "Pierre" });
    await expect(page.locator("#guess")).toBeEnabled();
    await expect(page.locator("#guess")).toBeFocused();
    await expect(board).toHaveCount(0);
    await expect(page.locator("#remaining")).toHaveText("8 guesses left");
    await page.unroute("**/api/guess");
  }

  await page.route("**/api/guess", (route) => route.abort("connectionfailed"));
  await page.fill("#guess", "Pierre");
  await page.keyboard.press("Enter");
  await expect(page.locator("#notice")).toHaveText(
    "That guess did not get checked, no connection. Try again.",
  );
  await expect(page.locator("#guess")).toBeEnabled();
  await expect(board).toHaveCount(0);
});
