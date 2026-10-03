import { DurableObject } from "cloudflare:workers";
import type { Env } from "./env.ts";
import { qidForNumber } from "./schedule.ts";

// One instance holds every day's pin so a new day can skip faces earlier days served.
export class Schedule extends DurableObject<Env> {
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    ctx.storage.sql.exec(
      "CREATE TABLE IF NOT EXISTS pins (n INTEGER PRIMARY KEY, qid TEXT NOT NULL)",
    );
  }

  pin(n: number, qids: string[]): string {
    const sql = this.ctx.storage.sql;
    const row = sql.exec<{ qid: string }>("SELECT qid FROM pins WHERE n = ?", n).toArray()[0];
    if (row) return row.qid;
    const taken = new Set(
      sql
        .exec<{ qid: string }>("SELECT qid FROM pins")
        .toArray()
        .map((r) => r.qid),
    );
    const qid = qidForNumber(qids, n, taken);
    sql.exec("INSERT INTO pins (n, qid) VALUES (?, ?)", n, qid);
    return qid;
  }
}
