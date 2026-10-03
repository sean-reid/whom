import { execSync } from "node:child_process";
import { readdirSync } from "node:fs";

// Builds once and seeds a local R2 for each wrangler dev instance the suite starts.
export default function globalSetup() {
  const run = (cmd: string) => execSync(cmd, { stdio: "inherit" });
  run("npm run build");
  const fixtures = [
    "pool.json",
    "names.json",
    ...readdirSync("tests/fixtures/crops").map((f) => `crops/${f}`),
  ];
  for (const state of [".wrangler/state", ".wrangler/state-api"]) {
    for (const f of fixtures) {
      run(
        `npx wrangler r2 object put whom/${f} --file tests/fixtures/${f} --local --persist-to ${state}`,
      );
    }
  }
}
