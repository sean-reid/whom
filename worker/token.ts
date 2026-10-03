export interface GameToken {
  n: number;
  nonce: string;
  guesses: string[];
  done: boolean;
  issued: number;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function toBase64Url(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(s: string): Uint8Array<ArrayBuffer> | null {
  if (!/^[A-Za-z0-9_-]*$/.test(s)) return null;
  const b64 = s
    .replace(/-/g, "+")
    .replace(/_/g, "/")
    .padEnd(Math.ceil(s.length / 4) * 4, "=");
  try {
    const bin = atob(b64);
    const out = new Uint8Array(new ArrayBuffer(bin.length));
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  } catch {
    return null;
  }
}

export async function hmacKey(secret: string, usages: ("sign" | "verify")[]): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    usages,
  );
}

export const hex = (bytes: Uint8Array): string =>
  Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");

export function newNonce(): string {
  return hex(crypto.getRandomValues(new Uint8Array(16)));
}

export async function signToken(payload: GameToken, secret: string): Promise<string> {
  const body = encoder.encode(JSON.stringify(payload));
  const key = await hmacKey(secret, ["sign"]);
  const sig = new Uint8Array(await crypto.subtle.sign("HMAC", key, body));
  return `${toBase64Url(body)}.${toBase64Url(sig)}`;
}

function isGameToken(v: unknown): v is GameToken {
  if (typeof v !== "object" || v === null) return false;
  const t = v as Record<string, unknown>;
  return (
    Number.isInteger(t.n) &&
    typeof t.nonce === "string" &&
    /^[0-9a-f]{32}$/.test(t.nonce) &&
    Array.isArray(t.guesses) &&
    t.guesses.every((g) => typeof g === "string") &&
    typeof t.done === "boolean" &&
    typeof t.issued === "number"
  );
}

export async function verifyToken(token: string, secret: string): Promise<GameToken | null> {
  const parts = token.split(".");
  if (parts.length !== 2) return null;
  const body = fromBase64Url(parts[0] ?? "");
  const sig = fromBase64Url(parts[1] ?? "");
  if (!body || !sig || sig.length !== 32) return null;
  const key = await hmacKey(secret, ["verify"]);
  if (!(await crypto.subtle.verify("HMAC", key, sig, body))) return null;
  try {
    const parsed: unknown = JSON.parse(decoder.decode(body));
    return isGameToken(parsed) ? parsed : null;
  } catch {
    return null;
  }
}
