import { MAX_GUESSES } from "../shared/api.ts";
import type {
  GuessResponse,
  HealthResponse,
  NamesResponse,
  PuzzleResponse,
  StatsResponse,
} from "../shared/api.ts";
import type { Person } from "../shared/data.ts";
import { EPOCH, dateWithinWindow, latestAllowedNumber, puzzleNumber } from "../shared/day.ts";
import { answerRecord, facts, fallbackRecord, hint, isWin, phrases } from "../shared/grade.ts";
import { normalizeName } from "../shared/names.ts";
import { findPerson, loadData, type Loaded } from "./data.ts";
import type { Env } from "./env.ts";
import { hex, newNonce, signToken, verifyToken } from "./token.ts";

export { Schedule } from "./pins.ts";
export { Puzzle } from "./puzzle.ts";

const encoder = new TextEncoder();
const DAY = "public, max-age=86400";
const THREE_DAYS_MS = 3 * 86_400_000;
const MAX_BODY_BYTES = 4096;
const MAX_NAME_LENGTH = 64;

// API responses never render as documents, so they carry the strictest policy.
const apiHeaders = {
  "x-content-type-options": "nosniff",
  "content-security-policy": "default-src 'none'; frame-ancestors 'none'",
};

const json = (body: unknown, status = 200, cache = "no-store"): Response =>
  new Response(JSON.stringify(body), {
    status,
    headers: {
      ...apiHeaders,
      "content-type": "application/json; charset=utf-8",
      "cache-control": cache,
    },
  });

const error = (message: string, status: number): Response => json({ error: message }, status);

const puzzleStub = (env: Env, n: number) => env.PUZZLES.get(env.PUZZLES.idFromName(String(n)));
const scheduleStub = (env: Env) => env.SCHEDULE.get(env.SCHEDULE.idFromName("schedule"));

// A pin never changes once made, so each isolate asks the Durable Object once per puzzle.
const pins = new Map<number, string>();
const unrecorded = new Set<string>();

async function answerFor(
  env: Env,
  ctx: ExecutionContext,
  n: number,
): Promise<{ answer: Person; data: Loaded }> {
  let qid = pins.get(n);
  if (qid === undefined) {
    const order = (await loadData(env, ctx)).order;
    qid = await scheduleStub(env).pin(
      n,
      order.map((p) => p.qid),
    );
    pins.set(n, qid);
  }
  const found = await findPerson(env, ctx, qid);
  if (!found) throw new Error(`pinned ${qid} is missing from the pool`);
  return { answer: found.person, data: found.data };
}

const clientIp = (request: Request): string => request.headers.get("cf-connecting-ip") ?? "unknown";

async function throttled(request: Request, env: Env): Promise<boolean> {
  const { success } = await env.GUESS_RATE.limit({ key: clientIp(request) });
  return !success;
}

// The object stores only this digest, never the address.
async function clientKey(request: Request, n: number): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(`${clientIp(request)}/${n}`));
  return hex(new Uint8Array(digest));
}

function parseNumber(segment: string, nowMs: number): number | null {
  if (!/^\d+$/.test(segment)) return null;
  const n = Number(segment);
  return n >= 1 && n <= latestAllowedNumber(nowMs) ? n : null;
}

async function cached(
  request: Request,
  ctx: ExecutionContext,
  build: () => Promise<Response>,
): Promise<Response> {
  const cache = caches.default;
  const hit = await cache.match(request);
  if (hit) return hit;
  const res = await build();
  if (res.ok) ctx.waitUntil(cache.put(request, res.clone()));
  return res;
}

async function getPuzzle(
  request: Request,
  env: Env,
  ctx: ExecutionContext,
  url: URL,
): Promise<Response> {
  if (await throttled(request, env)) return error("slow down", 429);
  const date = url.searchParams.get("date") ?? "";
  const n = puzzleNumber(date);
  if (n === null || n < 1 || !dateWithinWindow(date, Date.now())) return error("bad date", 400);
  await answerFor(env, ctx, n);
  const token = await signToken(
    { n, nonce: newNonce(), guesses: [], done: false, issued: Date.now() },
    env.SESSION_SECRET,
  );
  const body: PuzzleResponse = { n, token, guessesLeft: MAX_GUESSES };
  return json(body);
}

async function postGuess(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
  if (await throttled(request, env)) return error("slow down", 429);
  if (Number(request.headers.get("content-length") ?? 0) > MAX_BODY_BYTES) {
    return error("body too large", 413);
  }
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return error("bad json", 400);
  }
  const { token, name } = (body ?? {}) as Record<string, unknown>;
  if (typeof token !== "string" || typeof name !== "string") return error("bad request", 400);
  if (name.length > MAX_NAME_LENGTH) return error("unknown name", 422);
  const game = await verifyToken(token, env.SESSION_SECRET);
  if (!game || Date.now() - game.issued > THREE_DAYS_MS) return error("bad token", 400);
  if (game.done) return error("game over", 409);

  const { answer, data } = await answerFor(env, ctx, game.n);
  const guess = normalizeName(name);
  const guessRec = data.names.names[guess];
  if (!guessRec) return error("unknown name", 422);
  if (game.guesses.includes(guess)) return error("already guessed", 422);
  let answerRec = answerRecord(answer, data.names);
  if (!answerRec) {
    if (!unrecorded.has(answer.qid)) {
      unrecorded.add(answer.qid);
      console.warn(`${answer.qid} has no record in names.json; grading against an empty one`);
    }
    answerRec = fallbackRecord(answer);
  }

  const guesses = [...game.guesses, guess];
  const won = isWin(guess, answer);
  const done = won || guesses.length >= MAX_GUESSES;
  const puzzle = puzzleStub(env, game.n);
  const bucket = won ? guesses.length - 1 : MAX_GUESSES;
  const replayed = done
    ? !(await puzzle.record(game.nonce, await clientKey(request, game.n), bucket))
    : await puzzle.recorded(game.nonce);
  if (replayed) return error("game over", 409);
  const next = await signToken({ ...game, guesses, done }, env.SESSION_SECRET);
  const answerName = normalizeName(answer.display);
  const languages = data.names.languages;
  const res: GuessResponse = {
    token: next,
    name: guessRec.display,
    phrases: won
      ? phrases(guess, guessRec, guess, guessRec, languages)
      : phrases(guess, guessRec, answerName, answerRec, languages),
    facts: facts(answer, guesses.length),
    guessesLeft: MAX_GUESSES - guesses.length,
    done,
    won,
  };
  const tip = hint(answerRec, guesses.length, languages);
  if (tip !== undefined) res.hint = tip;
  if (done) {
    res.reveal = {
      label: answer.label,
      display: answer.display,
      description: answer.description,
      wiki: answer.wiki,
      image: answer.image,
    };
  }
  return json(res);
}

async function getCrop(env: Env, ctx: ExecutionContext, segment: string): Promise<Response> {
  const n = parseNumber(segment, Date.now());
  if (n === null) return error("not found", 404);
  const { answer } = await answerFor(env, ctx, n);
  const obj = await env.FILES.get(answer.crop);
  if (!obj) return error("not found", 404);
  return new Response(obj.body, {
    headers: {
      ...apiHeaders,
      "content-type": "image/jpeg",
      "cache-control": DAY,
      etag: obj.httpEtag,
    },
  });
}

async function getStats(request: Request, env: Env, segment: string): Promise<Response> {
  if (await throttled(request, env)) return error("slow down", 429);
  const n = parseNumber(segment, Date.now());
  if (n === null) return error("not found", 404);
  const counts = await puzzleStub(env, n).stats();
  const body: StatsResponse = { n, counts, total: counts.reduce((a, b) => a + b, 0) };
  return json(body);
}

async function getNames(env: Env, ctx: ExecutionContext): Promise<Response> {
  const body: NamesResponse = { names: (await loadData(env, ctx)).displays };
  return json(body, 200, DAY);
}

async function route(
  request: Request,
  env: Env,
  ctx: ExecutionContext,
  url: URL,
): Promise<Response> {
  const { pathname } = url;
  const method = request.method;
  if (pathname === "/api/health") {
    const body: HealthResponse = { ok: true, epoch: EPOCH };
    return json(body);
  }
  if (pathname === "/api/puzzle" && method === "GET") return getPuzzle(request, env, ctx, url);
  if (pathname === "/api/guess" && method === "POST") return postGuess(request, env, ctx);
  if (pathname === "/api/names" && method === "GET") {
    return cached(request, ctx, () => getNames(env, ctx));
  }
  const crop = /^\/api\/crop\/([^/]+)$/.exec(pathname);
  if (crop && method === "GET") {
    return cached(request, ctx, () => getCrop(env, ctx, crop[1] ?? ""));
  }
  const stats = /^\/api\/stats\/([^/]+)$/.exec(pathname);
  if (stats && method === "GET") return getStats(request, env, stats[1] ?? "");
  return error("not found", 404);
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
    try {
      return await route(request, env, ctx, url);
    } catch (err) {
      console.error(err);
      return error("internal error", 500);
    }
  },
} satisfies ExportedHandler<Env>;
