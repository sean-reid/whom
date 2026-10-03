import { expect, test, type APIRequestContext } from "@playwright/test";
import { MAX_GUESSES } from "../../shared/api";
import type { GuessResponse, PuzzleResponse, StatsResponse } from "../../shared/api";
import type { PoolFile } from "../../shared/data";
import { latestAllowedNumber, puzzleNumber } from "../../shared/day";
import { personForNumber, scheduleOrder } from "../../worker/schedule";
import pool from "../fixtures/pool.json" with { type: "json" };

// The seed matches the --var in playwright.config.ts, so the suite can work out today's answer.
const SEED = "e2e-seed";
const DAY_MS = 86_400_000;

// Today in UTC, or tomorrow when today sits before the epoch.
function playableDate(): string {
  const today = new Date().toISOString().slice(0, 10);
  if ((puzzleNumber(today) ?? 0) >= 1) return today;
  return new Date(Date.now() + DAY_MS).toISOString().slice(0, 10);
}

const date = playableDate();
const n = puzzleNumber(date) ?? 0;
const misses = ["pierre", "maria", "john", "carlos", "anna", "nicolas", "giovanni", "helen"];

async function newGame(request: APIRequestContext): Promise<PuzzleResponse> {
  const res = await request.get(`/api/puzzle?date=${date}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as PuzzleResponse;
}

async function guess(request: APIRequestContext, token: string, name: string) {
  const res = await request.post("/api/guess", { data: { token, name } });
  return { status: res.status(), body: (await res.json()) as GuessResponse & { error?: string } };
}

async function stats(request: APIRequestContext): Promise<StatsResponse> {
  const res = await request.get(`/api/stats/${n}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as StatsResponse;
}

test("today's puzzle returns a token with eight guesses left", async ({ request }) => {
  const body = await newGame(request);
  expect(body.n).toBe(n);
  expect(body.guessesLeft).toBe(MAX_GUESSES);
  expect(body.token).toMatch(/^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/);
});

test("bad and out of window dates are 400", async ({ request }) => {
  expect((await request.get("/api/puzzle?date=2026-02-30")).status()).toBe(400);
  expect((await request.get("/api/puzzle?date=2030-01-01")).status()).toBe(400);
  expect((await request.get("/api/puzzle")).status()).toBe(400);
});

test("an unknown name is 422 and a tampered token is 400", async ({ request }) => {
  const { token } = await newGame(request);
  const unknown = await guess(request, token, "Zebedee");
  expect(unknown.status).toBe(422);
  expect(unknown.body.error).toBe("unknown name");
  const [body, sig = ""] = token.split(".");
  const flipped = (sig[0] === "A" ? "B" : "A") + sig.slice(1);
  const tampered = await guess(request, `${body}.${flipped}`, "Pierre");
  expect(tampered.status).toBe(400);
  expect(tampered.body.error).toBe("bad token");
});

test("a lost game grades every miss, drips facts, and counts once", async ({ request }) => {
  const before = await stats(request);
  let { token } = await newGame(request);
  let penultimate = token;
  let last: GuessResponse | undefined;
  for (const [i, name] of misses.entries()) {
    penultimate = token;
    const { status, body } = await guess(request, token, name);
    expect(status, `guess ${i + 1}`).toBe(200);
    expect(body.phrases).toHaveLength(5);
    expect(body.name).toBe(name[0]?.toUpperCase() + name.slice(1));
    expect(body.guessesLeft).toBe(MAX_GUESSES - i - 1);
    const kinds = body.facts.map((f) => f.kind);
    if (i + 1 < 2) expect(kinds).toEqual([]);
    if (i + 1 >= 2) expect(kinds).toContain("born");
    if (i + 1 >= 4) expect(kinds).toContain("citizenship");
    if (i + 1 >= 6) expect(kinds).toContain("field");
    if (i + 1 < 5) expect(body.hint).toBeUndefined();
    else expect(body.hint).toMatch(/^the answer is used in /);
    token = body.token;
    last = body;
  }
  expect(last?.done).toBe(true);
  expect(last?.won).toBe(false);
  expect(last?.facts.find((f) => f.kind === "born")?.text).toMatch(/^Born in the \d{3}0s$/);
  expect(last?.reveal?.label).toBeTruthy();
  expect(JSON.stringify({ ...last, token: "" })).not.toMatch(/Q\d+/);

  const repeat = await guess(request, penultimate, "Ellen");
  expect(repeat.status).toBe(200);
  expect(repeat.body.done).toBe(true);
  const over = await guess(request, token, "Ellen");
  expect(over.status).toBe(409);

  const after = await stats(request);
  expect(after.total).toBe(before.total + 1);
  expect(after.counts[MAX_GUESSES]).toBe((before.counts[MAX_GUESSES] ?? 0) + 1);
});

test("a repeated guess is 422 and the right name wins with a reveal", async ({ request }) => {
  const order = await scheduleOrder((pool as PoolFile).people, SEED);
  const answer = personForNumber(order, n);
  const { token } = await newGame(request);
  const first = await guess(request, token, "Pierre");
  const again = await guess(request, first.body.token, "pierre");
  expect(again.status).toBe(422);
  expect(again.body.error).toBe("already guessed");
  const win = await guess(request, first.body.token, answer.display.toUpperCase());
  expect(win.status).toBe(200);
  expect(win.body.won).toBe(true);
  expect(win.body.done).toBe(true);
  expect(win.body.phrases.every((p) => p.exact)).toBe(true);
  expect(win.body.reveal).toEqual({
    label: answer.label,
    display: answer.display,
    description: answer.description,
    wiki: answer.wiki,
    image: answer.image,
  });
  const after = await stats(request);
  expect(after.counts[1]).toBeGreaterThanOrEqual(1);
});

test("the crop is a jpeg for live numbers and 404 otherwise", async ({ request }) => {
  const res = await request.get(`/api/crop/${n}`);
  expect(res.status()).toBe(200);
  expect(res.headers()["content-type"]).toBe("image/jpeg");
  expect(res.headers()["cache-control"]).toBe("public, max-age=86400");
  expect((await res.body()).subarray(0, 3)).toEqual(Buffer.from([0xff, 0xd8, 0xff]));
  expect((await request.get("/api/crop/0")).status()).toBe(404);
  expect((await request.get(`/api/crop/${latestAllowedNumber(Date.now()) + 1}`)).status()).toBe(
    404,
  );
  expect((await request.get("/api/crop/abc")).status()).toBe(404);
});

test("names lists every display form in order", async ({ request }) => {
  const res = await request.get("/api/names");
  expect(res.status()).toBe(200);
  expect(res.headers()["cache-control"]).toBe("public, max-age=86400");
  const { names } = (await res.json()) as { names: string[] };
  expect(names).toContain("Antonín");
  expect(names).toContain("Alan");
  expect(names).toEqual([...names].sort(new Intl.Collator("en").compare));
});

// wrangler dev enforces the ratelimits binding with a 60 second window, so this test runs last.
test("the rate limit answers 429 within a minute of traffic", async ({ request }) => {
  let limited = false;
  for (let i = 0; i < 61 && !limited; i++) {
    const res = await request.get(`/api/puzzle?date=${date}`);
    if (res.status() === 429) {
      expect(await res.json()).toEqual({ error: "slow down" });
      limited = true;
    }
  }
  expect(limited).toBe(true);
});
