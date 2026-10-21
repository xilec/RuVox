# Design: prune-it-terms

## Decisions

- **The `IT_TERMS` contract is pronunciation only.** A curated entry exists to
  correct a wrong transliteration, not to translate. This is the rule the
  change locks with tests and with the spec.
- **No-ops are identified by comparing the curated reading with
  `transliterate_simple` of the same word.** 66 entries matched, but the check
  has to cover every path that reads the entry, not only lowercase: the
  all-caps branch spells a token out unless it is curated, so for 27 of the 66
  the entry is the only thing that keeps "TEST", "DATA", or "APP" readable.
  Those stay as caps guards with a comment. Removal is
  safe only if the word's reading and the readings of its inflected forms do
  not change, so the audit was done in two steps: first the removed words
  themselves, then every generated inflection (`s`, `es`, `ies`, `ed`, `ing`,
  `er`, `ers`, `able`, `ment`, `ments`, `ation`, `ally`) compared by both
  resolution paths. Every form produced the same Cyrillic before and after,
  because composition of a no-op stem is by definition the transliteration of
  the stem plus the transliteration of the suffix.
- **One more kept entry: `living`.** The entry is not redundant — it
  shadows the inflection composition that would otherwise read the curated
  `live` stem (/laɪv/) plus "-ing" as "лайвинг". The word is /ˈlɪvɪŋ/ →
  "ливинг". The audit therefore treats "curated == raw" as a signal, not as a
  proof of redundancy: an entry that blocks a wrong composition stays, with a
  comment saying why.
- **`method` follows the pronunciation, not the issue's class A.** The review
  classified it as a pronunciation fix, but "th" → "с" ("месод") is closer to
  the English word than the curated "метод", so the entry is removed. This is
  the one place where the manual pass overrode the issue text.
- **No new transliteration rules here.** The remaining 155 entries are
  pronunciation fixes; converting them to rules (silent "e", "magic e", vowel
  digraphs, soft "c"/"g", intervocalic "s") is a separate transliteration
  change. Their targets stay in `IT_TERMS` until that change exists, so no
  reading regresses in the meantime. The rules work is tracked separately and
  includes the known bad targets `package` → "пэкэдж" and `feature` → "фиче"
  from the manual pass.

## Rejected alternatives

- **Remove every no-op mechanically.** Loses `living`, which is a guard, not
  duplication.
- **Translate the semantic entries instead of removing them.** Contradicts the
  review decision: the listener should hear the English word.
- **Do the rules conversion in the same change.** A phonics rewrite touches
  hundreds of readings and needs its own calibration and listening pass; mixing
  it with pure removals makes both harder to verify.
