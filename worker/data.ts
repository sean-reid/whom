import type { NamesFile, Person, PoolFile } from "../shared/data.ts";
import type { Env } from "./env.ts";
import { scheduleOrder } from "./schedule.ts";

export interface Loaded {
  pool: PoolFile;
  names: NamesFile;
  byQid: Map<string, Person>;
  order: Person[];
  displays: string[];
  loadedAt: number;
}

const TTL_MS = 3_600_000;
let cached: { promise: Promise<Loaded>; at: number } | null = null;

async function readJson<T>(bucket: R2Bucket, key: string): Promise<T> {
  const obj = await bucket.get(key);
  if (!obj) throw new Error(`${key} is missing from R2`);
  return (await obj.json()) as T;
}

async function load(env: Env): Promise<Loaded> {
  const [pool, names] = await Promise.all([
    readJson<PoolFile>(env.FILES, "pool.json"),
    readJson<NamesFile>(env.FILES, "names.json"),
  ]);
  const collate = new Intl.Collator("en").compare;
  return {
    pool,
    names,
    byQid: new Map(pool.people.map((p) => [p.qid, p])),
    order: await scheduleOrder(pool.people, env.PUZZLE_SEED),
    displays: Object.values(names.names)
      .map((r) => r.display)
      .sort(collate),
    loadedAt: Date.now(),
  };
}

export function loadData(env: Env): Promise<Loaded> {
  const now = Date.now();
  if (!cached || now - cached.at > TTL_MS) {
    const promise = load(env);
    cached = { promise, at: now };
    promise.catch(() => {
      cached = null;
    });
  }
  return cached.promise;
}
