import { describe, expect, it } from "vitest";
import { shareLine } from "../../src/share";

describe("shareLine", () => {
  it("is one line with the number, the score, and the site", () => {
    expect(shareLine(12, true, 4)).toBe("WHOM? #12 4/8 whom.dwainosaur.com");
  });
  it("marks a loss with X", () => {
    expect(shareLine(12, false, 8)).toBe("WHOM? #12 X/8 whom.dwainosaur.com");
  });
});
