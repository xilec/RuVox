## Why

#325 made a fenced code block a text fragment whose reading the profile can
route `Native` (TeraTTSv2 and Piper in the read mode). The `user-dictionary`
spec still says that "the user dictionary SHALL still apply to [a fragment
routed `Native`]", which now promises dictionary rewrites inside engine-read
code. Rewriting code would corrupt it (`forEach` → «форИч»), and a native block
never reaches the narrating phases by design.

## What Changes

- Scope the dictionary claim to native fragments the pipeline still renders as
  text — Latin prose runs and code identifiers.
- State that a native fenced code block bypasses the user dictionary along with
  the built-in tables, and add a scenario for it.

## Impact

- Affected specs: `user-dictionary`
- Affected code: none — the behavior already matches this clarification; #325
  keeps the block behind a placeholder through the dictionary phase.
- Docs: the README dictionary bullet already scopes the claim (#325).

## Non-goals

- Letting the dictionary rewrite native code.
- Changing dictionary precedence, scopes, forms or stress behavior.
