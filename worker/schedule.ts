import type { Person } from "../shared/data.ts";
import { hex, hmacKey } from "./token.ts";

const encoder = new TextEncoder();

export async function scheduleOrder(people: Person[], seed: string): Promise<Person[]> {
  const key = await hmacKey(seed, ["sign"]);
  const keyed = await Promise.all(
    people
      .filter((p) => !p.retired)
      .map(async (person) => {
        const mac = new Uint8Array(
          await crypto.subtle.sign("HMAC", key, encoder.encode(person.qid)),
        );
        return { hex: hex(mac), person };
      }),
  );
  keyed.sort((a, b) => (a.hex < b.hex ? -1 : a.hex > b.hex ? 1 : 0));
  return keyed.map((k) => k.person);
}

export function qidForNumber(
  qids: string[],
  n: number,
  pinned: ReadonlySet<string> = new Set(),
): string {
  if (qids.length === 0) throw new Error("empty schedule");
  const start = (((n - 1) % qids.length) + qids.length) % qids.length;
  for (let i = 0; i < qids.length; i++) {
    const qid = qids[(start + i) % qids.length] as string;
    if (!pinned.has(qid)) return qid;
  }
  // Every face has been served, so the schedule laps and repeats in order.
  return qids[start] as string;
}

export function personForNumber(
  order: Person[],
  n: number,
  pinned: ReadonlySet<string> = new Set(),
): Person {
  const qid = qidForNumber(
    order.map((p) => p.qid),
    n,
    pinned,
  );
  return order.find((p) => p.qid === qid) as Person;
}
