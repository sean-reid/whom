import { expect, test, type APIRequestContext, type APIResponse } from "@playwright/test";
import { MAX_GUESSES } from "../../shared/api";
import type { GuessResponse, PuzzleResponse, StatsResponse } from "../../shared/api";
import type { NamesFile, Person, PoolFile } from "../../shared/data";
import { DAY_MS, latestAllowedNumber, puzzleNumber } from "../../shared/day";
import { normalizeName } from "../../shared/names";
import { personForNumber, scheduleOrder } from "../../worker/schedule";
import { signToken } from "../../worker/token";
import names from "../fixtures/names.json" with { type: "json" };
import pool from "../fixtures/pool.json" with { type: "json" };
import { E2E_SEED, flipSignature } from "./helpers";

// Today in UTC, or tomorrow when today sits before the epoch.
function playableDate(): string {
  const today = new Date().toISOString().slice(0, 10);
  if ((puzzleNumber(today) ?? 0) >= 1) return today;
  return new Date(Date.now() + DAY_MS).toISOString().slice(0, 10);
}

const date = playableDate();
const n = puzzleNumber(date) ?? 0;
const misses = ["pierre", "maria", "john", "carlos", "anna", "nicolas", "giovanni", "helen"];
const ERA = /^(same era|an older name|a newer name|era unknown)$/;
const ROOT =
  /^(a form of the same name|your guess is a short form of the answer|the answer is a short form of your guess)$/;

// A fixture name the grader links to the answer through sameAs or shortOf, if the day has one.
function rootPartner(answer: string): string | undefined {
  const records = (names as NamesFile).names;
  const linked = (from: string, to: string) => {
    const rec = records[from];
    return rec?.sameAs?.includes(to) || rec?.shortOf?.includes(to);
  };
  return Object.keys(records).find(
    (key) => key !== answer && (linked(key, answer) || linked(answer, key)),
  );
}

// The answer the Worker pins for n, since nothing earlier is pinned in a fresh state.
let answer: Person;
test.beforeAll(async () => {
  answer = personForNumber(await scheduleOrder((pool as PoolFile).people, E2E_SEED), n);
});

interface Client {
  get(path: string): Promise<APIResponse>;
  post(path: string, data: unknown): Promise<APIResponse>;
}

const octet = () => Math.floor(Math.random() * 254) + 1;

// A game plays from its own address, as Cloudflare labels visitors, so the per-client
// count and the rate limit see each game on its own.
function client(request: APIRequestContext, ip = `10.${octet()}.${octet()}.${octet()}`): Client {
  const headers = { "cf-connecting-ip": ip };
  return {
    get: (path) => request.get(path, { headers }),
    post: (path, data) => request.post(path, { data, headers }),
  };
}

async function newGame(from: Client): Promise<PuzzleResponse> {
  const res = await from.get(`/api/puzzle?date=${date}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as PuzzleResponse;
}

async function guess(from: Client, token: string, name: string) {
  const res = await from.post("/api/guess", { token, name });
  return { status: res.status(), body: (await res.json()) as GuessResponse & { error?: string } };
}

async function stats(from: Client): Promise<StatsResponse> {
  const res = await from.get(`/api/stats/${n}`);
  expect(res.status()).toBe(200);
  return (await res.json()) as StatsResponse;
}

test("today's puzzle returns a token with eight guesses left", async ({ request }) => {
  const body = await newGame(client(request));
  expect(body.n).toBe(n);
  expect(body.guessesLeft).toBe(MAX_GUESSES);
  expect(body.token).toMatch(/^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/);
  expect(body.names).toMatch(/^[^"\s]+$/);
});

test("bad and out of window dates are 400", async ({ request }) => {
  expect((await request.get("/api/puzzle?date=2026-02-30")).status()).toBe(400);
  expect((await request.get("/api/puzzle?date=2030-01-01")).status()).toBe(400);
  expect((await request.get("/api/puzzle")).status()).toBe(400);
});

test("an unknown name is 422 and a tampered token is 400", async ({ request }) => {
  const me = client(request);
  const { token } = await newGame(me);
  const unknown = await guess(me, token, "Zebedee");
  expect(unknown.status).toBe(422);
  expect(unknown.body.error).toBe("unknown name");
  const tampered = await guess(me, flipSignature(token), "Pierre");
  expect(tampered.status).toBe(400);
  expect(tampered.body.error).toBe("bad token");
});

test("a lost game grades every miss, drips facts, and counts once per client", async ({
  request,
}) => {
  const me = client(request);
  const before = await stats(me);
  let { token } = await newGame(me);
  let penultimate = token;
  let last: GuessResponse | undefined;
  for (const [i, name] of misses.entries()) {
    penultimate = token;
    const { status, body } = await guess(me, token, name);
    expect(status, `guess ${i + 1}`).toBe(200);
    expect(body.phrases).toHaveLength(6);
    expect(body.phrases[5]?.text).toMatch(ERA);
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
  expect(last?.reveal?.label).toBe(answer.label);
  expect(JSON.stringify({ ...last, token: "" })).not.toMatch(/Q\d+/);

  const repeat = await guess(me, penultimate, "Ellen");
  expect(repeat.status).toBe(409);
  expect(repeat.body.error).toBe("game over");
  const over = await guess(me, token, "Ellen");
  expect(over.status).toBe(409);

  const after = await stats(me);
  expect(after.total).toBe(before.total + 1);
  expect(after.counts[MAX_GUESSES]).toBe((before.counts[MAX_GUESSES] ?? 0) + 1);

  const second = await newGame(me);
  const win = await guess(me, second.token, answer.display);
  expect(win.status).toBe(200);
  expect(win.body.won).toBe(true);
  expect(win.body.reveal?.label).toBe(answer.label);
  expect((await stats(me)).counts).toEqual(after.counts);
});

test("a repeated guess is 422 and the right name wins with the same reveal", async ({
  request,
}) => {
  const me = client(request);
  const { token } = await newGame(me);
  const first = await guess(me, token, "Pierre");
  const again = await guess(me, first.body.token, "pierre");
  expect(again.status).toBe(422);
  expect(again.body.error).toBe("already guessed");
  let token2 = first.body.token;
  let made = 1;
  const partner = rootPartner(normalizeName(answer.display));
  if (partner) {
    const related = await guess(me, token2, partner);
    expect(related.status).toBe(200);
    expect(related.body.won).toBe(false);
    expect(related.body.phrases).toHaveLength(7);
    expect(related.body.phrases[6]).toEqual({ text: expect.stringMatching(ROOT), exact: true });
    token2 = related.body.token;
    made += 1;
  }
  const win = await guess(me, token2, answer.display.toUpperCase());
  made += 1;
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
  const after = await stats(me);
  expect(after.counts[made - 1]).toBeGreaterThanOrEqual(1);
});

test("a token older than three days is refused", async ({ request }) => {
  const stale = await signToken(
    { n, nonce: "0".repeat(32), guesses: [], done: false, issued: Date.now() - 4 * DAY_MS },
    "e2e-session",
  );
  const res = await guess(client(request), stale, "Pierre");
  expect(res.status).toBe(400);
  expect(res.body.error).toBe("bad token");
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

test("names lists every display form, most common first", async ({ request }) => {
  const res = await request.get("/api/names?v=test");
  expect(res.status()).toBe(200);
  expect(res.headers()["cache-control"]).toBe("public, max-age=86400");
  const { names: list } = (await res.json()) as { names: string[] };
  expect(list).toContain("Antonín");
  expect(list).toContain("Alan");
  const records = (names as NamesFile).names;
  const counts = list.map((d) => Object.values(records).find((r) => r.display === d)?.count ?? 0);
  expect(counts).toEqual([...counts].sort((a, b) => b - a));
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
