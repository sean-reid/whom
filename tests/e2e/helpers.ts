import type { Page } from "@playwright/test";
import { DAY_MS, EPOCH, localIsoDate, parseIsoDate, puzzleNumber } from "../../shared/day";

// The Worker under test runs with this PUZZLE_SEED, so a suite can work out today's answer.
export const E2E_SEED = "e2e-seed";

const epoch = parseIsoDate(EPOCH) ?? 0;
const beforeLaunch = () => Date.now() < epoch;

// The client reads its own clock, so before launch day the browser is moved to the first puzzle.
export function launchTime(): number {
  return beforeLaunch() ? epoch + DAY_MS / 2 : Date.now();
}

// The number the client will show on that clock, in this machine's time zone like the browser's.
export function launchNumber(): number {
  return puzzleNumber(localIsoDate(new Date(launchTime()))) ?? 0;
}

export async function gotoLaunchDay(page: Page): Promise<void> {
  if (beforeLaunch()) await page.clock.install({ time: new Date(launchTime()) });
  await page.goto("/");
}

// Changes one character of a token's signature so the Worker rejects it.
export function flipSignature(token: string): string {
  const [body, sig = ""] = token.split(".");
  return `${body}.${sig[0] === "A" ? "B" : "A"}${sig.slice(1)}`;
}
