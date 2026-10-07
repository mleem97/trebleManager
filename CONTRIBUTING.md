# Contributing — trebleManager

Repo: [https://github.com/mleem97/trebleManager](https://github.com/mleem97/trebleManager) · License: Apache-2.0 · Code of Conduct: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Workflow

1. Briefly describe the issue or idea (what/why).
2. Dev environment: `.devcontainer` is a symlink to the central Rust profile
   (`../.vendors/.devcontainers/rust` — do not replace with a local folder);
   shared caches via `source ../.vendors/env.sh`. New dependencies go into
   `../.vendors/` first (apt/pip/cargo manifests), never only into docs.
3. Branch off current `main`: `feat/<shortname>`, `fix/<shortname>`, `docs/<shortname>`.
4. Small, reviewable commits (Conventional Commits).
5. Before the PR: run the tests (see [QUICKSTART.md](QUICKSTART.md)), update docs (`README.md`, `INSTRUCTIONS.md`, `ARCHITECTURE.md`, `SECURITY.md`, `TROUBLESHOOTING.md`) and `CHANGELOG.md` (Unreleased).
6. PR with description, screenshots/logs for UI/behavior changes.

## Rules

- No secrets, no binaries unless necessary (firmware/Magisk/images via Releases or `data/` drops, never into git — see `.gitignore`).
- Do not commit generated artifacts (`logs/`, `backups/`, `*.zip`, `*.img`, `*.apk` …).
- PowerShell stays 5.1-compatible (no `?:`, no `??`, no `$PSStyle`); UI strings stay bilingual via `L "en" "de"`.
- No mock/fake success paths in production code (mocks only in `tests/`).
- Do NOT report security topics as issues, report them via [SECURITY.md](SECURITY.md) instead.

---

<a id="de"></a>
## Deutsch — Beitragen

Repo: [https://github.com/mleem97/trebleManager](https://github.com/mleem97/trebleManager) · Lizenz: Apache-2.0 · Verhaltenskodex: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Ablauf

1. Issue oder Idee kurz beschreiben (was/warum).
2. Dev-Umgebung: `.devcontainer` ist ein Symlink aufs zentrale Rust-Profil
   (`../.vendors/.devcontainers/rust` — nicht durch lokalen Ordner ersetzen);
   geteilte Caches via `source ../.vendors/env.sh`. Neue Abhängigkeiten zuerst
   nach `../.vendors/` (apt/pip/cargo-Manifeste), niemals nur in Docs.
3. Von aktuellem `main` abzweigen: `feat/<kürzel>`, `fix/<kürzel>`, `docs/<kürzel>`.
4. Kleine, reviewbare Commits (Conventional Commits).
5. Vor dem PR: Tests laufen lassen (siehe [QUICKSTART.md](QUICKSTART.md)), Docs
   aktualisieren (`README.md`, `INSTRUCTIONS.md`, `ARCHITECTURE.md`,
   `SECURITY.md`, `TROUBLESHOOTING.md`) und `CHANGELOG.md` (Unreleased).
6. PR mit Beschreibung, Screenshots/Logs bei UI-/Verhaltensänderungen.

## Regeln

- Keine Secrets, keine Binaries außer nötig (Firmware/Magisk/Images via
  Releases oder `data/`-Drops, niemals ins Git — siehe `.gitignore`).
- Keine generierten Artefakte committen (`logs/`, `backups/`, `*.zip`,
  `*.img`, `*.apk` …).
- PowerShell bleibt 5.1-kompatibel (kein `?:`, kein `??`, kein `$PSStyle`);
  UI-Strings bleiben zweisprachig via `L "en" "de"`.
- Keine Mock-/Fake-Erfolgs-Pfade in Produktionscode (Mocks nur in `tests/`).
- Sicherheitsthemen NICHT als Issues melden, sondern via [SECURITY.md](SECURITY.md).
