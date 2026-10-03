import { DurableObject } from "cloudflare:workers";
import type { Env } from "./env.ts";

const BUCKETS = 9;

export class Puzzle extends DurableObject<Env> {
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec(`
      CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS results (bucket INTEGER PRIMARY KEY, count INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS nonces (nonce TEXT PRIMARY KEY);
    `);
  }

  pin(qid: string): string {
    const row = this.ctx.storage.sql
      .exec<{ value: string }>("SELECT value FROM meta WHERE key = 'qid'")
      .toArray()[0];
    if (row) return row.value;
    this.ctx.storage.sql.exec("INSERT INTO meta (key, value) VALUES ('qid', ?)", qid);
    return qid;
  }

  record(nonce: string, bucket: number): boolean {
    if (!Number.isInteger(bucket) || bucket < 0 || bucket >= BUCKETS) return false;
    const inserted = this.ctx.storage.sql.exec(
      "INSERT OR IGNORE INTO nonces (nonce) VALUES (?)",
      nonce,
    ).rowsWritten;
    if (inserted === 0) return false;
    this.ctx.storage.sql.exec(
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
