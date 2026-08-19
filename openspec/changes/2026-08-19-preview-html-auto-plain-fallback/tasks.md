# Tasks: preview-html-auto-plain-fallback

- [x] `AddAction::preview` carries `plainFallback`; AppShell stores/resets
      it per dialog opening
- [x] Reject arm of `handlePreviewSynthesize`: auto-detected `html` +
      carried fallback → ingest plain; explicit choice → red error
- [x] Unit tests: fallback carried/omitted; whitespace-trim cells in the
      preview branch
- [x] Delta spec `preview-dialog`; `openspec validate` green
