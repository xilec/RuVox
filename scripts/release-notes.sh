#!/usr/bin/env bash
# Extract the CHANGELOG.md section for this version so the draft release
# body is the human-curated notes, not GitHub's auto-generated diff summary
# (issue #94). The `## [Unreleased]` heading and deeper `###`/`####`
# headings never match the `## [<semver>]` pattern, so only the target
# version's block is captured (up to the next top-level `## [` heading).
# Shared by both release jobs (.github/workflows/release.yml) so whichever
# job finishes first writes identical notes into the same draft release.
set -euo pipefail

tag="${1:?usage: release-notes.sh <tag>}"
version="${tag#v}"
awk -v v="$version" '
  /^## \[/ {
    rest = substr($0, 5)
    cb = index(rest, "]")
    hdr = (cb > 0) ? substr(rest, 1, cb - 1) : rest
    capture = (hdr == v)
  }
  capture { print }
' CHANGELOG.md > release_notes.md

# Fallback when the tag has no CHANGELOG section yet (a tag pushed before
# the entry was added): keep a usable draft body instead of empty notes.
if [ ! -s release_notes.md ]; then
  echo "See CHANGELOG.md — no section for version $version yet." > release_notes.md
  echo "::warning::No CHANGELOG.md section for $version — using stub release notes"
fi
