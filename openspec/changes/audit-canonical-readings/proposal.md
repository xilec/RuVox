# Proposal: audit-canonical-readings

## Why

#282 asks for an audit of every canonical reading in `IT_TERMS`,
`MULTI_WORD_PHRASES`, acronym spellings, and the raw fallback. The audit ran
four dictionaries against each other — `IT_TERMS`, `abbreviations::AS_WORD`,
`abbreviations::SPECIAL_CASES`, and the `code_words` table of the code
identifier normalizer — and found two classes of defects:

- **Inconsistent readings for the same word.** `graphql` read "граф кью эль"
  in `IT_TERMS` but "граф кью эл" in `SPECIAL_CASES`; `html`, `xml`, and `sql`
  used the letter name "эль" while the spelling machinery (`letter_name`) and
  `code_words` use "эл"; `jpg` was "джэйпег" next to `jpeg` "джейпег";
  `hello`, `image`, and `node` read differently in code identifiers than in
  prose.
- **Wrong readings.** `png` was curated as "пнг", which no engine can
  pronounce as letters; `succ` was "сакс" (the reading of "sucks"/"sax"),
  not "сак".

The 155 pronunciation entries otherwise hold up: modern borrowings
("интерфейс", "сервис", "кластер", "нода") are intentional adaptations, not
aggressive phonetic rewrites, and the remaining semantic translations
(`package` → "пакет", `feature` → "фича", `module` → "модуль") stay because
the raw fallback is worse ("пакаге", "фитуре", "модюле") — they belong to the
transliteration-rules work tracked separately.

## What Changes

- Spell the letter Л as "эл" everywhere: `html` "эйч ти эм эл", `xml`
  "икс эм эл", `sql` "эс кью эл", `graphql` "граф кью эл". This matches
  `letter_name('l')` — the reading the spelling fallback already produces — and
  `SPECIAL_CASES`.
- Read `jpg` as "джейпег", the same reading as `jpeg` in `IT_TERMS` and
  `AS_WORD`.
- Read `png` as "пи эн джи" instead of the unpronounceable "пнг", like every
  other spelled-out file format (`csv`, `svg`, `pdf`).
- Read `succ` as "сак".
- Align the code-identifier readings with their prose counterparts: `hello`
  "хеллоу" (was "хелло"), `image` "имидж" (was "имадж"), `node` "нода" (was
  "ноуд").
- Guard the alignment with a test that fails when a key shared by two
  dictionaries has different readings.
- Update the golden fixtures, the transliteration baseline, and the spec
  scenarios; no rule changes.

## Impact

- Affected code: `IT_TERMS` in `normalizers/english.rs`, `code_words` in
  `normalizers/code.rs`.
- Affected specs: `text-pipeline` — the spelled-out scenario now reads
  HTML/SQL with "эл", and a new requirement pins cross-dictionary consistency.
- Not changed: the 155 genuine pronunciation entries, the 28 caps guards, the
  `living` composition guard, and the semantic translations listed above.
