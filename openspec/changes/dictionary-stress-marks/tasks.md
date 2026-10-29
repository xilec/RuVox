# Tasks: dictionary stress marks

## 1. Contract and profiles

- [ ] 1.1 Document the `Stress` semantics on the type (what `Auto` and
  `Manual` mean for a renderer, and that the pipeline is the renderer).
  Verify: doc comment reads as a contract, no behavior claimed beyond it.
- [ ] 1.2 Set `Stress::Auto` for `SILERO_NATIVE_PROFILE`, keep `None` for the
  sidecar and Piper with a comment naming the probe (#311), and `Manual` for
  the test profile. Verify: the profile registry test asserts the values and
  explains each one.

## 2. Dictionary parsing and validation

- [ ] 2.1 Parse one `+` out of `to`: a mark must sit immediately before a
  Russian vowel, appear at most once, and never at the end. Expose the spoken
  form (mark stripped) and the verbatim form. Verify: unit tests for
  `М+осква`, `Москв+а`, `+Москва`, `М+оск+ва`, `Москва+`, and a vowel-less
  word.
- [ ] 2.2 Validate on save and on load from the file: a malformed mark is a
  named error, an entry from a hand-edited file is skipped with a warning like
  any other invalid entry. The length limit counts the spoken form. Verify:
  `validate_entry` tests plus a store test with a malformed hand-edited entry.

## 3. Pipeline rendering

- [ ] 3.1 Insert the verbatim `to` when the active profile declares stress
  support and the spoken form otherwise, inside the existing replacement
  region. Verify: pipeline tests for both branches on the same entry.
- [ ] 3.2 Add a profile-pinned golden fixture whose dictionary carries a mark,
  so the rendered and stripped forms are both pinned. Verify: `native_en/`
  fixture passes and the Silero-pinned negative case would fail if the gate
  regressed.

## 4. Editor

- [ ] 4.1 Help icon next to the spoken-form label: tooltip explains the `+`
  before a vowel and that only engines with stress support apply it. Verify:
  i18n keys exist in both locales and the modal renders the icon.
- [ ] 4.2 Inline validation of a malformed mark, mirroring the backend rules.
  Verify: `userDictionary` unit tests for the same six cases as 2.1.

## 5. Verification

- [ ] 5.1 `nix develop -c just test` and `just lint` green.
- [ ] 5.2 Manual pass: create an entry with a mark, preview the text, and hear
  the stress on the native engine; check that the sidecar and Piper read the
  word without the mark.
