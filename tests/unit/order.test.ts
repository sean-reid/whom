import { describe, expect, it } from "vitest";
import { orderRows } from "../../src/render";

const row = (name: string, exact: number) => ({
  name,
  phrases: Array.from({ length: 6 }, (_, i) => ({ text: String(i), exact: i < exact })),
});

describe("orderRows", () => {
  it("puts the most matching guess first", () => {
    const rows = [row("Pierre", 1), row("Alec", 4), row("Maria", 0)];
    expect(orderRows(rows).map((r) => r.name)).toEqual(["Alec", "Pierre", "Maria"]);
  });
  it("breaks ties by recency and leaves the input untouched", () => {
    const rows = [row("Ann", 2), row("Jean", 2), row("Carlos", 2)];
    expect(orderRows(rows).map((r) => r.name)).toEqual(["Carlos", "Jean", "Ann"]);
    expect(rows.map((r) => r.name)).toEqual(["Ann", "Jean", "Carlos"]);
  });
});
