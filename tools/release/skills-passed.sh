#!/usr/bin/env bash
# Where release.yml's skills gate passed (T1f, 11-07): the commit tools/release.sh takes as RAEA_SKILLS_PASSED, since a
# public checkout has no feed app to run the gate with. It's the skills_passed input when the workflow is run by hand,
# else the annotated tag's "skills-passed: <sha>" line; with neither, the release refuses. The commit is fetched so
# release.sh can compare the gate's inputs, and written to GITHUB_ENV (or printed, off Actions).
#   tools/release/skills-passed.sh TAG [SHA]
set -euo pipefail
tag=${1:?tag}
sha=${2:-}
if [ -z "$sha" ]; then
  git fetch -q --depth=1 origin "refs/tags/$tag:refs/tags/$tag" 2>/dev/null || true
  [ "$(git cat-file -t "refs/tags/$tag" 2>/dev/null)" = tag ] ||
    { echo "skills-passed: $tag isn't an annotated tag, so it can't say where the skills gate passed (tag with -a and a \"skills-passed: <sha>\" line, or run release.yml by hand with skills_passed)" >&2; exit 1; }
  sha=$(git for-each-ref --format='%(contents)' "refs/tags/$tag" | sed -nE 's/^skills-passed: *([0-9a-f]{40}) *$/\1/p' | head -1)
  [ -n "$sha" ] || { echo "skills-passed: $tag's message has no \"skills-passed: <sha>\" line (a full SHA where tools/skills.sh check passed with the feed app)" >&2; exit 1; }
fi
[[ $sha =~ ^[0-9a-f]{40}$ ]] || { echo "skills-passed: $sha isn't a full commit SHA" >&2; exit 1; }
git cat-file -e "$sha^{commit}" 2>/dev/null || git fetch -q --depth=1 origin "$sha"
echo "skills-passed: the gate passed at $sha"
if [ -n "${GITHUB_ENV:-}" ]; then echo "RAEA_SKILLS_PASSED=$sha" >> "$GITHUB_ENV"; else echo "RAEA_SKILLS_PASSED=$sha"; fi
