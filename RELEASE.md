# Release Process — trebleManager

Releases are **immutable**: a published version is never modified, re-uploaded,
or force-pushed. Fixes always ship as a **new version + new release**.

## Branch + tag layout

- Development happens on `main` (or `dev` → `main`).
- Every release gets **both**:
  - branch `release/vX.Y.Z` (hotfixes: `release/vX.Y.Z-hfN`), pushed once, never rewritten
  - annotated tag `vX.Y.Z` on the same commit
- GitHub Releases attach the immutable artifacts (`trebleManager-vX.Y.Z.zip` +
  `.sha256`) to the tag. Old releases keep working because branches/tags never move.

Existing: `release/v2.1.0`, `release/v2.2.0`, `release/v2.4.0` (+ tags).

## Cutting a release (maintainer)

1. Bump the version **everywhere** (single logical version):
   `VERSION`, `scripts/Treble-Toolkit.ps1` (`$TTVersion`),
   `scripts/treble-toolkit.sh` (`TTVERSION`), README badge, `CHANGELOG.md` entry.
   The test suites assert all of them match — CI-red means the bump is incomplete.
2. Run both test suites green:
   `powershell -File tests/Test-Parsers.ps1`, `bash tests/test-parsers.sh`.
3. Commit on main, push.
4. Create + push branch and tag:
   `git branch release/vX.Y.Z && git tag -a vX.Y.Z -m "..." && git push origin release/vX.Y.Z vX.Y.Z`
5. Build the ZIP from the **tag checkout** (never from a dirty tree) + `.sha256`,
   attach both to a new GitHub Release for that tag. Never edit release assets afterwards.
6. If a released version needs a fix: new version (patch bump) → new branch/tag/release.
   Exceptional hotfix on an old line: `release/vX.Y.Z-hfN` branch + new tag, same rules.

## Version tests (enforced)

- `tests/Test-Parsers.ps1`: asserts PS1 `$TTVersion` == `VERSION` == README badge == CHANGELOG entry.
- `tests/test-parsers.sh`: asserts bash `TTVERSION` == `VERSION`.
