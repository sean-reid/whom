# WHOM?

A daily game at [whom.dwainosaur.com](https://whom.dwainosaur.com). One famous face, eight guesses at the first name. Every wrong guess grades your name against the answer on length, first letter, language, sound, and how common it is, and every second guess reveals a fact about the person.

Faces and facts come from Wikidata and Wikimedia Commons: people born since 1900 with forty or more Wikipedia articles and a freely licensed portrait. Names are compared through the languages Wikidata lists for each given name, Double Metaphone codes, and how many Wikidata people share the name.

## Development

```sh
npm install
npm run dev        # Worker and site at http://127.0.0.1:8787
npm test           # unit tests
npm run test:e2e   # Playwright against a local build
```

`npm run lint`, `npm run format:check`, `npm run typecheck`, `npm run build`, and `npm run size` are what CI runs. The size check fails when first-load JavaScript passes 25 KB gzipped.

## How it runs

One Cloudflare Worker serves the static build and the `/api` routes. Guesses are graded on the Worker against a signed game token; the answer never reaches the browser before the game ends. A Durable Object per puzzle pins the day's person the first time it is served and keeps the result distribution. Portraits and the name graph live in R2 and are produced by the Rust tool under `tools/pipeline`, which runs quarterly from GitHub Actions and fetches from Commons one request at a time. The Worker reads two secrets, `SESSION_SECRET` for tokens and `PUZZLE_SEED` for the schedule, set with `wrangler secret put`.

Deploys happen from GitHub Actions on every green merge to `main`. Releases are cut by release-please from the conventional commit history.
