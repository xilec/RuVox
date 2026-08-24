#!/usr/bin/env bash
# Guard: a tag pushed without bumping tauri.conf.json would make the freshly
# installed app OLDER than latest.json — an update prompt loop on every
# start. Fail fast, before the expensive build. Used by both release jobs
# (.github/workflows/release.yml).
set -euo pipefail

tag="${1:?usage: guard-tag-version.sh <tag>}"
conf_version=$(jq -r .version src-tauri/tauri.conf.json)
if [ "$conf_version" != "${tag#v}" ]; then
  echo "::error::tag $tag does not match tauri.conf.json version $conf_version — bump the version first"
  exit 1
fi
