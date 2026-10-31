## Context

The dictionary runs inside the narrating pipeline. A fragment routed `Native`
skips the built-in tables but still passes through the user-dictionary phase,
which is how a native prose run or a native identifier can be corrected
without transliterating it. A native fenced code block is different: it is
held behind a placeholder for every narrating phase and spliced back into the
finished text, so neither the built-ins nor the user dictionary see it.

## Decisions

### D1. Scope the guarantee by fragment kind, not by route

The guarantee "the user dictionary still applies to a native fragment" exists
so that a user can fix an engine's reading without losing native routing. It
only makes sense while the pipeline still renders the fragment as text. Scope
it explicitly to Latin prose runs and code identifiers, and name the native
code block as the exception.

Rejected: drop the guarantee and say "the dictionary applies only where the
pipeline narrates". That would lose the deliberate native-prose behavior from
#311 (dictionary corrections on top of native routing).

Rejected: keep the sentence and accept the contradiction. The pipeline would
then violate its own spec for every native code block.

### D2. No code change

The pipeline already implements this: `dictionary_does_not_rewrite_native_code`
covers the behavior, and the block never reaches the dictionary phase. The
change is a spec correction that keeps the archive honest.

## Risks

- The wording must not read as "the dictionary is disabled for a native
  profile": native prose and identifiers keep their dictionary application.
