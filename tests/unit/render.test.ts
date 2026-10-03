import { describe, expect, it } from "vitest";
import { announceRow, distSentence, formatCountdown, resultText } from "../../src/render";

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

describe("announceRow", () => {
  it("reads the phrases, the hint, and the guesses left as one sentence", () => {
    const row = {
      name: "Pierre",
      phrases: [
        { text: "shorter", exact: false },
        { text: "same first letter", exact: true },
      ],
    };
    expect(announceRow({ ...row, hint: "a form of the same name" }, 7)).toBe(
      "shorter, same first letter, a form of the same name. 7 guesses left",
    );
    expect(announceRow(row, 1)).toBe("shorter, same first letter. 1 guess left");
  });
});

describe("distSentence", () => {
  it("names the slot and both counts, or only yours before the stats arrive", () => {
    expect(distSentence(2, 1, 12)).toBe("Won in 3: 1 of your games, 12 of everyone's");
    expect(distSentence(8, 0, 4)).toBe("Lost: 0 of your games, 4 of everyone's");
    expect(distSentence(0, 2, null)).toBe("Won in 1: 2 of your games");
  });
});
