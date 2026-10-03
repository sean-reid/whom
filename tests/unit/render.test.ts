import { describe, expect, it } from "vitest";
import { formatCountdown, resultText } from "../../src/render";

const reveal = {
  label: "Alan Turing",
  display: "Alan",
  description: null,
  wiki: null,
  image: { file: "", artist: "", licence: "Public domain", licenceUrl: "", pageUrl: "" },
};

describe("resultText", () => {
  it("names the person and the guess count", () => {
    expect(resultText(true, 1, reveal)).toBe("Alan Turing, first guess.");
    expect(resultText(true, 4, reveal)).toBe("Alan Turing, in 4.");
    expect(resultText(false, 8, reveal)).toBe("Not this time. It was Alan Turing.");
  });
});

describe("formatCountdown", () => {
  it("rounds up to the minute and drops zero hours", () => {
    expect(formatCountdown(5 * 3_600_000 + 12 * 60_000 + 1)).toBe("Next face in 5h 13m");
    expect(formatCountdown(30 * 60_000)).toBe("Next face in 30m");
  });
});
