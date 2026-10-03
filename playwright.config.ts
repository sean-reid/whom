import { defineConfig, devices } from "@playwright/test";
import { E2E_SEED } from "./tests/e2e/helpers";

const port = 8787;
const apiPort = 8788;
const vars = `--var SESSION_SECRET:e2e-session --var PUZZLE_SEED:${E2E_SEED}`;
const states = [".wrangler/state", ".wrangler/state-api"];
const seed = states.map((state) => `node scripts/seed-local.mjs ${state}`).join(" && ");
const dev = (p: number, state: string) =>
  `npx wrangler dev --port ${p} --ip 127.0.0.1 ${vars} --persist-to ${state}`;
const api = /api\.spec\.ts$/;

// Servers start before any setup hook, so the first one builds and seeds both local R2
// stores. The API suite gets its own Worker so its rate-limit case never eats the budget
// the browser suites play against.
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
    { name: "api", testMatch: api, use: { baseURL: `http://127.0.0.1:${apiPort}` } },
  ],
  webServer: [
    {
      command: `npm run build && ${seed} && ${dev(port, states[0] ?? "")}`,
      url: `http://127.0.0.1:${port}/`,
      reuseExistingServer: !process.env.CI,
      timeout: 180_000,
    },
    {
      command: dev(apiPort, states[1] ?? ""),
      url: `http://127.0.0.1:${apiPort}/`,
      reuseExistingServer: !process.env.CI,
      timeout: 180_000,
    },
  ],
});
