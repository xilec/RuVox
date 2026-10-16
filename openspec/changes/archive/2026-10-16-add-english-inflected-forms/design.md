# Design: add-english-inflected-forms

## Context

The English phase already resolves known terms from `IT_TERMS` before digraph
transliteration. Inflected forms of those terms were losing the curated base
reading because only exact dictionary matches were accepted.

## Decisions

- Use a conservative suffix list and require the full stem to exist in
  `IT_TERMS`. Whole-word exact matches retain precedence.
- Compose the suffix reading via the existing digraph transliterator rather
  than hard-coding suffix variants, keeping one source of phonetic behavior.
- Add a compiled-in `sync` base term so its inflected forms receive the same
  curated reading as the base.
- Treat composed forms as known (not unknown words), matching dictionary hits.
- Lock the changed readings through focused unit tests and regenerated canonical baseline/golden expected outputs.
