import type { Schedule } from "./pins.ts";
import type { Puzzle } from "./puzzle.ts";

export interface Env {
  ASSETS: Fetcher;
  FILES: R2Bucket;
  PUZZLES: DurableObjectNamespace<Puzzle>;
  SCHEDULE: DurableObjectNamespace<Schedule>;
  GUESS_RATE: RateLimit;
  SESSION_SECRET: string;
  PUZZLE_SEED: string;
}
