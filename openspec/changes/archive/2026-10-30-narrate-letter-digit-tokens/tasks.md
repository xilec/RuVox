## 1. Specification

- [x] 1.1 Proposal, design, delta spec and tasks for
  `narrate-letter-digit-tokens`.

## 2. Implementation

- [x] 2.1 Add the letter+digit pattern to the English phase and exclude
  mixed-case identifiers.
- [x] 2.2 Resolve a whole-token curated reading first, then split into letter
  and digit runs and use the existing owners.
- [x] 2.3 Annotate a native letter+digit token as a code-identifier span.
- [x] 2.4 Expose the curated `CODE_WORDS` reading for the split letter run.

## 3. Tests and fixtures

- [x] 3.1 Unit tests for narrated lowercase, curated, code-word and
  all-uppercase cases, plus the native annotation.
- [x] 3.2 Golden fixtures under `silero/` and `native_en/`.
- [x] 3.3 Invariant: a narrating `Charset::ModelAlphabet` profile leaves no
  ASCII Latin in the output.

## 4. Documentation and gates

- [x] 4.1 `CHANGELOG.md` entry (user-visible speech fix).
- [x] 4.2 `just test`, `just lint`, `openspec validate --all --strict`.
- [x] 4.3 Archive the change and sync the `text-pipeline` spec.
