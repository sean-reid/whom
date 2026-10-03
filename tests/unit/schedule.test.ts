import { describe, expect, it } from "vitest";
import type { Person } from "../../shared/data";
import { personForNumber, scheduleOrder } from "../../worker/schedule";

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
  });
});
