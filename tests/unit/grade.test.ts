import { describe, expect, it } from "vitest";
import type { NameRecord, NamesFile, Person } from "../../shared/data";
import { answerRecord, facts, hint, isWin, phrases, placeName } from "../../shared/grade";

const languages = { Q1860: "English", Q150: "French", Q1321: "Spanish", Q188: "German" };

const rec = (over: Partial<NameRecord> = {}): NameRecord => ({
  display: "X",
  langs: ["Q1860"],
  families: ["germanic"],
  count: 1000,
  dm: "XX",
  rhyme: "XX",
  ...over,
});

const alan = rec({
  display: "Alan",
  langs: ["Q1860", "Q150"],
  count: 31000,
  dm: "ALN",
  rhyme: "LN",
});
const alec = rec({ display: "Alec", count: 2100, dm: "ALK", rhyme: "LK" });
const william = rec({ display: "William", count: 210000, dm: "ALM", rhyme: "LM" });
const edward = rec({ display: "Edward", count: 50000, dm: "ATRT", rhyme: "RT" });

const texts = (p: ReturnType<typeof phrases>) => p.map((x) => x.text);

describe("phrases", () => {
  it("grades William against Alan", () => {
    const p = phrases("william", william, "alan", alan, languages);
    expect(texts(p)).toEqual([
      "three shorter",
      "starts earlier",
      "same language (English)",
      "starts with the same sound",
      "much rarer",
    ]);
    expect(p.map((x) => x.exact)).toEqual([false, false, true, false, false]);
  });
  it("grades Alec against Alan", () => {
    const p = phrases("alec", alec, "alan", alan, languages);
    expect(texts(p)).toEqual([
      "same length",
      "same first letter",
      "same language (English)",
      "starts with the same sound",
      "much more common",
    ]);
  });
  it("grades Edward against Alan", () => {
    expect(texts(phrases("edward", edward, "alan", alan, languages))).toEqual([
      "two shorter",
      "starts earlier",
      "same language (English)",
      "starts with the same sound",
      "rarer",
    ]);
  });
  it("counts length differences up to five and then says much", () => {
    const a = rec();
    expect(phrases("ab", a, "abcdef", a, languages)[0]?.text).toBe("four longer");
    expect(phrases("ab", a, "abcdefg", a, languages)[0]?.text).toBe("five longer");
    expect(phrases("ab", a, "abcdefgh", a, languages)[0]?.text).toBe("much longer");
    expect(phrases("abcdefgh", a, "ab", a, languages)[0]?.text).toBe("much shorter");
    expect(phrases("mary ann", a, "jean-paul", a, languages)[0]?.text).toBe("one longer");
  });
  it("says starts later when the answer's first letter comes after", () => {
    expect(phrases("alan", alan, "edward", edward, languages)[1]?.text).toBe("starts later");
  });
  it("names the first shared language in the answer's order", () => {
    const g = rec({ langs: ["Q1860", "Q150"] });
    const a = rec({ langs: ["Q150", "Q1860"] });
    expect(phrases("a", g, "b", a, languages)[2]).toEqual({
      text: "same language (French)",
      exact: true,
    });
  });
  it("falls back to the family and then to different family", () => {
    const pierre = rec({ langs: ["Q150"], families: ["romance"] });
    const jose = rec({ langs: ["Q1321"], families: ["romance"] });
    const hans = rec({ langs: ["Q188"], families: ["germanic"] });
    expect(phrases("pierre", pierre, "jose", jose, languages)[2]).toEqual({
      text: "same family (Romance)",
      exact: false,
    });
    expect(phrases("pierre", pierre, "hans", hans, languages)[2]?.text).toBe("different family");
  });
  it("says language unknown when a side has no languages and no families", () => {
    const blank = rec({ langs: [], families: [] });
    expect(phrases("a", blank, "b", alan, languages)[2]?.text).toBe("language unknown");
    expect(phrases("a", alan, "b", blank, languages)[2]?.text).toBe("language unknown");
  });
  it("hears identical codes, shared first sounds, rhymes, and nothing", () => {
    const ellen = rec({ dm: "ALN", rhyme: "LN" });
    const helen = rec({ dm: "HLN", rhyme: "LN" });
    const pierre = rec({ dm: "PR", rhyme: "PR" });
    expect(phrases("ellen", ellen, "alan", alan, languages)[3]).toEqual({
      text: "sounds the same",
      exact: true,
    });
    expect(phrases("helen", helen, "alan", alan, languages)[3]?.text).toBe("rhymes");
    expect(phrases("pierre", pierre, "alan", alan, languages)[3]?.text).toBe("no shared sound");
  });
  it("describes popularity from the answer's side", () => {
    const common = rec({ count: 4000 });
    const rare = rec({ count: 900 });
    const same = rec({ count: 1100 });
    expect(phrases("g", common, "a", rare, languages)[4]?.text).toBe("much rarer");
    expect(phrases("g", rare, "a", common, languages)[4]?.text).toBe("much more common");
    expect(phrases("g", same, "a", rare, languages)[4]?.text).toBe("rarer");
    expect(phrases("g", rare, "a", same, languages)[4]?.text).toBe("more common");
    expect(phrases("g", rare, "a", rare, languages)[4]).toEqual({
      text: "equally common",
      exact: true,
    });
  });
});

const turing: Person = {
  qid: "Q7251",
  label: "Alan Turing",
  display: "Alan",
  names: ["alan"],
  born: 1912,
  citizenship: ["United Kingdom"],
  occupations: ["mathematician", "computer scientist"],
  description: null,
  wiki: null,
  crop: "crops/Q7251.jpg",
  image: { file: "", artist: "", licence: "", licenceUrl: "", pageUrl: "" },
};

describe("facts", () => {
  it("accumulate after guesses 2, 4, and 6", () => {
    expect(facts(turing, 1)).toEqual([]);
    expect(facts(turing, 2)).toEqual([{ kind: "born", text: "Born in the 1910s" }]);
    expect(facts(turing, 3)).toHaveLength(1);
    expect(facts(turing, 4)[1]).toEqual({ kind: "citizenship", text: "From the United Kingdom" });
    expect(facts(turing, 6)[2]).toEqual({
      kind: "field",
      text: "Known as a mathematician and computer scientist",
    });
  });
  it("joins lists and picks the article", () => {
    const p = {
      ...turing,
      citizenship: ["France", "Mexico", "Netherlands"],
      occupations: ["actress"],
    };
    expect(facts(p, 6).map((f) => f.text)).toEqual([
      "Born in the 1910s",
      "From France, Mexico and the Netherlands",
      "Known as an actress",
    ]);
  });
  it("skips facts with no data", () => {
    const p = { ...turing, citizenship: [], occupations: [] };
    expect(facts(p, 8)).toEqual([{ kind: "born", text: "Born in the 1910s" }]);
  });
  it("puts the article on compound country names", () => {
    expect(placeName("United States of America")).toBe("the United States of America");
    expect(placeName("Weimar Republic")).toBe("the Weimar Republic");
    expect(placeName("Japan")).toBe("Japan");
  });
});

describe("hint, win, and answer record", () => {
  it("names the answer's first language from the fifth guess", () => {
    expect(hint(alan, 4, languages)).toBeUndefined();
    expect(hint(alan, 5, languages)).toBe("the answer is used in English");
    expect(hint(rec({ langs: [] }), 8, languages)).toBeUndefined();
  });
  it("wins on any accepted form", () => {
    expect(isWin("alan", turing)).toBe(true);
    expect(isWin("allan", turing)).toBe(false);
  });
  it("finds the record by display and falls back to accepted forms", () => {
    const names: NamesFile = { version: 1, languages, names: { allan: alec } };
    const p = { ...turing, names: ["alan", "allan"] };
    expect(answerRecord(p, names)).toBe(alec);
    names.names.alan = alan;
    expect(answerRecord(p, names)).toBe(alan);
    expect(answerRecord(turing, { version: 1, languages, names: {} })).toBeUndefined();
  });
});
