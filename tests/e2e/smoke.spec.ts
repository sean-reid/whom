import { expect, test } from "@playwright/test";
import { gotoLaunchDay } from "./helpers";

test("the front page loads with the wordmark and today's number", async ({ page }, info) => {
  await gotoLaunchDay(page);
  await expect(page).toHaveTitle("WHOM?");
  await expect(page.locator(".wordmark")).toHaveText("WHOM?");
  await expect(page.locator("#issue")).toHaveText(/^No\. \d+ · \d{1,2} \w{3} \d{4}$/);
  await page.screenshot({ path: info.outputPath("front.png"), fullPage: true });
});

test("health answers and unknown api routes answer 404 as json", async ({ request }) => {
  const health = await request.get("/api/health");
  expect(health.status()).toBe(200);
  expect(await health.json()).toMatchObject({ ok: true });
  const res = await request.get("/api/nothing");
  expect(res.status()).toBe(404);
  expect(await res.json()).toEqual({ error: "not found" });
});

test("security headers are set", async ({ request }) => {
  const res = await request.get("/");
  expect(res.headers()["content-security-policy"] ?? "").toContain("default-src 'self'");
  expect(res.headers()["x-content-type-options"]).toBe("nosniff");
});
