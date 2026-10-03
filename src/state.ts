import type { Fact, Phrase, Reveal } from "../shared/api.ts";
import { MAX_GUESSES } from "../shared/api.ts";

export interface Row {
  name: string;
  phrases: Phrase[];
  hint?: string;
}

export interface Game {
  n: number;
  token: string;
  rows: Row[];
  facts: Fact[];
  done: boolean;
  won: boolean;
  reveal?: Reveal;
  recorded?: boolean;
}

export interface Stats {
  played: number;
  wins: number;
  streak: number;
  maxStreak: number;
  dist: number[];
  lastN: number | null;
}

const GAME_KEY = "whom:game";
const STATS_KEY = "whom:stats";
const SEEN_KEY = "whom:seen";

function read<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
}

function write(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage can be full or blocked; the game still plays for this page load.
  }
}

export function loadGame(n: number): Game | null {
  const game = read<Game>(GAME_KEY);
  return game && game.n === n && Array.isArray(game.rows) ? game : null;
}

export function saveGame(game: Game): void {
  write(GAME_KEY, game);
}

export function emptyStats(): Stats {
  return {
    played: 0,
    wins: 0,
    streak: 0,
    maxStreak: 0,
    dist: new Array<number>(MAX_GUESSES + 1).fill(0),
    lastN: null,
  };
}

export function loadStats(): Stats {
  const stats = read<Stats>(STATS_KEY);
  if (!stats || !Array.isArray(stats.dist) || stats.dist.length !== MAX_GUESSES + 1) {
    return emptyStats();
  }
  return stats;
}

export function recordResult(stats: Stats, n: number, won: boolean, guesses: number): Stats {
  const dist = stats.dist.slice();
  const slot = won ? guesses - 1 : MAX_GUESSES;
  dist[slot] = (dist[slot] ?? 0) + 1;
  const continues = stats.lastN === n - 1;
  const streak = won ? (continues ? stats.streak + 1 : 1) : 0;
  const next: Stats = {
    played: stats.played + 1,
    wins: stats.wins + (won ? 1 : 0),
    streak,
    maxStreak: Math.max(stats.maxStreak, streak),
    dist,
    lastN: n,
  };
  write(STATS_KEY, next);
  return next;
}

export function hasSeenHelp(): boolean {
  return read<boolean>(SEEN_KEY) === true;
}

export function markHelpSeen(): void {
  write(SEEN_KEY, true);
}
