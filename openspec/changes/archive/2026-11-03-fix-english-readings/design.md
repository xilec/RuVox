## Context

The narrated route speaks English words through two curated tables: the numeric
layer's word table (cardinals and constructs) and the general `IT_TERMS` table
plus spelling rules for prose. The manual listen pass found six readings that
sound wrong: the rules read "the" as «те» and "and" as «анд», and the numeric
tables carry «сри», «сёти», «хандред», «персент».

## Goals / Non-Goals

**Goals**

- The six words read the way the manual pass expects, on every path that emits
  them (a bare cardinal, a construct, ordinary prose).
- The native route and every mapping stay byte-identical.

**Non-Goals**

- Touching the spelling rules: only curated data changes.
- Rereading other words; the pass accepted the rest.

## Decisions

- **Readings only, Latin words untouched.** Every table entry carries a
  `(latin, reading)` pair, and only the reading half changes. The native route
  emits the Latin half, and the numeric layer maps the reading to the source
  digits, so fixture text and `CharMapping` do not move.
- **A `FUNCTION_WORDS` table instead of `IT_TERMS` entries.** `IT_TERMS` is the
  technical vocabulary with an explicit "irregular reading the rules cannot
  express" rule; the article and the conjunction are grammar words, so they get
  their own table, consulted before `IT_TERMS` and before inflection, so
  "THE"/"And" and inflected forms resolve consistently.
- **The ordinal derivatives move with their cardinals.** "thirtieth" reads
  «фётис» and "hundredth" «хандрэдс» so the ordinal does not contradict the
  cardinal it derives from.
- **Fixtures are regenerated, not hand-edited.** Only the Silero cases that
  contain an affected word change; the writer produces both text and map.

## Risks

- **A curated word shadows a rule that other words rely on.** The table holds
  exactly two entries and the rules stay intact for every other word.
- **Stale spec examples.** The three scenarios that pin a reading are updated in
  this change's delta; the archived `read-english-constructs` change keeps its
  historical text.

## Migration

No migration: the tables are compiled in, profiles and stored texts are
unchanged.
