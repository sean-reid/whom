import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";

const BUDGET = 25 * 1024;
const dir = "dist/assets";
let total = 0;
for (const name of readdirSync(dir)) {
  if (!name.endsWith(".js")) continue;
  const size = gzipSync(readFileSync(join(dir, name))).length;
  total += size;
  console.log(`${name}\t${size} B gzipped`);
}
console.log(`total\t${total} B gzipped, budget ${BUDGET} B`);
if (total > BUDGET) {
  console.error(`first-load JavaScript exceeds the budget by ${total - BUDGET} B`);
  process.exit(1);
}
