import type { ImageCredit } from "./api.ts";

export interface Person {
  qid: string;
  label: string;
  display: string;
  names: string[];
  born: number;
  citizenship: string[];
  occupations: string[];
  description: string | null;
  wiki: string | null;
  crop: string;
  image: ImageCredit;
  retired?: true;
}

export interface PoolFile {
  version: 1;
  generated: string;
  people: Person[];
}

export interface NameRecord {
  display: string;
  langs: string[];
  families: string[];
  count: number;
  dm: string;
  rhyme: string;
}

export interface NamesFile {
  version: 1;
  languages: Record<string, string>;
  names: Record<string, NameRecord>;
}
