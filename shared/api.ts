export const MAX_GUESSES = 8;
// One slot per winning guess count, then one for a loss.
export const RESULT_SLOTS = MAX_GUESSES + 1;

export interface HealthResponse {
  ok: true;
  epoch: string;
}

export interface PuzzleResponse {
  n: number;
  token: string;
  guessesLeft: number;
}

export interface Phrase {
  text: string;
  exact: boolean;
}

export type FactKind = "born" | "citizenship" | "field";

export interface Fact {
  kind: FactKind;
  text: string;
}

export interface ImageCredit {
  file: string;
  artist: string;
  licence: string;
  licenceUrl: string;
  pageUrl: string;
}

export interface Reveal {
  label: string;
  display: string;
  description: string | null;
  wiki: string | null;
  image: ImageCredit;
}

export interface GuessResponse {
  token: string;
  name: string;
  phrases: Phrase[];
  hint?: string;
  facts: Fact[];
  guessesLeft: number;
  done: boolean;
  won: boolean;
  reveal?: Reveal;
}

export interface StatsResponse {
  n: number;
  counts: number[];
  total: number;
}

export interface NamesResponse {
  names: string[];
}

export interface ErrorResponse {
  error: string;
}
