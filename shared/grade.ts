import type { Fact, Phrase } from "./api.ts";
import type { NameRecord, NamesFile, Person } from "./data.ts";
import { normalizeName } from "./names.ts";

const COUNTS = ["one", "two", "three", "four", "five"];

export function answerRecord(answer: Person, names: NamesFile): NameRecord | undefined {
  const byDisplay = names.names[normalizeName(answer.display)];
  if (byDisplay) return byDisplay;
  for (const key of answer.names) {
    const rec = names.names[key];
    if (rec) return rec;
  }
  return undefined;
}

// Grades a person the name graph has not caught up with yet.
export function fallbackRecord(answer: Person): NameRecord {
  return {
    display: answer.display,
    langs: [],
    families: [],
    count: 0,
    dm: "",
    rhyme: "",
    era: null,
    region: null,
    continent: null,
  };
}

export function isWin(guess: string, answer: Person): boolean {
  return answer.names.includes(guess);
}

const letters = (name: string): number => name.replace(/[\s'\u2018\u2019-]/g, "").length;

function lengthPhrase(guess: string, answer: string): Phrase {
  const diff = letters(answer) - letters(guess);
  if (diff === 0) return { text: "same length", exact: true };
  const side = diff < 0 ? "shorter" : "longer";
  const n = Math.abs(diff);
  const word = COUNTS[n - 1];
  if (!word) return { text: `much ${side}`, exact: false };
  return { text: `${word} ${n === 1 ? "letter" : "letters"} ${side}`, exact: false };
}

// NFKD leaves these letters whole; the pipeline folds them the same way in text.rs.
const FOLDED: Record<string, string> = {
  ł: "l",
  ø: "o",
  ɔ: "o",
  đ: "d",
  ð: "d",
  æ: "a",
  œ: "o",
  ß: "s",
  þ: "t",
  ı: "i",
  ħ: "h",
  ŧ: "t",
};

const firstLetter = (name: string): number => {
  const first = String.fromCodePoint(name.codePointAt(0) ?? 0);
  return (FOLDED[first] ?? first).codePointAt(0) ?? 0;
};

function firstLetterPhrase(guess: string, answer: string): Phrase {
  const g = firstLetter(guess);
  const a = firstLetter(answer);
  if (g === a) return { text: "same first letter", exact: true };
  const side = a < g ? "earlier" : "later";
  return { text: `starts ${side} in the alphabet`, exact: false };
}

const capitalise = (s: string): string => s.charAt(0).toUpperCase() + s.slice(1);

const regionLabel = (slug: string, regions: Record<string, string>): string =>
  regions[slug] ?? slug.split("-").map(capitalise).join(" ");

const CONTINENT_ADJECTIVES: Record<string, string> = {
  europe: "European",
  americas: "American",
  asia: "Asian",
  africa: "African",
  oceania: "Oceanian",
};

function regionPhrase(
  guess: NameRecord,
  answer: NameRecord,
  regions: Record<string, string>,
): Phrase {
  if (!guess.region || !answer.region) return { text: "region unknown", exact: false };
  if (guess.region === answer.region)
    return { text: `both common in ${regionLabel(answer.region, regions)}`, exact: true };
  if (guess.continent && guess.continent === answer.continent) {
    const adjective = CONTINENT_ADJECTIVES[answer.continent] ?? capitalise(answer.continent);
    return { text: `both ${adjective} names`, exact: false };
  }
  return { text: "from a different part of the world", exact: false };
}

function soundPhrase(guess: NameRecord, answer: NameRecord): Phrase {
  if (guess.dm === "" || answer.dm === "") return { text: "sound unknown", exact: false };
  if (guess.dm === answer.dm) return { text: "sounds alike", exact: true };
  if (guess.dm.charAt(0) === answer.dm.charAt(0))
    return { text: "starts with the same sound", exact: false };
  if (guess.rhyme === answer.rhyme) return { text: "ends the same way", exact: false };
  return { text: "no shared sound", exact: false };
}

// Three steps per doubling of holders.
const bucket = (count: number): number => Math.floor(3 * Math.log2(count + 1));

function popularityPhrase(guess: NameRecord, answer: NameRecord): Phrase {
  const diff = bucket(answer.count) - bucket(guess.count);
  if (diff === 0) return { text: "equally common", exact: true };
  const side = diff < 0 ? "rarer" : "more common";
  const gap = Math.abs(diff);
  if (gap <= 2) return { text: `a bit ${side}`, exact: false };
  if (gap <= 5) return { text: side, exact: false };
  return { text: `much ${side}`, exact: false };
}

const ERA_SPAN = 10;

function eraPhrase(
  guess: string,
  guessRec: NameRecord,
  answer: string,
  answerRec: NameRecord,
): Phrase {
  const g = guessRec.era;
  const a = answerRec.era;
  if (typeof g !== "number" || typeof a !== "number") {
    return guess === answer
      ? { text: "same generation", exact: true }
      : { text: "era unknown", exact: false };
  }
  const diff = a - g;
  if (Math.abs(diff) <= ERA_SPAN) return { text: "same generation", exact: true };
  return { text: diff < 0 ? "an earlier generation" : "a later generation", exact: false };
}

function rootPhrase(
  guess: string,
  guessRec: NameRecord,
  answer: string,
  answerRec: NameRecord,
): Phrase | undefined {
  if (answerRec.sameAs?.includes(guess) || guessRec.sameAs?.includes(answer))
    return { text: "a form of the same name", exact: true };
  if (guessRec.shortOf?.includes(answer))
    return { text: "your guess is a short form of the answer", exact: true };
  if (answerRec.shortOf?.includes(guess))
    return { text: "the answer is a short form of your guess", exact: true };
  return undefined;
}

export function phrases(
  guess: string,
  guessRec: NameRecord,
  answer: string,
  answerRec: NameRecord,
  regions: Record<string, string> = {},
): Phrase[] {
  const out = [
    lengthPhrase(guess, answer),
    firstLetterPhrase(guess, answer),
    regionPhrase(guessRec, answerRec, regions),
    soundPhrase(guessRec, answerRec),
    popularityPhrase(guessRec, answerRec),
    eraPhrase(guess, guessRec, answer, answerRec),
  ];
  const root = rootPhrase(guess, guessRec, answer, answerRec);
  if (root) out.push(root);
  return out;
}

export function hint(
  answerRec: NameRecord,
  guessCount: number,
  regions: Record<string, string> = {},
): string | undefined {
  const region = answerRec.region;
  if (guessCount < 5 || !region) return undefined;
  return `the answer is common in ${regionLabel(region, regions)}`;
}

const article = (word: string): string => (/^[aeio]|^u(?!ni|se|su|ro)/i.test(word) ? "an" : "a");

function joinList(items: string[]): string {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

const THE_COUNTRIES = new Set([
  "Netherlands",
  "Philippines",
  "Bahamas",
  "Gambia",
  "Maldives",
  "Comoros",
  "Isle of Man",
  "Ivory Coast",
  "Vatican City",
]);
const THE_WORDS =
  /\b(Republic|Kingdom|States|Islands|Empire|Federation|Emirates|Union|Confederation|Commonwealth|Territory|Protectorate|Mandate)\b/;

export function placeName(country: string): string {
  return THE_COUNTRIES.has(country) || THE_WORDS.test(country) ? `the ${country}` : country;
}

export function facts(person: Person, guessCount: number): Fact[] {
  const out: Fact[] = [];
  if (guessCount >= 2 && Number.isInteger(person.born)) {
    out.push({ kind: "born", text: `Born in the ${Math.floor(person.born / 10) * 10}s` });
  }
  if (guessCount >= 4 && person.citizenship.length > 0) {
    out.push({ kind: "citizenship", text: `From ${joinList(person.citizenship.map(placeName))}` });
  }
  if (guessCount >= 6 && person.occupations.length > 0) {
    const first = person.occupations[0] ?? "";
    out.push({
      kind: "field",
      text: `Known as ${article(first)} ${joinList(person.occupations)}`,
    });
  }
  return out;
}
