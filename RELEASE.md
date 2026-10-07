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
   `powershell -File .\tests\Test-Parsers.ps1`, `bash tests/test-parsers.sh`.
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

---

<a id="de"></a>
## Deutsch — Release-Prozess

Releases sind **unveränderlich**: Eine publizierte Version wird niemals
modifiziert, neu hochgeladen oder force-gepusht. Fixes erscheinen immer als
**neue Version + neues Release**.

## Branch- + Tag-Layout

- Entwicklung läuft auf `main` (oder `dev` → `main`).
- Jedes Release bekommt **beides**:
  - Branch `release/vX.Y.Z` (Hotfixes: `release/vX.Y.Z-hfN`), einmal gepusht, nie umgeschrieben
  - annotierten Tag `vX.Y.Z` auf demselben Commit
- GitHub Releases hängen die unveränderlichen Artefakte
  (`trebleManager-vX.Y.Z.zip` + `.sha256`) an den Tag. Alte Releases laufen
  weiter, weil Branches/Tags sich nie bewegen.

## Release schneiden (Maintainer)

1. Version **überall** bumpen (eine logische Version):
   `VERSION`, `scripts/Treble-Toolkit.ps1` (`$TTVersion`),
   `scripts/treble-toolkit.sh` (`TTVERSION`), README-Badge,
   `CHANGELOG.md`-Eintrag. Die Test-Suiten prüfen, dass alles matcht —
   CI-rot heißt unvollständiger Bump.
2. Beide Test-Suiten grün laufen lassen:
   `powershell -File .\tests\Test-Parsers.ps1`, `bash tests/test-parsers.sh`.
3. Auf main committen, pushen.
4. Branch und Tag erstellen + pushen:
   `git branch release/vX.Y.Z && git tag -a vX.Y.Z -m "..." && git push origin release/vX.Y.Z vX.Y.Z`
5. ZIP aus dem **Tag-Checkout** bauen (nie aus dirty tree) + `.sha256`,
   beides an ein neues GitHub Release für den Tag hängen. Release-Assets
   danach niemals anfassen.
6. Braucht eine releaste Version einen Fix: neue Version (Patch-Bump) →
   neuer Branch/Tag/Release. Ausnahme-Hotfix auf alter Linie:
   `release/vX.Y.Z-hfN`-Branch + neuer Tag, gleiche Regeln.

## Versions-Tests (erzwungen)

- `tests/Test-Parsers.ps1`: prüft PS1 `$TTVersion` == `VERSION` == README-Badge == CHANGELOG-Eintrag.
- `tests/test-parsers.sh`: prüft Bash `TTVERSION` == `VERSION`.
