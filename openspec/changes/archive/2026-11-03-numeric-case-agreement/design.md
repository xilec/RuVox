## Context

The numeric span layer computes the case of a number from the preposition that
precedes it (`context_case`), and until now only the numeral consumed that case.
Units, currencies, percentages and scale words carried nominative, singular
genitive and plural genitive forms; the dative and prepositional fell back to the
plural genitive, which produced "к пяти килограммов" and "при пяти процентов".

## Decisions

**Derive the dative and prepositional forms instead of extending every table.**
The result is regular for the unit, currency, percentage and scale tables: the
dative plural is the genitive plural stem plus "ам"/"ям", the prepositional plural
is the same stem plus "ах"/"ях", and the singular dative/prepositional follow from
the singular genitive ending. Derivation keeps one table per noun class and avoids
four forms per entry; explicitly irregular entries keep an override, indeclinable
nouns (евро) and fixed units (эм, рем) reuse their single form.

**Rejected: adding four forms to every entry.** It would have meant editing every
unit, currency and scale row, with a large chance of inconsistency between the
tables and no gain for regular nouns.

**Rejected: falling back to the genitive for noun-bearing constructs.** It avoids
the mixed case but keeps "к пять килограммов", which is equally audible. The
numeral-only path stays for dates and ordinal suffixes, where the noun is a month
name or a head word handled by other rules.

**Fractions.** An adjectival fraction ("одна вторая") is a noun phrase, so the
unit after it must be singular genitive; a simple fraction ("полтора") declines
the unit with the surrounding case. This keeps "около 1/2 кг" →
"около одной второй килограмма" and "к 1,5 кг" → "к полутора килограммам".

**Magnitude guard.** Dotted groups are numbers too: above the cardinal limit or
on a parse error they read digit by digit, the same rule the Numbers requirement
already states for standalone integers, so no spec text changes for that part.

## Risks

- Derivation can produce a wrong form for an irregular noun; the override and the
  indeclinable fallback limit the damage, and the corpus pins the common units.
- Sandhi (о → об before a vowel) stays outside the layer; "о одном" is left for a
  later prosody pass.
