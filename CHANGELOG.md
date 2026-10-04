# Changelog

## [0.3.0](https://github.com/sean-reid/whom/compare/whom-v0.2.0...whom-v0.3.0) (2026-10-04)


### Features

* **grade:** compare names by region and say every phrase in plain words ([#48](https://github.com/sean-reid/whom/issues/48)) ([f90782b](https://github.com/sean-reid/whom/commit/f90782b1b42bd1d60cb254e5b4750b5370d6e0f8))
* **pipeline:** give every name a region, continent, and plurality share ([#47](https://github.com/sean-reid/whom/issues/47)) ([0934d57](https://github.com/sean-reid/whom/commit/0934d578e2a67f0a49358fdae955ea9d5b606b21))
* put the input above the guesses and rank every guess with the last one marked ([#46](https://github.com/sean-reid/whom/issues/46)) ([c9632e9](https://github.com/sean-reid/whom/commit/c9632e9408c78133baad92f7cb4b2af8b58c73cc))


### Bug Fixes

* **pipeline:** accept only the name the person goes by ([#44](https://github.com/sean-reid/whom/issues/44)) ([04e0c62](https://github.com/sean-reid/whom/commit/04e0c6230bc55a746dd6dd6345d616115c795bd8)), closes [#43](https://github.com/sean-reid/whom/issues/43)
* **pipeline:** map the historical states the first region run left out ([#49](https://github.com/sean-reid/whom/issues/49)) ([ed255f0](https://github.com/sean-reid/whom/commit/ed255f0adb50b128947778a8edc4c33768f6b80e))

## [0.2.0](https://github.com/sean-reid/whom/compare/whom-v0.1.0...whom-v0.2.0) (2026-10-04)


### Features

* suggest the most carried names first ([#38](https://github.com/sean-reid/whom/issues/38)) ([41ece3c](https://github.com/sean-reid/whom/commit/41ece3cf15fd46474fd1d580b86f9b737eb0494c))


### Bug Fixes

* fetch the name list under its version so a rebuilt list shows at once ([#39](https://github.com/sean-reid/whom/issues/39)) ([a6f71f7](https://github.com/sean-reid/whom/commit/a6f71f76f257236995f4cc8099ba906dd7dcd74e))
* **pipeline:** capitalise lowercase Wikidata name labels ([#42](https://github.com/sean-reid/whom/issues/42)) ([aeabe75](https://github.com/sean-reid/whom/commit/aeabe750ad00e0c6f68c6cc6255e3bff965447ac))
* **pipeline:** keep cased displays for pool-only names and rank merged names by holders ([#41](https://github.com/sean-reid/whom/issues/41)) ([4952d4a](https://github.com/sean-reid/whom/commit/4952d4a421bfbcbf42fe8620f9d25a91a5384508))
* **pipeline:** keep name forms to one word and revalidate stored ones on load ([#37](https://github.com/sean-reid/whom/issues/37)) ([733fbc3](https://github.com/sean-reid/whom/commit/733fbc32d0f3839f72019ed37f57f5abd50affc5))
* **pipeline:** POST QLever queries, pace them at 3 s, and back off longer on 429 ([#34](https://github.com/sean-reid/whom/issues/34)) ([07d9b14](https://github.com/sean-reid/whom/commit/07d9b143b263d7705d6f37fc8daa0538bb9a37f9))

## 0.1.0 (2026-10-03)


### Features

* grade era and shared root as sixth and seventh phrases ([#7](https://github.com/sean-reid/whom/issues/7)) ([cab42cf](https://github.com/sean-reid/whom/commit/cab42cfba3a7d2ef596f3c4fb6ee75cf751007f5))
* **pipeline:** era, sameAs, and shortOf on every name ([#9](https://github.com/sean-reid/whom/issues/9)) ([487265a](https://github.com/sean-reid/whom/commit/487265abbb4a15b367826a3ab1ae106ca4254da9))
* **pipeline:** Rust tool that builds the face pool and name graph ([#3](https://github.com/sean-reid/whom/issues/3)) ([0e96b27](https://github.com/sean-reid/whom/commit/0e96b2716dd903568210c3c93da337ebafa7907b))
* play the daily game in the browser ([#6](https://github.com/sean-reid/whom/issues/6)) ([5ac7b30](https://github.com/sean-reid/whom/commit/5ac7b300b61aac0c1bc6e59282e9c6591a8d112c))
* shell page, worker health route, and day numbering ([c0662b1](https://github.com/sean-reid/whom/commit/c0662b109672f3551d226eebf8ecda2cb7a5532d))
* sort earlier guesses by matches and pin the last guess by the input ([#10](https://github.com/sean-reid/whom/issues/10)) ([bcb8c82](https://github.com/sean-reid/whom/commit/bcb8c825315818dde8e49449b49f075e5a67b1c5))
* worker api with signed tokens, puzzle durable object, and r2 data ([#4](https://github.com/sean-reid/whom/issues/4)) ([3b222f8](https://github.com/sean-reid/whom/commit/3b222f8bac3f2b88fb194be0cdf9c95095f3d26f))


### Bug Fixes

* **client:** subset the fonts, inline the stylesheet, read guesses aloud, and retry a failed fetch ([#30](https://github.com/sean-reid/whom/issues/30)) ([84abbbe](https://github.com/sean-reid/whom/commit/84abbbe1325db83ef54a224134a0cc3202d86a9b))
* fade in only the newest guess row ([#8](https://github.com/sean-reid/whom/issues/8)) ([bf48469](https://github.com/sean-reid/whom/commit/bf4846907c046b1b0a87685cf973cf9b7d8ffc5e))
* launch today, harden the api, and land the audit's quick wins ([#11](https://github.com/sean-reid/whom/issues/11)) ([4e022cb](https://github.com/sean-reid/whom/commit/4e022cb3ce34590886245c0f440e2ac2df0acb25))
* **pipeline:** hold back the manifests after a failed run and land the audit fixes ([#29](https://github.com/sean-reid/whom/issues/29)) ([6a6770a](https://github.com/sean-reid/whom/commit/6a6770a4d42857c15b6eabdbc885c1d6a08b28b5))
* **worker:** count once per client, pin days in one object, reload on a miss, and fix grading ([#28](https://github.com/sean-reid/whom/issues/28)) ([2b60cb0](https://github.com/sean-reid/whom/commit/2b60cb06ba65c61012802ce3bc2ef792e52718d3))
