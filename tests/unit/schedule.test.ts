import { describe, expect, it } from "vitest";
import type { Person } from "../../shared/data";
import {
  choosePin,
  personForNumber,
  qidForNumber,
  resolvePin,
  scheduleOrder,
} from "../../worker/schedule";

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

// A Schedule stand-in with the object's rules: a stored day wins, an adopted qid is kept
// verbatim, anything else walks past the qids other days hold.
function fakeSchedule(rows: Record<number, string> = {}) {
  const calls: string[] = [];
  return {
    calls,
    rows,
    lookup: async (n: number) => {
      calls.push(`lookup ${n}`);
      return rows[n] ?? null;
    },
    pin: async (n: number, qids: string[], adopt: string | null) => {
      calls.push(`pin ${n} ${adopt ?? "walk"}`);
      rows[n] ??= choosePin(n, qids, new Set(Object.values(rows)), adopt);
      return rows[n];
    },
  };
}

const legacyOf = (qid: string | null, calls: string[]) => ({
  legacyPin: async () => {
    calls.push("legacy");
    return qid;
  },
});

describe("resolvePin", () => {
  const qids = ["Q1", "Q2", "Q3", "Q4", "Q5"];
  it("returns a stored pin without asking the Puzzle object", async () => {
    const schedule = fakeSchedule({ 1: "Q4" });
    expect(await resolvePin(schedule, legacyOf("Q9", schedule.calls), 1, qids)).toBe("Q4");
    expect(schedule.calls).toEqual(["lookup 1"]);
  });
  it("adopts the Puzzle object's old pin verbatim for a day Schedule has not seen", async () => {
    const schedule = fakeSchedule({ 2: "Q3" });
    expect(await resolvePin(schedule, legacyOf("Q3", schedule.calls), 1, qids)).toBe("Q3");
    expect(schedule.calls).toEqual(["lookup 1", "legacy", "pin 1 Q3"]);
    expect(schedule.rows).toEqual({ 1: "Q3", 2: "Q3" });
    expect(await resolvePin(schedule, legacyOf(null, schedule.calls), 1, qids)).toBe("Q3");
  });
  it("walks from the candidate when neither object has a pin", async () => {
    const schedule = fakeSchedule({ 1: "Q2" });
    expect(await resolvePin(schedule, legacyOf(null, schedule.calls), 2, qids)).toBe("Q3");
    expect(schedule.calls).toEqual(["lookup 2", "legacy", "pin 2 walk"]);
  });
  it("choosePin keeps an adopted qid even when the walk would skip it", () => {
    expect(choosePin(2, qids, new Set(["Q2"]), "Q2")).toBe("Q2");
    expect(choosePin(2, qids, new Set(["Q2"]), null)).toBe("Q3");
  });
});
