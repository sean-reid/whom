import { DurableObject } from "cloudflare:workers";
import type { Env } from "./env.ts";
import { choosePin } from "./schedule.ts";

// One instance holds every day's pin so a new day can skip faces earlier days served.
export class Schedule extends DurableObject<Env> {
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS pins (n INTEGER PRIMARY KEY, qid TEXT NOT NULL)",
    );
  }

  lookup(n: number): string | null {
    const row = this.ctx.storage.sql
      .exec<{ qid: string }>("SELECT qid FROM pins WHERE n = ?", n)
      .toArray()[0];
    return row ? row.qid : null;
  }

  pin(n: number, qids: string[], adopt: string | null = null): string {
    const stored = this.lookup(n);
    if (stored !== null) return stored;
    const sql = this.ctx.storage.sql;
    const taken = new Set(
      sql
        .exec<{ qid: string }>("SELECT qid FROM pins")
        .toArray()
        .map((r) => r.qid),
    );
    const qid = choosePin(n, qids, taken, adopt);
    sql.exec("INSERT INTO pins (n, qid) VALUES (?, ?)", n, qid);
    return qid;
  }
}
