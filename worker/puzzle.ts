import { DurableObject } from "cloudflare:workers";
import { RESULT_SLOTS } from "../shared/api.ts";
import type { Env } from "./env.ts";

const BUCKETS = RESULT_SLOTS;

export class Puzzle extends DurableObject<Env> {
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS results (bucket INTEGER PRIMARY KEY, count INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS nonces (nonce TEXT PRIMARY KEY);
      CREATE TABLE IF NOT EXISTS keys (key TEXT PRIMARY KEY);
    `);
  }

  recorded(nonce: string): boolean {
    return (
      this.ctx.storage.sql.exec("SELECT 1 FROM nonces WHERE nonce = ?", nonce).toArray().length > 0
    );
  }

  // A nonce finishes once; the first finish per client key is the one that counts.
  record(nonce: string, key: string, bucket: number): boolean {
    if (!Number.isInteger(bucket) || bucket < 0 || bucket >= BUCKETS) {
      throw new RangeError(`bucket ${bucket} is outside 0..${BUCKETS - 1}`);
    }
    const sql = this.ctx.storage.sql;
    if (sql.exec("INSERT OR IGNORE INTO nonces (nonce) VALUES (?)", nonce).rowsWritten === 0) {
      return false;
    }
    if (sql.exec("INSERT OR IGNORE INTO keys (key) VALUES (?)", key).rowsWritten === 0) {
      return true;
    }
    sql.exec(
      "INSERT INTO results (bucket, count) VALUES (?, 1) ON CONFLICT (bucket) DO UPDATE SET count = count + 1",
      bucket,
    );
    return true;
  }

  stats(): number[] {
    const counts = new Array<number>(BUCKETS).fill(0);
    for (const row of this.ctx.storage.sql
      .exec<{ bucket: number; count: number }>("SELECT bucket, count FROM results")
      .toArray()) {
      counts[row.bucket] = row.count;
    }
    return counts;
  }
}
