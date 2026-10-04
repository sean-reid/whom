import { describe, expect, it } from "vitest";
import type { NameRecord, NamesFile, Person } from "../../shared/data";
import {
  answerRecord,
  facts,
  fallbackRecord,
  hint,
  isWin,
  phrases,
  placeName,
} from "../../shared/grade";
import { normalizeName } from "../../shared/names";

const regions = {
  "western-europe": "Western Europe",
  "northern-europe": "Northern Europe",
  "northern-america": "Northern America",
  "eastern-asia": "Eastern Asia",
};

const rec = (over: Partial<NameRecord> = {}): NameRecord => ({
  display: "X",
  langs: ["Q1860"],
  families: ["germanic"],
  count: 1000,
  dm: "XX",
  rhyme: "XX",
  region: "western-europe",
  continent: "europe",
  regionShare: 0.5,
  ...over,
});

const alan = rec({
  display: "Alan",
  count: 31000,
  dm: "ALN",
  rhyme: "LN",
  era: 1952,
  sameAs: ["allan", "allen"],
});
const alec = rec({
  display: "Alec",
  count: 2100,
  dm: "ALK",
  rhyme: "LK",
  era: 1948,
  region: "northern-europe",
});
const william = rec({
  display: "William",
  count: 210000,
  dm: "ALM",
  rhyme: "LM",
  era: 1925,
  region: "northern-america",
  continent: "americas",
});
const edward = rec({ display: "Edward", count: 50000, dm: "ATRT", rhyme: "RT", era: 1920 });
const bill = rec({
  display: "Bill",
  count: 9000,
  dm: "PL",
  rhyme: "PL",
  era: 1935,
  region: "northern-america",
  continent: "americas",
  shortOf: ["william"],
});

const texts = (p: ReturnType<typeof phrases>) => p.map((x) => x.text);

describe("phrases", () => {
  it("grades William against Alan", () => {
    const p = phrases("william", william, "alan", alan, regions);
    expect(texts(p)).toEqual([
      "three letters shorter",
      "starts earlier in the alphabet",
      "from a different part of the world",
      "starts with the same sound",
      "much rarer",
      "a later generation",
    ]);
    expect(p.map((x) => x.exact)).toEqual([false, false, false, false, false, false]);
  });
  it("grades Alec against Alan", () => {
    const p = phrases("alec", alec, "alan", alan, regions);
    expect(texts(p)).toEqual([
      "same length",
      "same first letter",
      "both European names",
      "starts with the same sound",
      "much more common",
      "same generation",
    ]);
    expect(p.map((x) => x.exact)).toEqual([true, true, false, false, false, true]);
  });
  it("grades Edward against Alan", () => {
    expect(texts(phrases("edward", edward, "alan", alan, regions))).toEqual([
      "two letters shorter",
      "starts earlier in the alphabet",
      "both common in Western Europe",
      "starts with the same sound",
      "a bit rarer",
      "a later generation",
    ]);
  });
  it("counts letters up to five and then says much", () => {
    const a = rec();
    expect(phrases("ab", a, "abcdef", a, regions)[0]?.text).toBe("four letters longer");
    expect(phrases("ab", a, "abcdefg", a, regions)[0]?.text).toBe("five letters longer");
    expect(phrases("ab", a, "abcdefgh", a, regions)[0]?.text).toBe("much longer");
    expect(phrases("abcdefgh", a, "ab", a, regions)[0]?.text).toBe("much shorter");
    expect(phrases("abc", a, "ab", a, regions)[0]?.text).toBe("one letter shorter");
    expect(phrases("mary ann", a, "jean-paul", a, regions)[0]?.text).toBe("one letter longer");
  });
  it("says where the answer's first letter sits in the alphabet", () => {
    expect(phrases("alan", alan, "edward", edward, regions)[1]?.text).toBe(
      "starts later in the alphabet",
    );
    expect(phrases("edward", edward, "alan", alan, regions)[1]?.text).toBe(
      "starts earlier in the alphabet",
    );
  });
  it("names the shared subregion with its label", () => {
    expect(phrases("edward", edward, "alan", alan, regions)[2]).toEqual({
      text: "both common in Western Europe",
      exact: true,
    });
    const tokyo = rec({ region: "eastern-asia", continent: "asia" });
    expect(phrases("a", tokyo, "b", tokyo, regions)[2]?.text).toBe("both common in Eastern Asia");
  });
  it("spells out a slug when the file has no label for it", () => {
    const a = rec({ region: "south-eastern-asia", continent: "asia" });
    expect(phrases("a", a, "b", a, regions)[2]?.text).toBe("both common in South Eastern Asia");
    expect(phrases("a", a, "b", a)[2]?.text).toBe("both common in South Eastern Asia");
  });
  it("falls back to the continent and then to a different part of the world", () => {
    const by = (continent: string, region: string) => rec({ region, continent });
    const pairs: [string, string, string, string][] = [
      ["europe", "western-europe", "northern-europe", "both European names"],
      ["americas", "northern-america", "south-america", "both American names"],
      ["asia", "eastern-asia", "southern-asia", "both Asian names"],
      ["africa", "southern-africa", "northern-africa", "both African names"],
      ["oceania", "melanesia", "polynesia", "both Oceanian names"],
    ];
    for (const [continent, g, a, text] of pairs) {
      expect(phrases("g", by(continent, g), "a", by(continent, a), regions)[2]).toEqual({
        text,
        exact: false,
      });
    }
    expect(phrases("william", william, "alan", alan, regions)[2]?.text).toBe(
      "from a different part of the world",
    );
    expect(phrases("alan", alan, "william", william, regions)[2]?.text).toBe(
      "from a different part of the world",
    );
  });
  it("says region unknown when either side has no region", () => {
    const nulled = rec({ region: null, continent: null });
    expect(phrases("a", nulled, "b", alan, regions)[2]).toEqual({
      text: "region unknown",
      exact: false,
    });
    expect(phrases("a", alan, "b", nulled, regions)[2]?.text).toBe("region unknown");
    expect(phrases("a", nulled, "b", nulled, regions)[2]?.text).toBe("region unknown");
  });
  it("hears identical codes, shared first sounds, shared endings, and nothing", () => {
    const ellen = rec({ dm: "ALN", rhyme: "LN" });
    const helen = rec({ dm: "HLN", rhyme: "LN" });
    const pierre = rec({ dm: "PR", rhyme: "PR" });
    expect(phrases("ellen", ellen, "alan", alan, regions)[3]).toEqual({
      text: "sounds alike",
      exact: true,
    });
    expect(phrases("helen", helen, "alan", alan, regions)[3]).toEqual({
      text: "ends the same way",
      exact: false,
    });
    expect(phrases("pierre", pierre, "alan", alan, regions)[3]?.text).toBe("no shared sound");
  });
  it("steps popularity three times per doubling and names the answer's side", () => {
    const at = (count: number) => rec({ count });
    const pop = (g: number, a: number) => phrases("g", at(g), "a", at(a), regions)[4];
    expect(pop(1023, 1023)).toEqual({ text: "equally common", exact: true });
    expect(pop(1023, 1200)?.text).toBe("equally common");
    expect(pop(1023, 1500)?.text).toBe("a bit more common");
    expect(pop(1500, 1023)?.text).toBe("a bit rarer");
    expect(pop(1023, 1700)?.text).toBe("a bit more common");
    expect(pop(1023, 2047)).toEqual({ text: "more common", exact: false });
    expect(pop(2047, 1023)?.text).toBe("rarer");
    expect(pop(1023, 3500)?.text).toBe("more common");
    expect(pop(1023, 4095)?.text).toBe("much more common");
    expect(pop(4095, 1023)?.text).toBe("much rarer");
    expect(pop(0, 3)?.text).toBe("much more common");
  });
  it("calls eras within ten years one generation and names the answer's direction", () => {
    const era = (year: number) => rec({ era: year });
    expect(phrases("g", era(1950), "a", era(1960), regions)[5]).toEqual({
      text: "same generation",
      exact: true,
    });
    expect(phrases("g", era(1950), "a", era(1940), regions)[5]?.text).toBe("same generation");
    expect(phrases("g", era(1950), "a", era(1939), regions)[5]).toEqual({
      text: "an earlier generation",
      exact: false,
    });
    expect(phrases("g", era(1950), "a", era(1961), regions)[5]?.text).toBe("a later generation");
    expect(phrases("william", william, "alan", alan, regions)[5]?.text).toBe("a later generation");
    expect(phrases("alan", alan, "william", william, regions)[5]?.text).toBe(
      "an earlier generation",
    );
  });
  it("says era unknown when either side has no era", () => {
    const blank = rec();
    const nulled = rec({ era: null });
    const unknown = { text: "era unknown", exact: false };
    expect(phrases("g", blank, "a", alan, regions)[5]).toEqual(unknown);
    expect(phrases("g", alan, "a", nulled, regions)[5]).toEqual(unknown);
    expect(phrases("g", blank, "a", nulled, regions)[5]).toEqual(unknown);
  });
  it("links forms of the same name from either side's sameAs", () => {
    const allan = rec({ display: "Allan", sameAs: ["alan"] });
    const allen = rec({ display: "Allen" });
    const same = { text: "a form of the same name", exact: true };
    expect(phrases("allan", allan, "alan", alan, regions)[6]).toEqual(same);
    expect(phrases("alan", alan, "allan", allan, regions)[6]).toEqual(same);
    expect(phrases("allen", allen, "alan", alan, regions)[6]).toEqual(same);
    expect(phrases("alan", alan, "allen", allen, regions)[6]).toEqual(same);
  });
  it("names which side is the short form", () => {
    expect(phrases("bill", bill, "william", william, regions)[6]).toEqual({
      text: "your guess is a short form of the answer",
      exact: true,
    });
    expect(phrases("william", william, "bill", bill, regions)[6]).toEqual({
      text: "the answer is a short form of your guess",
      exact: true,
    });
  });
  it("prefers sameAs over shortOf when both apply", () => {
    const will = rec({ display: "Will", sameAs: ["william"], shortOf: ["william"] });
    expect(phrases("will", will, "william", william, regions)[6]?.text).toBe(
      "a form of the same name",
    );
  });
  it("adds no root phrase when the names are unrelated", () => {
    expect(phrases("william", william, "alan", alan, regions)).toHaveLength(6);
    expect(phrases("bill", bill, "alan", alan, regions)).toHaveLength(6);
  });
  it("folds Ł, Ø, Ð, Þ, and Æ before comparing first letters", () => {
    const a = rec();
    const lukasz = normalizeName("Łukasz");
    const earlier = "starts earlier in the alphabet";
    expect(phrases("zoe", a, lukasz, a, regions)[1]?.text).toBe(earlier);
    expect(phrases(lukasz, a, "zoe", a, regions)[1]?.text).toBe("starts later in the alphabet");
    expect(phrases("lars", a, lukasz, a, regions)[1]).toEqual({
      text: "same first letter",
      exact: true,
    });
    expect(phrases("pierre", a, normalizeName("Øyvind"), a, regions)[1]?.text).toBe(earlier);
    expect(phrases("edward", a, normalizeName("Ðorđe"), a, regions)[1]?.text).toBe(earlier);
    expect(phrases("zoe", a, normalizeName("Þórður"), a, regions)[1]?.text).toBe(earlier);
    expect(phrases("bill", a, normalizeName("Æsa"), a, regions)[1]?.text).toBe(earlier);
  });
  it("says sound unknown when either side has no sound code", () => {
    const ivan = rec({ dm: "", rhyme: "" });
    const petr = rec({ dm: "", rhyme: "" });
    const unknown = { text: "sound unknown", exact: false };
    const p = phrases(normalizeName("Иван"), ivan, normalizeName("Петр"), petr, regions);
    expect(p[3]).toEqual(unknown);
    expect(p[1]?.text).toBe("starts later in the alphabet");
    expect(phrases("ivan", ivan, "alan", alan, regions)[3]).toEqual(unknown);
    expect(phrases("alan", alan, "petr", petr, regions)[3]).toEqual(unknown);
  });
  it("leaves apostrophes out of the letter count", () => {
    const a = rec();
    expect(phrases(normalizeName("D'Angelo"), a, "daniela", a, regions)[0]).toEqual({
      text: "same length",
      exact: true,
    });
    expect(phrases("daniela", a, normalizeName("D\u2019Angelo"), a, regions)[0]?.text).toBe(
      "same length",
    );
    expect(phrases("daniela", a, normalizeName("\u2018Abd"), a, regions)[0]?.text).toBe(
      "four letters shorter",
    );
  });
  it("grades records written before era, region, sameAs, or shortOf existed", () => {
    const old = {
      display: "Old",
      langs: ["Q1860"],
      families: ["germanic"],
      count: 5,
      dm: "AL",
      rhyme: "L",
    };
    const p = phrases("old", old, "alan", alan, regions);
    expect(p).toHaveLength(6);
    expect(p[2]).toEqual({ text: "region unknown", exact: false });
    expect(p[5]).toEqual({ text: "era unknown", exact: false });
    expect(phrases("alan", alan, "old", old, regions)[2]?.text).toBe("region unknown");
    expect(phrases("old", old, "old", old, regions).map((x) => x.exact)).toEqual([
      true,
      true,
      false,
      true,
      true,
      true,
    ]);
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
  it("names the answer's subregion from the fifth guess", () => {
    expect(hint(alan, 4, regions)).toBeUndefined();
    expect(hint(alan, 5, regions)).toBe("the answer is common in Western Europe");
    expect(hint(alan, 5)).toBe("the answer is common in Western Europe");
    expect(hint(rec({ region: null }), 8, regions)).toBeUndefined();
    expect(hint(fallbackRecord(turing), 8, regions)).toBeUndefined();
  });
  it("wins on any accepted form", () => {
    expect(isWin("alan", turing)).toBe(true);
    expect(isWin("allan", turing)).toBe(false);
  });
  it("grades a nickname win against the display name, root phrase and all", () => {
    const clinton = {
      ...turing,
      label: "Bill Clinton",
      display: "William",
      names: ["william", "bill"],
    };
    expect(isWin("bill", clinton)).toBe(true);
    const p = phrases("bill", bill, normalizeName(clinton.display), william, regions);
    expect(texts(p)).toEqual([
      "three letters longer",
      "starts later in the alphabet",
      "both common in Northern America",
      "no shared sound",
      "much more common",
      "same generation",
      "your guess is a short form of the answer",
    ]);
    expect(p.map((x) => x.exact)).toEqual([false, false, true, false, false, true, true]);
    expect(phrases("william", william, "william", william, regions).every((x) => x.exact)).toBe(
      true,
    );
  });
  it("grades a person with no record against an empty one", () => {
    const empty = fallbackRecord(turing);
    expect(empty).toEqual({
      display: "Alan",
      langs: [],
      families: [],
      count: 0,
      dm: "",
      rhyme: "",
      era: null,
      region: null,
      continent: null,
    });
    expect(texts(phrases("william", william, "alan", empty, regions))).toEqual([
      "three letters shorter",
      "starts earlier in the alphabet",
      "region unknown",
      "sound unknown",
      "much rarer",
      "era unknown",
    ]);
    expect(phrases("alan", empty, "alan", empty, regions).map((x) => x.exact)).toEqual([
      true,
      true,
      false,
      false,
      true,
      true,
    ]);
    expect(hint(empty, 8, regions)).toBeUndefined();
  });
  it("finds the record by display and falls back to accepted forms", () => {
    const names: NamesFile = { version: 1, languages: {}, names: { allan: alec } };
    const p = { ...turing, names: ["alan", "allan"] };
    expect(answerRecord(p, names)).toBe(alec);
    names.names.alan = alan;
    expect(answerRecord(p, names)).toBe(alan);
    expect(answerRecord(turing, { version: 1, languages: {}, names: {} })).toBeUndefined();
  });
});
