#!/usr/bin/env bash
# Push main repo + wiki subrepo in sequence (step 4 of the release flow).
set -u
BRANCH="${1:-main}"
git push origin "$BRANCH" || exit 1
git -C wiki push origin master || exit 1
printf 'Pushed: main repo (%s) + wiki (master).\n' "$BRANCH"
