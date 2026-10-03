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
import { answerRecord, facts, hint, isWin, phrases } from "../shared/grade.ts";
import { normalizeName } from "../shared/names.ts";
import { loadData, type Loaded } from "./data.ts";
import type { Env } from "./env.ts";
import { newNonce, signToken, verifyToken } from "./token.ts";
import { personForNumber } from "./schedule.ts";

export { Puzzle } from "./puzzle.ts";

const DAY = "public, max-age=86400";

const json = (body: unknown, status = 200, cache = "no-store"): Response =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", "cache-control": cache },
  });

const error = (message: string, status: number): Response => json({ error: message }, status);

const puzzleStub = (env: Env, n: number) => env.PUZZLES.get(env.PUZZLES.idFromName(String(n)));

async function answerFor(env: Env, data: Loaded, n: number): Promise<Person> {
  const candidate = personForNumber(data.order, n);
  const qid = await puzzleStub(env, n).pin(candidate.qid);
  const pinned = data.byQid.get(qid);
  if (!pinned) throw new Error(`pinned ${qid} is missing from the pool`);
  return pinned;
}

async function throttled(request: Request, env: Env): Promise<boolean> {
  const key = request.headers.get("cf-connecting-ip") ?? "unknown";
  const { success } = await env.GUESS_RATE.limit({ key });
  return !success;
}

function parseNumber(segment: string, nowMs: number): number | null {
  if (!/^\d+$/.test(segment)) return null;
  const n = Number(segment);
  return n >= 1 && n <= latestAllowedNumber(nowMs) ? n : null;
}

async function getPuzzle(request: Request, env: Env, url: URL): Promise<Response> {
  if (await throttled(request, env)) return error("slow down", 429);
  const date = url.searchParams.get("date") ?? "";
  const n = puzzleNumber(date);
  if (n === null || n < 1 || !dateWithinWindow(date, Date.now())) return error("bad date", 400);
  await answerFor(env, await loadData(env), n);
  const token = await signToken(
    { n, nonce: newNonce(), guesses: [], done: false, issued: Date.now() },
    env.SESSION_SECRET,
  );
  const body: PuzzleResponse = { n, token, guessesLeft: MAX_GUESSES };
  return json(body);
}

async function postGuess(request: Request, env: Env): Promise<Response> {
  if (await throttled(request, env)) return error("slow down", 429);
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return error("bad json", 400);
  }
  const { token, name } = (body ?? {}) as Record<string, unknown>;
  if (typeof token !== "string" || typeof name !== "string") return error("bad request", 400);
  const game = await verifyToken(token, env.SESSION_SECRET);
  if (!game) return error("bad token", 400);
  if (game.done) return error("game over", 409);

  const data = await loadData(env);
  const answer = await answerFor(env, data, game.n);
  const guess = normalizeName(name);
  const guessRec = data.names.names[guess];
  if (!guessRec) return error("unknown name", 422);
  if (game.guesses.includes(guess)) return error("already guessed", 422);
  const answerRec = answerRecord(answer, data.names);
  if (!answerRec) return error("answer has no name record", 500);

  const guesses = [...game.guesses, guess];
  const won = isWin(guess, answer);
  const done = won || guesses.length >= MAX_GUESSES;
  const next = await signToken({ ...game, guesses, done }, env.SESSION_SECRET);
  if (done) {
    await puzzleStub(env, game.n).record(game.nonce, won ? guesses.length - 1 : MAX_GUESSES);
  }
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

async function getCrop(env: Env, segment: string): Promise<Response> {
  const n = parseNumber(segment, Date.now());
  if (n === null) return error("not found", 404);
  const answer = await answerFor(env, await loadData(env), n);
  const obj = await env.FILES.get(answer.crop);
  if (!obj) return error("not found", 404);
  return new Response(obj.body, {
    headers: { "content-type": "image/jpeg", "cache-control": DAY, etag: obj.httpEtag },
  });
}

async function getStats(env: Env, segment: string): Promise<Response> {
  const n = parseNumber(segment, Date.now());
  if (n === null) return error("not found", 404);
  const counts = await puzzleStub(env, n).stats();
  const body: StatsResponse = { n, counts, total: counts.reduce((a, b) => a + b, 0) };
  return json(body);
}

async function getNames(env: Env): Promise<Response> {
  const body: NamesResponse = { names: (await loadData(env)).displays };
  return json(body, 200, DAY);
}

async function route(request: Request, env: Env, url: URL): Promise<Response> {
  const { pathname } = url;
  const method = request.method;
  if (pathname === "/api/health") {
    const body: HealthResponse = { ok: true, epoch: EPOCH };
    return json(body);
  }
  if (pathname === "/api/puzzle" && method === "GET") return getPuzzle(request, env, url);
  if (pathname === "/api/guess" && method === "POST") return postGuess(request, env);
  if (pathname === "/api/names" && method === "GET") return getNames(env);
  const crop = /^\/api\/crop\/([^/]+)$/.exec(pathname);
  if (crop && method === "GET") return getCrop(env, crop[1] ?? "");
  const stats = /^\/api\/stats\/([^/]+)$/.exec(pathname);
  if (stats && method === "GET") return getStats(env, stats[1] ?? "");
  return error("not found", 404);
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) return env.ASSETS.fetch(request);
    try {
      return await route(request, env, url);
    } catch (err) {
      console.error(err);
      return error("internal error", 500);
    }
  },
} satisfies ExportedHandler<Env>;
