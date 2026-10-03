import { describe, expect, it } from "vitest";
import { normalizeName } from "../../shared/names";

describe("normalizeName", () => {
  it("strips accents and lowercases", () => {
    expect(normalizeName("Antonín")).toBe("antonin");
    expect(normalizeName("José")).toBe("jose");
    expect(normalizeName("Seán")).toBe("sean");
  });
  it("collapses and trims whitespace", () => {
    expect(normalizeName("  Mary   Ann ")).toBe("mary ann");
  });
  it("keeps hyphens", () => {
    expect(normalizeName("Jean-Paul")).toBe("jean-paul");
  });
  it("folds compatibility forms", () => {
    expect(normalizeName("Ｗilliam")).toBe("william");
  });
});
