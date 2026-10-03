import { describe, expect, it } from "vitest";
import type { NameRecord, NamesFile, Person } from "../../shared/data";
import { answerRecord, facts, hint, isWin, phrases, placeName } from "../../shared/grade";
import { normalizeName } from "../../shared/names";

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
  era: 1952,
  sameAs: ["allan", "allen"],
});
const alec = rec({ display: "Alec", count: 2100, dm: "ALK", rhyme: "LK", era: 1948 });
const william = rec({
  display: "William",
  count: 210000,
  dm: "ALM",
  rhyme: "LM",
  era: 1925,
});
const edward = rec({ display: "Edward", count: 50000, dm: "ATRT", rhyme: "RT", era: 1920 });
const bill = rec({
  display: "Bill",
  count: 9000,
  dm: "PL",
  rhyme: "PL",
  era: 1935,
  shortOf: ["william"],
});

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
      "a newer name",
    ]);
    expect(p.map((x) => x.exact)).toEqual([false, false, true, false, false, false]);
  });
  it("grades Alec against Alan", () => {
    const p = phrases("alec", alec, "alan", alan, languages);
    expect(texts(p)).toEqual([
      "same length",
      "same first letter",
      "same language (English)",
      "starts with the same sound",
      "much more common",
      "same era",
    ]);
  });
  it("grades Edward against Alan", () => {
    expect(texts(phrases("edward", edward, "alan", alan, languages))).toEqual([
      "two shorter",
      "starts earlier",
      "same language (English)",
      "starts with the same sound",
      "rarer",
      "a newer name",
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
  it("calls eras within ten years the same and names the answer's direction", () => {
    const era = (year: number) => rec({ era: year });
    expect(phrases("g", era(1950), "a", era(1960), languages)[5]).toEqual({
      text: "same era",
      exact: true,
    });
    expect(phrases("g", era(1950), "a", era(1940), languages)[5]?.text).toBe("same era");
    expect(phrases("g", era(1950), "a", era(1939), languages)[5]).toEqual({
      text: "an older name",
      exact: false,
    });
    expect(phrases("g", era(1950), "a", era(1961), languages)[5]?.text).toBe("a newer name");
    expect(phrases("william", william, "alan", alan, languages)[5]?.text).toBe("a newer name");
    expect(phrases("alan", alan, "william", william, languages)[5]?.text).toBe("an older name");
  });
  it("says era unknown when either side has no era", () => {
    const blank = rec();
    const nulled = rec({ era: null });
    const unknown = { text: "era unknown", exact: false };
    expect(phrases("g", blank, "a", alan, languages)[5]).toEqual(unknown);
    expect(phrases("g", alan, "a", nulled, languages)[5]).toEqual(unknown);
    expect(phrases("g", blank, "a", nulled, languages)[5]).toEqual(unknown);
  });
  it("links forms of the same name from either side's sameAs", () => {
    const allan = rec({ display: "Allan", sameAs: ["alan"] });
    const allen = rec({ display: "Allen" });
    const same = { text: "a form of the same name", exact: true };
    expect(phrases("allan", allan, "alan", alan, languages)[6]).toEqual(same);
    expect(phrases("alan", alan, "allan", allan, languages)[6]).toEqual(same);
    expect(phrases("allen", allen, "alan", alan, languages)[6]).toEqual(same);
    expect(phrases("alan", alan, "allen", allen, languages)[6]).toEqual(same);
  });
  it("names which side is the short form", () => {
    expect(phrases("bill", bill, "william", william, languages)[6]).toEqual({
      text: "your guess is a short form of the answer",
      exact: true,
    });
    expect(phrases("william", william, "bill", bill, languages)[6]).toEqual({
      text: "the answer is a short form of your guess",
      exact: true,
    });
  });
  it("prefers sameAs over shortOf when both apply", () => {
    const will = rec({ display: "Will", sameAs: ["william"], shortOf: ["william"] });
    expect(phrases("will", will, "william", william, languages)[6]?.text).toBe(
      "a form of the same name",
    );
  });
  it("adds no root phrase when the names are unrelated", () => {
    expect(phrases("william", william, "alan", alan, languages)).toHaveLength(6);
    expect(phrases("bill", bill, "alan", alan, languages)).toHaveLength(6);
  });
  it("folds Ł, Ø, Ð, Þ, and Æ before comparing first letters", () => {
    const a = rec();
    const lukasz = normalizeName("Łukasz");
    expect(phrases("zoe", a, lukasz, a, languages)[1]?.text).toBe("starts earlier");
    expect(phrases(lukasz, a, "zoe", a, languages)[1]?.text).toBe("starts later");
    expect(phrases("lars", a, lukasz, a, languages)[1]).toEqual({
      text: "same first letter",
      exact: true,
    });
    expect(phrases("pierre", a, normalizeName("Øyvind"), a, languages)[1]?.text).toBe(
      "starts earlier",
    );
    expect(phrases("edward", a, normalizeName("Ðorđe"), a, languages)[1]?.text).toBe(
      "starts earlier",
    );
    expect(phrases("zoe", a, normalizeName("Þórður"), a, languages)[1]?.text).toBe(
      "starts earlier",
    );
    expect(phrases("bill", a, normalizeName("Æsa"), a, languages)[1]?.text).toBe("starts earlier");
  });
  it("says sound unknown when either side has no sound code", () => {
    const ivan = rec({ dm: "", rhyme: "" });
    const petr = rec({ dm: "", rhyme: "" });
    const unknown = { text: "sound unknown", exact: false };
    const p = phrases(normalizeName("Иван"), ivan, normalizeName("Петр"), petr, languages);
    expect(p[3]).toEqual(unknown);
    expect(p[1]?.text).toBe("starts later");
    expect(phrases("ivan", ivan, "alan", alan, languages)[3]).toEqual(unknown);
    expect(phrases("alan", alan, "petr", petr, languages)[3]).toEqual(unknown);
  });
  it("leaves apostrophes out of the letter count", () => {
    const a = rec();
    expect(phrases(normalizeName("D'Angelo"), a, "daniela", a, languages)[0]).toEqual({
      text: "same length",
      exact: true,
    });
    expect(phrases("daniela", a, normalizeName("D\u2019Angelo"), a, languages)[0]?.text).toBe(
      "same length",
    );
    expect(phrases("daniela", a, normalizeName("\u2018Abd"), a, languages)[0]?.text).toBe(
      "four shorter",
    );
  });
  it("grades records without era, sameAs, or shortOf", () => {
    const old = {
      display: "Old",
      langs: ["Q1860"],
      families: ["germanic"],
      count: 5,
      dm: "AL",
      rhyme: "L",
    };
    const p = phrases("old", old, "alan", alan, languages);
    expect(p).toHaveLength(6);
    expect(p[5]).toEqual({ text: "era unknown", exact: false });
    expect(phrases("old", old, "old", old, languages).every((x) => x.exact)).toBe(true);
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
