# Contributing — trebleManager

Repo: [https://github.com/mleem97/trebleManager](https://github.com/mleem97/trebleManager) · License: Apache-2.0 · Code of Conduct: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Workflow

1. Briefly describe the issue or idea (what/why).
2. Dev environment: `.devcontainer` is a symlink to the central Rust profile
   (`../.vendors/.devcontainers/rust` — do not replace with a local folder);
   shared caches via `source ../.vendors/env.sh`. New dependencies go into
   `../.vendors/` first (apt/pip/cargo manifests), never only into docs.
3. Branch off current `main`: `feat/<shortname>`, `fix/<shortname>`, `docs/<shortname>`.
3. Small, reviewable commits (Conventional Commits).
4. Before the PR: run the tests (see [QUICKSTART.md](QUICKSTART.md)), update docs (`README.md`, `INSTRUCTIONS.md`, `ARCHITECTURE.md`, `SECURITY.md`, `TROUBLESHOOTING.md`) and `CHANGELOG.md` (Unreleased).
5. PR with description, screenshots/logs for UI/behavior changes.

## Rules

- No secrets, no binaries unless necessary (firmware/Magisk/images via Releases or `data/` drops, never into git — see `.gitignore`).
- Do not commit generated artifacts (`logs/`, `backups/`, `*.zip`, `*.img`, `*.apk` …).
- PowerShell stays 5.1-compatible (no `?:`, no `??`, no `$PSStyle`); UI strings stay bilingual via `L "en" "de"`.
- No mock/fake success paths in production code (mocks only in `tests/`).
- Do NOT report security topics as issues, report them via [SECURITY.md](SECURITY.md) instead.
