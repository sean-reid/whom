import type { Fact, FactKind, Reveal, StatsResponse } from "../shared/api.ts";
import { MAX_GUESSES } from "../shared/api.ts";
import type { Row, Stats } from "./state.ts";

const LOCKED: { kind: FactKind; after: number; text: string }[] = [
  { kind: "born", after: 2, text: "Birth decade after guess 2" },
  { kind: "citizenship", after: 4, text: "Nationality after guess 4" },
  { kind: "field", after: 6, text: "Field after guess 6" },
];

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function renderFacts(list: HTMLElement, facts: Fact[], guesses: number, done: boolean) {
  list.replaceChildren();
  for (const fact of facts) list.append(el("li", undefined, fact.text));
  if (!done) {
    for (const lock of LOCKED) {
      if (guesses < lock.after && !facts.some((f) => f.kind === lock.kind)) {
        list.append(el("li", "locked", lock.text));
      }
    }
  }
  list.hidden = list.childElementCount === 0;
}

export function renderRows(list: HTMLElement, rows: Row[]) {
  list.replaceChildren();
  for (const row of rows) {
    const item = el("li");
    item.append(el("span", "name", row.name));
    const line = el("span", "line");
    row.phrases.forEach((p, i) => {
      if (i > 0) line.append(" · ");
      line.append(p.exact ? el("b", undefined, p.text) : el("span", undefined, p.text));
    });
    if (row.hint) {
      line.append(" · ");
      line.append(el("span", undefined, row.hint));
    }
    item.append(line);
    list.append(item);
  }
}

const matches = (row: Row): number => row.phrases.filter((p) => p.exact).length;

// Best first; among equals the more recent guess comes first.
export function orderRows(rows: Row[]): Row[] {
  return rows
    .map((row, i) => ({ row, i }))
    .sort((a, b) => matches(b.row) - matches(a.row) || b.i - a.i)
    .map(({ row }) => row);
}

export interface GuessLists {
  sorted: HTMLElement;
  sortedCaption: HTMLElement;
  latest: HTMLElement;
  latestCaption: HTMLElement;
}

export function renderGuesses(lists: GuessLists, rows: Row[], done: boolean) {
  const last = rows[rows.length - 1];
  const earlier = done ? rows : rows.slice(0, -1);
  renderRows(lists.sorted, orderRows(earlier));
  renderRows(lists.latest, done || !last ? [] : [last]);
  lists.sortedCaption.hidden = earlier.length < 2;
  lists.latestCaption.hidden = done || !last || earlier.length === 0;
}

export function renderRemaining(node: HTMLElement, left: number, done: boolean) {
  node.textContent = done ? "" : left === 1 ? "1 guess left" : `${left} guesses left`;
}

export function resultText(won: boolean, guesses: number, reveal: Reveal): string {
  if (won)
    return guesses === 1 ? `${reveal.label}, first guess.` : `${reveal.label}, in ${guesses}.`;
  return `Not this time. It was ${reveal.label}.`;
}

export function renderReveal(
  result: HTMLElement,
  desc: HTMLElement,
  credit: HTMLElement,
  reveal: Reveal,
  won: boolean,
  guesses: number,
) {
  result.textContent = resultText(won, guesses, reveal);
  desc.replaceChildren();
  if (reveal.description) desc.append(reveal.description + " ");
  if (reveal.wiki) {
    const a = el("a", undefined, "Wikipedia");
    a.href = `https://en.wikipedia.org/wiki/${encodeURIComponent(reveal.wiki)}`;
    a.rel = "noopener";
    a.target = "_blank";
    desc.append(a);
  }
  credit.replaceChildren("Photo: ");
  const page = el("a", undefined, reveal.image.artist || "unknown photographer");
  page.href = reveal.image.pageUrl;
  page.rel = "noopener";
  page.target = "_blank";
  credit.append(page, ", ");
  if (reveal.image.licenceUrl) {
    const lic = el("a", undefined, reveal.image.licence);
    lic.href = reveal.image.licenceUrl;
    lic.rel = "noopener";
    lic.target = "_blank";
    credit.append(lic);
  } else {
    credit.append(reveal.image.licence);
  }
  credit.append(", via Wikimedia Commons");
}

export function renderFigures(dl: HTMLElement, stats: Stats) {
  dl.replaceChildren();
  const rate = stats.played ? Math.round((100 * stats.wins) / stats.played) : 0;
  const pairs: [string, string][] = [
    ["Played", String(stats.played)],
    ["Won", `${rate}%`],
    ["Streak", String(stats.streak)],
    ["Best", String(stats.maxStreak)],
  ];
  for (const [label, value] of pairs) {
    const wrap = el("div");
    wrap.append(el("dt", undefined, label), el("dd", undefined, value));
    dl.append(wrap);
  }
}

export function renderDist(
  grid: HTMLElement,
  note: HTMLElement,
  mine: number[],
  all: StatsResponse | null,
  slot: number,
) {
  grid.replaceChildren();
  const allCounts = all?.counts ?? new Array<number>(MAX_GUESSES + 1).fill(0);
  const maxMine = Math.max(1, ...mine);
  const maxAll = Math.max(1, ...allCounts);
  for (let i = 0; i <= MAX_GUESSES; i++) {
    const label = el("span", "label", i === MAX_GUESSES ? "X" : String(i + 1));
    const bars = el("div", "bars");
    const my = el("div", mine[i] ? "bar some" : "bar");
    my.style.width = `${(100 * (mine[i] ?? 0)) / maxMine}%`;
    const everyone = el("div", allCounts[i] ? "bar all some" : "bar all");
    everyone.style.width = `${(100 * (allCounts[i] ?? 0)) / maxAll}%`;
    bars.append(my, everyone);
    const count = el("span", "count", String(mine[i] ?? 0));
    if (i === slot) {
      label.className = "label mine";
      count.className = "count mine";
    }
    grid.append(label, bars, count);
  }
  const total = all?.total ?? 0;
  note.textContent = total
    ? `Solid bars are your games; outlines are everyone today, ${total} so far.`
    : "Solid bars are your games.";
}

export function formatCountdown(ms: number): string {
  const totalMinutes = Math.ceil(ms / 60_000);
  const h = Math.floor(totalMinutes / 60);
  const m = totalMinutes % 60;
  return h > 0 ? `Next face in ${h}h ${m}m` : `Next face in ${m}m`;
}

export function formatIssue(n: number, date: Date): string {
  const text = date.toLocaleDateString("en-GB", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
  return `No. ${n} · ${text}`;
}
