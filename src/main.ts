import { MAX_GUESSES } from "../shared/api.ts";
import {
  EPOCH,
  localIsoDate,
  msUntilLocalMidnight,
  parseIsoDate,
  puzzleNumber,
} from "../shared/day.ts";
import { ApiError, getNames, getPuzzle, getStats, postGuess } from "./api.ts";
import {
  announceRow,
  formatCountdown,
  formatDate,
  formatIssue,
  renderDist,
  renderFacts,
  renderFigures,
  renderRemaining,
  renderReveal,
  renderGuesses,
} from "./render.ts";
import { copyText, shareLine } from "./share.ts";
import {
  hasSeenHelp,
  loadGame,
  loadStats,
  markHelpSeen,
  recordResult,
  saveGame,
  type Game,
  type Stats,
} from "./state.ts";
import { Suggestions } from "./suggest.ts";

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

const issue = $("issue");
const portrait = $<HTMLImageElement>("portrait");
const notice = $("notice");
const announce = $("announce");
const facts = $("facts");
const lists = {
  sorted: $("guesses"),
  sortedCaption: $("sorted-caption"),
  latest: $("latest"),
  latestCaption: $("latest-caption"),
};
const form = $<HTMLFormElement>("guess-form");
const input = $<HTMLInputElement>("guess");
const submit = $<HTMLButtonElement>("submit");
const suggest = $("suggest");
const remaining = $("remaining");
const end = $("end");
const help = $<HTMLDialogElement>("help");

let game: Game | null = null;
let stats: Stats = loadStats();
const suggestions = new Suggestions();
let active = -1;

function say(text: string) {
  notice.textContent = text;
  if (text) notice.scrollIntoView({ block: "nearest" });
}

function render() {
  if (!game) return;
  renderFacts(facts, game.facts, game.rows.length, game.done);
  renderGuesses(lists, game.rows, game.done);
  renderRemaining(remaining, MAX_GUESSES - game.rows.length, game.done);
  form.hidden = game.done;
  if (game.done && game.reveal) {
    portrait.alt = game.reveal.label;
    renderReveal($("result"), $("desc"), $("credit"), game.reveal, game.won, game.rows.length);
    renderFigures($("figures"), stats);
    const slot = game.won ? game.rows.length - 1 : MAX_GUESSES;
    renderDist($("dist"), $("dist-note"), stats.dist, null, slot);
    end.hidden = false;
    void getStats(game.n)
      .then((all) => renderDist($("dist"), $("dist-note"), stats.dist, all, slot))
      .catch(() => undefined);
    tickCountdown(localIsoDate());
  }
}

function tickCountdown(today: string) {
  const node = $("next");
  const update = () => {
    if (localIsoDate() !== today) {
      window.location.reload();
      return;
    }
    node.textContent = formatCountdown(msUntilLocalMidnight());
  };
  update();
  window.setInterval(update, 60_000);
}

function closeSuggestions() {
  suggest.hidden = true;
  input.removeAttribute("aria-activedescendant");
  suggest.replaceChildren();
  input.setAttribute("aria-expanded", "false");
  active = -1;
}

function showSuggestions() {
  const matches = suggestions.match(input.value);
  suggest.replaceChildren();
  if (matches.length === 0) {
    closeSuggestions();
    return;
  }
  matches.forEach((name, i) => {
    const li = document.createElement("li");
    li.setAttribute("role", "option");
    li.id = `suggest-${i}`;
    li.textContent = name;
    li.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      input.value = name;
      closeSuggestions();
      void guess();
    });
    suggest.append(li);
  });
  suggest.hidden = false;
  input.setAttribute("aria-expanded", "true");
  suggest.scrollIntoView({ block: "nearest" });
  active = -1;
}

function moveActive(delta: number) {
  const options = suggest.querySelectorAll<HTMLElement>("li");
  if (options.length === 0) return;
  active = (active + delta + options.length) % options.length;
  options.forEach((o, i) => o.setAttribute("aria-selected", String(i === active)));
  const current = options[active];
  if (current) input.setAttribute("aria-activedescendant", current.id);
}

async function loadNames() {
  if (suggestions.size > 0) return;
  try {
    const res = await getNames();
    suggestions.load(res.names);
    if (document.activeElement === input && input.value) showSuggestions();
  } catch {
    // Without the list the input still works; the Worker rejects unknown names.
  }
}

async function guess() {
  if (!game || game.done) return;
  const options = suggest.querySelectorAll<HTMLElement>("li");
  const picked = active >= 0 ? options[active]?.textContent : null;
  const name = (picked ?? input.value).trim();
  closeSuggestions();
  if (!name) return;
  submit.disabled = true;
  input.disabled = true;
  say("");
  try {
    const res = await postGuess(game.token, name);
    game.token = res.token;
    const row = { name, phrases: res.phrases, ...(res.hint ? { hint: res.hint } : {}) };
    game.rows.push(row);
    game.facts = res.facts;
    game.done = res.done;
    game.won = res.won;
    if (res.reveal) game.reveal = res.reveal;
    if (game.done && !game.recorded) {
      stats = recordResult(stats, game.n, game.won, game.rows.length);
      game.recorded = true;
    }
    saveGame(game);
    input.value = "";
    render();
    if (game.done) end.focus();
    else announce.textContent = announceRow(row, MAX_GUESSES - game.rows.length);
  } catch (err) {
    if (err instanceof ApiError && err.status === 422) {
      say(err.message === "already guessed" ? "You already tried that one." : "Not a name I know.");
    } else if (err instanceof ApiError && err.status === 429) {
      say("Slow down a little.");
    } else if (err instanceof ApiError && err.status === 409) {
      say("This game is over. Reload for today's result.");
    } else {
      const why = err instanceof ApiError ? `the server answered ${err.status}` : "no connection";
      say(`That guess did not get checked, ${why}. Try again.`);
    }
  } finally {
    submit.disabled = false;
    input.disabled = false;
    if (!game.done) input.focus();
  }
}

async function start() {
  const today = localIsoDate();
  const n = puzzleNumber(today);
  if (n === null || n < 1) {
    issue.textContent = "";
    say(`The first face arrives on ${formatDate(parseIsoDate(EPOCH) ?? 0)}.`);
    return;
  }
  issue.textContent = formatIssue(n, new Date());
  portrait.src = `/api/crop/${n}`;
  portrait.hidden = false;
  game = loadGame(n);
  if (!game) {
    try {
      const res = await getPuzzle(today);
      game = { n: res.n, token: res.token, rows: [], facts: [], done: false, won: false };
      saveGame(game);
    } catch {
      say("No face right now. Try again in a moment.");
      return;
    }
  }
  form.hidden = game.done;
  render();
  if (!hasSeenHelp()) {
    help.showModal();
    markHelpSeen();
  }
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  void guess();
});
input.addEventListener("focus", () => void loadNames());
input.addEventListener("input", showSuggestions);
input.addEventListener("blur", () => window.setTimeout(closeSuggestions, 100));
input.addEventListener("keydown", (event) => {
  if (suggest.hidden) return;
  if (event.key === "ArrowDown") {
    event.preventDefault();
    moveActive(1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    moveActive(-1);
  } else if (event.key === "Escape") {
    closeSuggestions();
  } else if (event.key === "Tab" && active >= 0) {
    event.preventDefault();
    input.value = suggest.querySelectorAll("li")[active]?.textContent ?? input.value;
    closeSuggestions();
  }
});
$("help-button").addEventListener("click", () => help.showModal());
$("share").addEventListener("click", async () => {
  if (!game) return;
  const text = shareLine(game.n, game.won, game.rows.length);
  const ok = await copyText(text);
  $("share-done").textContent = ok ? "Copied" : text;
});

void start();
