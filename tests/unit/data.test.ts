import { beforeEach, describe, expect, it } from "vitest";
import type { NamesFile, Person, PoolFile } from "../../shared/data";
import { findPerson, loadData, resetData } from "../../worker/data";

const person = (qid: string): Person => ({
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
});

const names: NamesFile = { version: 1, languages: {}, names: {} };
const poolOf = (qids: string[]): PoolFile => ({
  version: 1,
  generated: "2026-10-01",
  people: qids.map(person),
});

// A bucket whose pool.json can change between reads, the way an upload changes R2.
function bucket(qids: string[]) {
  const files: Record<string, unknown> = { "pool.json": poolOf(qids), "names.json": names };
  let reads = 0;
  return {
    env: {
      FILES: {
        get: async (key: string) => {
          reads += 1;
          const file = files[key];
          return file === undefined ? null : { json: async () => file };
        },
      },
      PUZZLE_SEED: "seed",
    },
    upload: (next: string[]) => {
      files["pool.json"] = poolOf(next);
    },
    reads: () => reads,
  };
}

beforeEach(() => resetData());

describe("loadData", () => {
  it("reads the two files once and serves the same data after", async () => {
    const r2 = bucket(["Q1", "Q2"]);
    const first = await loadData(r2.env);
    const second = await loadData(r2.env);
    expect(second).toBe(first);
    expect(r2.reads()).toBe(2);
    expect([...first.byQid.keys()]).toEqual(["Q1", "Q2"]);
    expect(first.order.map((p) => p.qid).sort()).toEqual(["Q1", "Q2"]);
  });
  it("keeps serving the cached pool after an upload until resetData", async () => {
    const r2 = bucket(["Q1"]);
    await loadData(r2.env);
    r2.upload(["Q1", "Q2"]);
    expect((await loadData(r2.env)).byQid.has("Q2")).toBe(false);
    resetData();
    expect((await loadData(r2.env)).byQid.has("Q2")).toBe(true);
    expect(r2.reads()).toBe(4);
  });
  it("throws when a file is missing and lets the next call retry", async () => {
    const r2 = bucket(["Q1"]);
    r2.env.FILES.get = async () => null;
    await expect(loadData(r2.env)).rejects.toThrow(/missing from R2/);
    r2.env.FILES.get = bucket(["Q1"]).env.FILES.get;
    expect((await loadData(r2.env)).byQid.has("Q1")).toBe(true);
  });
});

describe("findPerson", () => {
  it("returns the cached pool's person without a reload", async () => {
    const r2 = bucket(["Q1"]);
    const found = await findPerson(r2.env, undefined, "Q1");
    expect(found?.person.qid).toBe("Q1");
    expect(r2.reads()).toBe(2);
  });
  it("reloads once when the qid is missing and returns the fresh pool", async () => {
    const r2 = bucket(["Q1"]);
    await loadData(r2.env);
    r2.upload(["Q1", "Q2"]);
    const found = await findPerson(r2.env, undefined, "Q2");
    expect(found?.person.qid).toBe("Q2");
    expect(found?.data.byQid.has("Q2")).toBe(true);
    expect(r2.reads()).toBe(4);
    expect(await loadData(r2.env)).toBe(found?.data);
  });
  it("returns null after one reload when the qid is in neither pool", async () => {
    const r2 = bucket(["Q1"]);
    await loadData(r2.env);
    expect(await findPerson(r2.env, undefined, "Q9")).toBeNull();
    expect(r2.reads()).toBe(4);
    expect(await findPerson(r2.env, undefined, "Q9")).toBeNull();
    expect(r2.reads()).toBe(6);
  });
});
