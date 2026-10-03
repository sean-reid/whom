import { describe, expect, it } from "vitest";
import { emptyStats, recordResult } from "../../src/state";

describe("recordResult", () => {
  it("counts a win in the matching slot and starts a streak", () => {
    const s = recordResult(emptyStats(), 5, true, 3);
    expect(s.played).toBe(1);
    expect(s.wins).toBe(1);
    expect(s.dist[2]).toBe(1);
    expect(s.streak).toBe(1);
    expect(s.maxStreak).toBe(1);
    expect(s.lastN).toBe(5);
  });
  it("extends the streak only across consecutive days", () => {
    let s = recordResult(emptyStats(), 5, true, 2);
    s = recordResult(s, 6, true, 1);
    expect(s.streak).toBe(2);
    s = recordResult(s, 9, true, 4);
    expect(s.streak).toBe(1);
    expect(s.maxStreak).toBe(2);
  });
  it("puts a loss in the last slot and resets the streak", () => {
    let s = recordResult(emptyStats(), 1, true, 8);
    s = recordResult(s, 2, false, 8);
    expect(s.dist[7]).toBe(1);
    expect(s.dist[8]).toBe(1);
    expect(s.streak).toBe(0);
    expect(s.wins).toBe(1);
  });
});
