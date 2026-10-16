# Design: add-english-normalization-layering

## Decisions

- Keep the implementation unchanged. This change captures behavior already
  shipped and locks the public contract for future dictionary work.
- Preserve exact-match precedence: multi-word phrases, `IT_TERMS`,
  abbreviations, `AS_WORD`, and finally transliteration.
- Keep the soft-c pre-pass inside the fallback because it is part of the
  phonetic behavior for unknown words, not a separate resolution layer.
