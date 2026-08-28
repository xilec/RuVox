# Tasks: fix-html-boundary-detection

## 1. Detection

- [x] 1.1 `src/lib/detectFormat.ts`: replace the ≥3 tag-fragment count with
      the boundary rule (trimmed of whitespace and zero-width chars — starts
      with a tag AND ends with a tag → `html`); drop
      `HTML_MIN_TAG_FRAGMENTS`; update the module doc comment.

## 2. Tests

- [x] 2.1 `src/lib/detectFormat.test.ts`: rework the fragment cases —
      markup fragment (starts+ends with tags) → `html`; bare tag pair
      `<b>жирным</b>` → `html`; trim incl. zero-width chars; changelog-style
      prose with `<type>(<module>): <desc>` / `<UnlistenFn>` → `markdown`;
      starts-with-tag-but-not-ends (`<T> get_user_data()`, unclosed
      `<p>…`) → non-html; existing prose/stray-fragment plain cases pinned.

## 3. Validation

- [x] 3.1 `nix develop -c pnpm typecheck`, `nix develop -c pnpm test:unit`
      and `nix develop -c pnpm lint` green.
