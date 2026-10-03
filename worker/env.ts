import type { Puzzle } from "./puzzle.ts";

export interface Env {
  ASSETS: Fetcher;
  FILES: R2Bucket;
  PUZZLES: DurableObjectNamespace<Puzzle>;
  GUESS_RATE: RateLimit;
  SESSION_SECRET: string;
  PUZZLE_SEED: string;
}
