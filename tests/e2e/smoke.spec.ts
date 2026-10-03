import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { gotoLaunchDay } from "./helpers";

test("the front page loads, and a failed puzzle fetch offers Retry", async ({ page }, info) => {
  await page.route("**/api/puzzle*", (route) =>
    route.fulfill({ status: 500, contentType: "application/json", body: '{"error":"down"}' }),
  );
  await gotoLaunchDay(page);
  await expect(page).toHaveTitle("WHOM?");
  await expect(page.locator(".wordmark")).toHaveText("WHOM?");
  await expect(page.locator("#issue")).toHaveText(/^No\. \d+ · \d{1,2} \w{3} \d{4}$/);
  await expect(page.locator("#notice")).toContainText("No face right now.");
  await expect(page.locator("#guess-form")).toBeHidden();
  const retry = page.getByRole("button", { name: "Retry" });
  expect((await retry.boundingBox())?.height ?? 0).toBeGreaterThanOrEqual(44);

  let release = () => {};
  const held = new Promise<void>((resolve) => (release = resolve));
  await page.route("**/api/puzzle*", async (route) => {
    await held;
    await route.continue();
  });
  await retry.click();
  await expect(page.locator("#notice")).toHaveText("Loading today's face");
  release();
  await expect(page.locator("#notice")).toHaveText("");
  await expect(page.locator("#guess-form")).toBeVisible();
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

test("security headers are set and the CSP admits the inline stylesheet", async ({ page }) => {
  const violations: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") violations.push(msg.text());
  });
  const res = await page.goto("/");
  const headers = res?.headers() ?? {};
  expect(headers["content-security-policy"] ?? "").toContain("default-src 'self'");
  expect(headers["x-content-type-options"]).toBe("nosniff");
  await expect(page.locator("link[rel=stylesheet]")).toHaveCount(0);
  const css = (await page.locator("head style").textContent()) ?? "";
  const hash = createHash("sha256").update(css).digest("base64");
  expect(headers["content-security-policy"]).toContain(`style-src 'self' 'sha256-${hash}'`);
  await expect(page.locator(".wordmark")).toHaveCSS("font-weight", "700");
  expect(violations.filter((v) => v.includes("Content Security Policy"))).toEqual([]);
});
