import { describe, expect, it } from "vitest";
import { displaysByPopularity } from "../../worker/data";

describe("displaysByPopularity", () => {
  it("puts the most carried name first and breaks ties alphabetically", () => {
    const names = {
      zed: { display: "Zed", count: 5 },
      amy: { display: "Amy", count: 5 },
      bob: { display: "Bob", count: 900 },
    };
    expect(displaysByPopularity(names)).toEqual(["Bob", "Amy", "Zed"]);
  });
});
