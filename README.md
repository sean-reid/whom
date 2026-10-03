# WHOM?

A daily game at [whom.dwainosaur.com](https://whom.dwainosaur.com). One famous face, eight guesses at the first name. Every wrong guess grades your name against the answer on length, first letter, language, sound, how common it is, and when it was in fashion, and every second guess reveals a fact about the person.

Faces and facts come from Wikidata and Wikimedia Commons: people born since 1900 with forty or more Wikipedia articles and a freely licensed portrait. Names are compared through the languages Wikidata lists for each given name, Double Metaphone codes, how many Wikidata people share the name, the median birth year of those people, and which names Wikidata links as the same or as short forms.

## Development

```sh
npm install
npm run dev        # Worker and site at http://127.0.0.1:8787
npm test           # unit tests
npm run test:e2e   # Playwright against a local build
```

`npm run lint`, `npm run format:check`, `npm run typecheck`, `npm run build`, and `npm run size` are what CI runs. The size check fails when first-load JavaScript passes 25 KB gzipped.

## How it runs

One Cloudflare Worker serves the static build and the `/api` routes. Guesses are graded on the Worker against a signed game token; the answer never reaches the browser before the game ends. A Durable Object per puzzle pins the day's person the first time it is served and keeps the result distribution. Portraits and the name graph live in R2 and are produced by the Rust tool under `tools/pipeline`, which runs quarterly from GitHub Actions and fetches from Commons one request at a time. The Worker reads two secrets, `SESSION_SECRET` for tokens and `PUZZLE_SEED` for the schedule, which the deploy workflow copies from the Actions secrets of the same names on every deploy.

To run the pipeline locally: `cd tools/pipeline && cargo run --release -- run --sample 20 --out out/`. It scans QLever one page a second, fetches from Commons one request every 1.5 seconds, stops after three 429s in a row, and caps image fetches per run with `--max-fetch` (default 4000). It resumes from `out/state.json`, so a second run never refetches a person it has seen.

Deploys happen from GitHub Actions on every green merge to `main`. Releases are cut by release-please from the conventional commit history.
