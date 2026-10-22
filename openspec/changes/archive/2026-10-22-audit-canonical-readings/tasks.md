# Tasks: audit-canonical-readings

## 1. Audit

- [x] 1.1 Compare `IT_TERMS` (183), `AS_WORD` (20), `SPECIAL_CASES` (4), and
      `code_words` (328) key by key: 11 keys are shared between `IT_TERMS` and
      the abbreviation tables, 44 between `IT_TERMS` and `code_words`.
- [x] 1.2 Classify every divergence: 4 acronyms used `эль` instead of the
      `letter_name` value `эл`, `jpg` disagreed with `jpeg`, `hello`, `image`,
      and `node` read differently in identifiers, `png` was unpronounceable,
      `succ` carried a spurious final `с`.
- [x] 1.3 Review the remaining 155 pronunciation entries and document the
      intentional adaptations (standard borrowings) and the deferred semantic
      translations (`package`, `feature`, `module`).

## 2. Implementation

- [x] 2.1 Fix the acronym readings in `IT_TERMS`: `html`, `xml`, `sql`,
      `graphql` use `эл`; `jpg` reads `джейпег`; `png` reads `пи эн джи`;
      `succ` reads `сак`.
- [x] 2.2 Align `code_words` with prose: `hello` → `хеллоу`, `image` →
      `имидж`, `node` → `нод`.
- [x] 2.3 Add a cross-dictionary consistency test over the shared keys, and pin
      the corrected readings in both the prose and identifier paths.

## 3. Fixtures and docs

- [x] 3.1 Regenerate the transliteration baseline and update the golden
      fixtures that quote the changed readings.
- [x] 3.2 Add the `CHANGELOG.md` entry and archive this change.
- [x] 3.3 Manual listening pass over the corrected words: the letter name `эл`, `джейпег`, `пи эн джи`, `сак`, `хеллоу`/`имидж`/`нод`, and the unchanged control words all sound right.
