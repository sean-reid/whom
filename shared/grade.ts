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

export function isWin(guess: string, answer: Person): boolean {
  return answer.names.includes(guess);
}

const letters = (name: string): number => name.replace(/[\s-]/g, "").length;

function lengthPhrase(guess: string, answer: string): Phrase {
  const diff = letters(answer) - letters(guess);
  if (diff === 0) return { text: "same length", exact: true };
  const side = diff < 0 ? "shorter" : "longer";
  const word = COUNTS[Math.abs(diff) - 1];
  return { text: word ? `${word} ${side}` : `much ${side}`, exact: false };
}

function firstLetterPhrase(guess: string, answer: string): Phrase {
  const g = guess.codePointAt(0) ?? 0;
  const a = answer.codePointAt(0) ?? 0;
  if (g === a) return { text: "same first letter", exact: true };
  return { text: a < g ? "starts earlier" : "starts later", exact: false };
}

const capitalise = (s: string): string => s.charAt(0).toUpperCase() + s.slice(1);

function languagePhrase(
  guess: NameRecord,
  answer: NameRecord,
  languages: Record<string, string>,
): Phrase {
  const shared = answer.langs.find((l) => guess.langs.includes(l));
  if (shared) return { text: `same language (${languages[shared] ?? shared})`, exact: true };
  const family = answer.families.find((f) => guess.families.includes(f));
  if (family) return { text: `same family (${capitalise(family)})`, exact: false };
  const unknown = (r: NameRecord) => r.langs.length === 0 && r.families.length === 0;
  if (unknown(guess) || unknown(answer)) return { text: "language unknown", exact: false };
  return { text: "different family", exact: false };
}

function soundPhrase(guess: NameRecord, answer: NameRecord): Phrase {
  if (guess.dm === answer.dm) return { text: "sounds the same", exact: true };
  if (guess.dm.charAt(0) === answer.dm.charAt(0))
    return { text: "starts with the same sound", exact: false };
  if (guess.rhyme === answer.rhyme) return { text: "rhymes", exact: false };
  return { text: "no shared sound", exact: false };
}

const bucket = (count: number): number => Math.floor(Math.log2(count + 1));

function popularityPhrase(guess: NameRecord, answer: NameRecord): Phrase {
  const diff = bucket(answer.count) - bucket(guess.count);
  if (diff === 0) return { text: "equally common", exact: true };
  const side = diff < 0 ? "rarer" : "more common";
  return { text: Math.abs(diff) === 1 ? side : `much ${side}`, exact: false };
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
      ? { text: "same era", exact: true }
      : { text: "era unknown", exact: false };
  }
  const diff = a - g;
  if (Math.abs(diff) <= ERA_SPAN) return { text: "same era", exact: true };
  return { text: diff < 0 ? "an older name" : "a newer name", exact: false };
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
  languages: Record<string, string>,
): Phrase[] {
  const out = [
    lengthPhrase(guess, answer),
    firstLetterPhrase(guess, answer),
    languagePhrase(guessRec, answerRec, languages),
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
  languages: Record<string, string>,
): string | undefined {
  const lang = answerRec.langs[0];
  if (guessCount < 5 || lang === undefined) return undefined;
  return `the answer is used in ${languages[lang] ?? lang}`;
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
