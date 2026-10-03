import { normalizeName } from "../shared/names.ts";

interface Entry {
  display: string;
  norm: string;
}

export class Suggestions {
  private entries: Entry[] = [];

  constructor(names: string[] = []) {
    this.load(names);
  }

  load(names: string[]): void {
    this.entries = names.map((display) => ({ display, norm: normalizeName(display) }));
  }

  get size(): number {
    return this.entries.length;
  }

  match(query: string, limit = 6): string[] {
    const q = normalizeName(query);
    if (!q) return [];
    const starts: string[] = [];
    const contains: string[] = [];
    for (const e of this.entries) {
      if (e.norm.startsWith(q)) starts.push(e.display);
      else if (e.norm.includes(q)) contains.push(e.display);
      if (starts.length >= limit) break;
    }
    return starts.concat(contains).slice(0, limit);
  }
}
