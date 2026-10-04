import type { NamesFile, Person, PoolFile } from "../shared/data.ts";
import { scheduleOrder } from "./schedule.ts";

// The slice of Env the loader reads, typed by shape so a test can pass a plain object.
export interface DataEnv {
  FILES: { get(key: string): Promise<{ json(): Promise<unknown> } | null> };
  PUZZLE_SEED: string;
}

interface Waiter {
  waitUntil(promise: Promise<unknown>): void;
}

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

async function readJson<T>(bucket: DataEnv["FILES"], key: string): Promise<T> {
  const obj = await bucket.get(key);
  if (!obj) throw new Error(`${key} is missing from R2`);
  return (await obj.json()) as T;
}

// Suggestions come out in this order, so the names most people carry surface first.
export function displaysByPopularity(
  names: Record<string, { display: string; count: number }>,
): string[] {
  const collate = new Intl.Collator("en").compare;
  return Object.values(names)
    .sort((a, b) => b.count - a.count || collate(a.display, b.display))
    .map((r) => r.display);
}

async function load(env: DataEnv): Promise<Loaded> {
  const [pool, names] = await Promise.all([
    readJson<PoolFile>(env.FILES, "pool.json"),
    readJson<NamesFile>(env.FILES, "names.json"),
  ]);
  return {
    pool,
    names,
    byQid: new Map(pool.people.map((p) => [p.qid, p])),
    order: await scheduleOrder(pool.people, env.PUZZLE_SEED),
    displays: displaysByPopularity(names.names),
    loadedAt: Date.now(),
  };
}

export function resetData(): void {
  cached = null;
}

// After the TTL the current data keeps serving while one refresh runs; a failed refresh
// leaves the old data in place.
export function loadData(env: DataEnv, ctx?: Waiter): Promise<Loaded> {
  const now = Date.now();
  if (!cached) {
    const promise = load(env);
    cached = { promise, at: now };
    promise.catch(() => {
      cached = null;
    });
    return promise;
  }
  if (now - cached.at > TTL_MS) {
    const current = cached.promise;
    cached = { promise: current, at: now };
    const refresh = load(env).then(
      (loaded) => {
        cached = { promise: Promise.resolve(loaded), at: Date.now() };
      },
      () => undefined,
    );
    ctx?.waitUntil(refresh);
  }
  return cached.promise;
}

// A qid pinned by a fresher isolate can be missing from this one's hour-old pool.
export async function findPerson(
  env: DataEnv,
  ctx: Waiter | undefined,
  qid: string,
): Promise<{ data: Loaded; person: Person } | null> {
  let data = await loadData(env, ctx);
  let person = data.byQid.get(qid);
  if (!person) {
    resetData();
    data = await loadData(env, ctx);
    person = data.byQid.get(qid);
  }
  return person ? { data, person } : null;
}
