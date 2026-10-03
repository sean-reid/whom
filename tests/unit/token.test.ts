import { describe, expect, it } from "vitest";
import { newNonce, signToken, verifyToken, type GameToken } from "../../worker/token";

const secret = "unit-secret";
const game: GameToken = {
  n: 12,
  nonce: newNonce(),
  guesses: ["alan"],
  done: false,
  issued: 1_700_000_000_000,
};

describe("token", () => {
  it("round trips", async () => {
    const token = await signToken(game, secret);
    expect(token).toMatch(/^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/);
    expect(await verifyToken(token, secret)).toEqual(game);
  });
  it("rejects a tampered payload", async () => {
    const token = await signToken(game, secret);
    const [, sig] = token.split(".");
    const forged = { ...game, guesses: [] };
    const body = Buffer.from(JSON.stringify(forged)).toString("base64url");
    expect(await verifyToken(`${body}.${sig}`, secret)).toBeNull();
  });
  it("rejects a tampered signature", async () => {
    const token = await signToken(game, secret);
    const [body, sig = ""] = token.split(".");
    const flipped = (sig[0] === "A" ? "B" : "A") + sig.slice(1);
    expect(await verifyToken(`${body}.${flipped}`, secret)).toBeNull();
    expect(await verifyToken(token, "other-secret")).toBeNull();
  });
  it("rejects bad shapes", async () => {
    expect(await verifyToken("", secret)).toBeNull();
    expect(await verifyToken("a.b.c", secret)).toBeNull();
    expect(await verifyToken("not base64!.x", secret)).toBeNull();
    const bad = {
      n: "12",
      nonce: "short",
      guesses: [],
      done: false,
      issued: 1,
    } as unknown as GameToken;
    const token = await signToken(bad, secret);
    expect(await verifyToken(token, secret)).toBeNull();
  });
  it("makes 32 hex character nonces that differ", () => {
    const a = newNonce();
    expect(a).toMatch(/^[0-9a-f]{32}$/);
    expect(newNonce()).not.toBe(a);
  });
});
