## Why

`user-dictionary` and `preview-dialog` each restate the source-token predicate
for the dictionary quick-add instead of referencing "Dictionary entry
semantics", and the two restatements disagree. The viewer requirement omits
the one-script clause, so `IPзамок` reads as valid there; the preview
requirement calls "punctuation" invalid, so `UTF-8` reads as invalid there.
The implemented gate (`isSingleSourceToken`) accepts `UTF-8`, rejects
`IPзамок`, and both entry points already use it. #348 has already synced the
viewer requirement, so the correction is a small follow-up change with
MODIFIED deltas.

## What Changes

- `user-dictionary` "Viewer quick-add": replace the inline token gloss with a
  reference to "Dictionary entry semantics", state the global prefill scope,
  and point menu placement at `viewer-copy-actions` "Viewer context menu"
  instead of restating the order.
- Add `UTF-8` (item offered) and `IPзамок` (item absent) scenarios to that
  requirement.
- `preview-dialog` "Quick-add to dictionary from preview": replace the
  "punctuation" wording with the same reference to "Dictionary entry
  semantics"; the action and hint semantics stay unchanged.
- No code change: the shared `isSingleSourceToken` gate and the Rust
  validator already implement the referenced predicate.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `user-dictionary`: "Viewer quick-add" references the canonical token
  predicate, states the initial scope, and adds the hyphenated and
  mixed-script scenarios; menu order is delegated to `viewer-copy-actions`.
- `preview-dialog`: "Quick-add to dictionary from preview" references the
  canonical token predicate instead of restating it.

## Impact

- Affected specs: `user-dictionary`, `preview-dialog`.
- Affected code: none — `src/lib/userDictionary.ts` (`isSingleSourceToken`)
  and the Rust `tts-normalize` validator already match the canonical gloss;
  their tests cover `UTF-8`, `IPзамок`, internal and malformed hyphens.
- No IPC, storage, UI or user-visible change; `CHANGELOG.md` stays untouched.

## Non-goals

- Changing the token predicate itself or the quick-add behavior.
- Touching the archived #348 change or its implementation.
- Rewording the "Dictionary entry semantics" gloss (it already matches the
  code).
