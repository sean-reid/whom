import { describe, expect, it } from "vitest";
import { Suggestions } from "../../src/suggest";

const names = ["Alan", "Alec", "Antonín", "José", "Josef", "Mary Ann", "Jean-Paul", "William"];

describe("Suggestions", () => {
  it("prefers prefix matches and ignores accents and case", () => {
    const s = new Suggestions(names);
    expect(s.match("al")).toEqual(["Alan", "Alec"]);
    expect(s.match("jose")).toEqual(["José", "Josef"]);
    expect(s.match("ANT")).toEqual(["Antonín"]);
  });
  it("falls back to substring matches after prefixes", () => {
    const s = new Suggestions(names);
    expect(s.match("ann")).toEqual(["Mary Ann"]);
    expect(s.match("paul")).toEqual(["Jean-Paul"]);
  });
  it("returns nothing for an empty query and caps the list", () => {
    const s = new Suggestions(names);
    expect(s.match("   ")).toEqual([]);
    expect(s.match("a", 2)).toHaveLength(2);
  });
});
