import { describe, expect, it } from "vitest";
import {
  dateWithinWindow,
  latestAllowedNumber,
  localIsoDate,
  msUntilLocalMidnight,
  parseIsoDate,
  puzzleNumber,
} from "../../shared/day";

describe("parseIsoDate", () => {
  it("accepts a real calendar date", () => {
    expect(parseIsoDate("2026-09-29")).toBe(Date.UTC(2026, 8, 29));
  });
  it("rejects dates that do not round trip", () => {
    expect(parseIsoDate("2026-02-30")).toBeNull();
    expect(parseIsoDate("2026-13-01")).toBeNull();
    expect(parseIsoDate("29-09-2026")).toBeNull();
    expect(parseIsoDate("")).toBeNull();
  });
});

describe("puzzleNumber", () => {
  it("numbers the epoch day as 1", () => {
    expect(puzzleNumber("2026-09-29", "2026-09-29")).toBe(1);
  });
  it("counts calendar days across a DST change", () => {
    expect(puzzleNumber("2026-11-02", "2026-09-29")).toBe(35);
  });
  it("returns null for a bad date", () => {
    expect(puzzleNumber("nope", "2026-09-29")).toBeNull();
  });
});

describe("dateWithinWindow", () => {
  const noon = Date.UTC(2026, 8, 29, 12);
  it("allows yesterday, today, and tomorrow in UTC", () => {
    expect(dateWithinWindow("2026-09-28", noon)).toBe(true);
    expect(dateWithinWindow("2026-09-29", noon)).toBe(true);
    expect(dateWithinWindow("2026-09-30", noon)).toBe(true);
  });
  it("refuses anything further out", () => {
    expect(dateWithinWindow("2026-09-27", noon)).toBe(false);
    expect(dateWithinWindow("2026-10-01", noon)).toBe(false);
  });
});

describe("latestAllowedNumber", () => {
  it("is tomorrow's puzzle in UTC", () => {
    expect(latestAllowedNumber(Date.UTC(2026, 8, 29, 23), "2026-09-29")).toBe(2);
  });
});

describe("local time helpers", () => {
  it("formats the local date as ISO", () => {
    expect(localIsoDate(new Date(2026, 0, 5, 8))).toBe("2026-01-05");
  });
  it("measures the gap to local midnight", () => {
    const now = new Date(2026, 8, 29, 23, 0, 0);
    expect(msUntilLocalMidnight(now)).toBe(3_600_000);
  });
});
