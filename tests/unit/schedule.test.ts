import { describe, expect, it } from "vitest";
import type { Person } from "../../shared/data";
import { personForNumber, qidForNumber, scheduleOrder } from "../../worker/schedule";

const person = (qid: string, retired = false): Person => ({
  qid,
  label: qid,
  display: qid,
  names: [qid.toLowerCase()],
  born: 1950,
  citizenship: [],
  occupations: [],
  description: null,
  wiki: null,
  crop: `crops/${qid}.jpg`,
  image: { file: "", artist: "", licence: "", licenceUrl: "", pageUrl: "" },
  ...(retired ? { retired: true as const } : {}),
});

const people = ["Q1", "Q2", "Q3", "Q4", "Q5"].map((q) => person(q));

describe("scheduleOrder", () => {
  it("is stable for a seed and independent of input order", async () => {
    const a = await scheduleOrder(people, "seed");
    const b = await scheduleOrder([...people].reverse(), "seed");
    expect(a.map((p) => p.qid)).toEqual(b.map((p) => p.qid));
    expect(a.map((p) => p.qid).sort()).toEqual(["Q1", "Q2", "Q3", "Q4", "Q5"]);
  });
  it("changes with the seed", async () => {
    const seeds = ["a", "b", "c", "d", "e", "f"];
    const orders = await Promise.all(seeds.map((s) => scheduleOrder(people, s)));
    const distinct = new Set(orders.map((o) => o.map((p) => p.qid).join(",")));
    expect(distinct.size).toBeGreaterThan(1);
  });
  it("drops retired people", async () => {
    const order = await scheduleOrder([...people, person("Q6", true)], "seed");
    expect(order.map((p) => p.qid)).not.toContain("Q6");
    expect(order).toHaveLength(5);
  });
});

describe("personForNumber", () => {
  it("wraps around the pool", async () => {
    const order = await scheduleOrder(people, "seed");
    expect(personForNumber(order, 1)).toBe(order[0]);
    expect(personForNumber(order, 5)).toBe(order[4]);
    expect(personForNumber(order, 6)).toBe(order[0]);
    expect(personForNumber(order, 12)).toBe(order[1]);
  });
  it("refuses an empty schedule", () => {
    expect(() => personForNumber([], 1)).toThrow();
    expect(() => qidForNumber([], 1)).toThrow();
  });
  it("walks past qids pinned by other days and wraps", async () => {
    const order = await scheduleOrder(people, "seed");
    const q = order.map((p) => p.qid);
    expect(personForNumber(order, 2, new Set([q[1] ?? ""]))).toBe(order[2]);
    expect(qidForNumber(q, 3, new Set([q[2], q[3]].map(String)))).toBe(q[4]);
    expect(qidForNumber(q, 5, new Set([q[4], q[0]].map(String)))).toBe(q[1]);
  });
  it("repeats in order once every qid is pinned", async () => {
    const order = await scheduleOrder(people, "seed");
    const all = new Set(order.map((p) => p.qid));
    expect(personForNumber(order, 7, all)).toBe(order[1]);
  });
  it("never serves a pinned face again after the pool grows", async () => {
    const pinned = new Set<string>();
    const before = await scheduleOrder(people, "seed");
    for (let n = 1; n <= 3; n++) pinned.add(personForNumber(before, n, pinned).qid);
    const grown = [...people, ...["Q6", "Q7", "Q8", "Q9"].map((q) => person(q))];
    const after = await scheduleOrder(grown, "seed");
    for (let n = 4; n <= after.length; n++) {
      const pick = personForNumber(after, n, pinned);
      expect(pinned.has(pick.qid), `day ${n}`).toBe(false);
      pinned.add(pick.qid);
    }
    expect(pinned.size).toBe(after.length);
  });
});
