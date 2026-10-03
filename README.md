# WHOM?

A daily game at [whom.dwainosaur.com](https://whom.dwainosaur.com). One famous face, eight guesses at the first name. Every wrong guess grades your name against the answer on length, first letter, language, sound, how common it is, and when it was in fashion, says when the two are forms of one name or one is short for the other, and every second guess reveals a fact about the person.

Faces and facts come from Wikidata and Wikimedia Commons: people born since 1900 with forty or more Wikimedia sitelinks and a freely licensed portrait. Names are compared through the languages Wikidata lists for each given name, Double Metaphone codes, how many Wikidata people share the name, the median birth year of those people, and which names Wikidata links as the same or as short forms.

## Development

```sh
npm install
npm run dev        # builds, seeds a local R2 from tests/fixtures, serves at http://127.0.0.1:8787
npm test           # unit tests
npm run test:e2e   # Playwright against a local build
```

CI runs `npm run lint`, `npm run format:check`, `npm run typecheck`, `npm test`, `npm run build`, `npm run size`, `npm run audit`, `npm run e2e:install`, `npm run test:e2e`, and `cargo fmt --check`, `cargo clippy`, and `cargo test` under tools/pipeline. The size check fails when first-load JavaScript passes 25 KB gzipped.

## How it runs

One Cloudflare Worker serves the static build and the `/api` routes. Guesses are graded on the Worker against a signed game token; the answer never reaches the browser before the game ends. A Durable Object per puzzle pins the day's person the first time it is served and keeps the result distribution. The pool, the name graph, and the portrait crops live in R2 and are produced by the Rust tool under `tools/pipeline`, which runs quarterly from GitHub Actions and fetches from Commons one request at a time. The Worker reads two secrets, `SESSION_SECRET` for tokens and `PUZZLE_SEED` for the schedule, which the deploy workflow copies from the Actions secrets of the same names on every deploy.

To run the pipeline locally: `cd tools/pipeline && cargo run --release -- run --sample 20 --out out/`. It scans QLever one page a second, fetches from Commons one request every 1.5 seconds, stops after three 429s in a row, and caps image fetches per run with `--max-fetch` (default 10000). It resumes from `out/state.json`; a second run refetches only people whose fetch failed transiently. `upload` publishes the manifests only after a run has written `out/run-ok`; a failed run uploads its crops and nothing else.

Deploys happen from GitHub Actions on every green merge to `main`. Releases are cut by release-please from the conventional commit history.
