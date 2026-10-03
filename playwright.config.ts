import { readdirSync } from "node:fs";
import { defineConfig, devices } from "@playwright/test";

const port = 8787;
const fixtures = [
  "pool.json",
  "names.json",
  ...readdirSync("tests/fixtures/crops").map((f) => `crops/${f}`),
];
const seed = fixtures
  .map((f) => `npx wrangler r2 object put whom/${f} --file tests/fixtures/${f} --local`)
  .join(" && ");
const dev = `npx wrangler dev --port ${port} --ip 127.0.0.1 --var SESSION_SECRET:e2e-session --var PUZZLE_SEED:e2e-seed`;
const api = /api\.spec\.ts$/;

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "api", testMatch: api },
    {
      name: "mobile",
      testIgnore: api,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 375, height: 740 },
        hasTouch: true,
        isMobile: true,
      },
    },
    {
      name: "tablet",
      testIgnore: api,
      use: { ...devices["Desktop Chrome"], viewport: { width: 768, height: 1024 }, hasTouch: true },
    },
    {
      name: "desktop",
      testIgnore: api,
      use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 800 } },
    },
  ],
  webServer: {
    command: `npm run build && ${seed} && ${dev}`,
    url: `http://127.0.0.1:${port}/`,
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
  },
});
