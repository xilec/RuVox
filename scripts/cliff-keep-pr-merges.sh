#!/usr/bin/env bash
# git-cliff commit_preprocessor: keep only PR merge commits.
#
# Why: in this repo every PR reaches main twice — the branch tip carries a
# plain conventional subject, the merge commit carries the PR title (same
# text plus a "(#N)" suffix, sometimes more refs). Message-based rules cannot
# separate them reliably (twins may reference issues as "(#N)" themselves),
# but topology can: the landing is always a two-parent merge commit, the twin
# never is. This script is wired from cliff.toml commit_preprocessors with
# pattern "(?s).*"; git-cliff feeds it the full commit message on stdin and
# exports $COMMIT_SHA.
#
# Output: the subject line for merge commits, empty otherwise. An emptied
# entry falls through to the catch-all skip at the end of cliff.toml's
# commit_parsers.
#
# Note: direct-to-main pushes (allowed by policy only for trivial docs/typo
# fixes) are single-parent and stay out of the notes — intended.
set -euo pipefail

# Consume stdin (the full commit message) even though the subject is re-read
# from git below — leaving it unread can SIGPIPE the generator.
cat > /dev/null
parents=$(git rev-list --parents -n 1 "${COMMIT_SHA:?}")
# rev-list prints "<sha> <parent1> [<parent2> ...]" on a single line.
if [ "$(printf '%s\n' "$parents" | wc -w)" -gt 2 ]; then
    # Emit the subject line only: merge-commit bodies carry the whole PR
    # description, which must not leak into the rendered bullets (subjects
    # that fail strict conventional parsing get no body split by git-cliff).
    #
    # Normalize the repo's multi-type form "feat(ui,tray),build(release): X"
    # to a strictly parseable "feat(ui,tray): X" (the extra ",type(scope)"
    # groups break git-cliff's conventional parser, leaving the raw prefixed
    # subject in the output). Anchored to the leading "type(scope)," group so
    # prose later in the subject can never match. The leading type/scope is
    # kept: commit_parsers group by it; git-cliff strips it during parsing.
    git show -s --format=%s "$COMMIT_SHA" \
        | sed -E 's/^([a-zA-Z]+\([^)]*\)),[a-zA-Z]+\([^)]*\):/\1:/'
fi
