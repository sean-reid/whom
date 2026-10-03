import { execFileSync } from "node:child_process";
import { readdirSync } from "node:fs";

// Puts the test fixtures into a local R2 so wrangler dev has a pool to serve.
const state = process.argv[2] ?? ".wrangler/state";
const files = [
  "pool.json",
  "names.json",
  ...readdirSync("tests/fixtures/crops").map((f) => `crops/${f}`),
];
for (const f of files) {
  execFileSync(
    "npx",
    [
      "wrangler",
      "r2",
      "object",
      "put",
      `whom/${f}`,
      "--file",
      `tests/fixtures/${f}`,
      "--local",
      "--persist-to",
      state,
    ],
    { stdio: "inherit" },
  );
}
