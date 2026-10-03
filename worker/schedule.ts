import type { Person } from "../shared/data.ts";
import { hmacKey } from "./token.ts";

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
        const hex = Array.from(mac, (b) => b.toString(16).padStart(2, "0")).join("");
        return { hex, person };
      }),
  );
  keyed.sort((a, b) => (a.hex < b.hex ? -1 : a.hex > b.hex ? 1 : 0));
  return keyed.map((k) => k.person);
}

export function personForNumber(order: Person[], n: number): Person {
  if (order.length === 0) throw new Error("empty schedule");
  const i = (((n - 1) % order.length) + order.length) % order.length;
  return order[i] as Person;
}
