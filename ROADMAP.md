# Roadmap — trebleManager

Starting point: the working toolkit in this repo (TUI + CLI, device profiles,
compatibility registry, safety gate, Linux bash port). This plan extends it —
it does not rewrite it.

## 0. Done in v2.7.0 (orchestrator foundation)

- Startup preflight gate (tool registry, writable dirs, explicit device states)
- 8 workflow goals + planner with live gate results + persistent state + resume
- Controlled failure flow (diagnostic → restore → abort), JSON status API
- CLI: `preflight|recon|status|workflow|resume|root` (+ `--goal`, `--mode`)
- Multi-device target selection, unauthorized/offline as own states

## 1. Where we are

- Wizard/CLI: detect → analyze → firmware → download → extract → export →
  patch → backup → flash → verify, plus flash-system, twrp, restore,
  diagnostic, compat, root-methods, devices.
- Rules already enforced: no blind flashing, `DO NOT FLASH` gate, backup
  before flash, on-device Magisk patch only, `uid=0` as the only root proof,
  Huawei `Command not allowed` is UNKNOWN (never LOCKED).

## 2. Run modes (done: v2.5.0)

- `safe` (default): every irreversible step is confirmed interactively.
- `unattended` + `--yes`: auto-confirms a **fully validated** run.
  Safety gates still run — `--yes` never skips checks.
- `developer`: unlocks `dump-partitions|properties|vendor|logs` for ROM research.

## 3. Post-flash validation (done: v2.5.0)

`validate` (TUI Tools + CLI) runs read-only system/hardware checks
(boot, ADB, OS, SELinux, root, mounts, Wi-Fi, Bluetooth, battery, sensors)
and writes `logs/validation-<stamp>.json`. Next: per-check remediation hints.

## 4. Wizard as state machine (planned)

`UNKNOWN → DETECTED → ANALYZED → COMPATIBLE → BACKED_UP → IMAGE_PREPARED →
READY_TO_FLASH → FLASHED → BOOTED → VERIFIED`, with
`BOOT_FAILED → DIAGNOSTIC → RESTORE_REQUIRED` recovery. Persist step state so
interrupted runs resume instead of restarting.

## 5. Firmware as a repository (planned)

Per-package metadata (model, CUST, region, EMUI, Android, build, source, URL,
SHA-256/512, extraction status, known compatibility) under `data/firmware/`,
so inputs are reproducible artifacts, not loose downloads.

## 6. Core extraction (started v2.18.0, stepwise)

PowerShell stays the reference implementation (direct-from-GitHub run must
keep working). `core/treble_core` holds the first pure modules with
`cargo test` suites: `images` (boot/system detection), `fastboot` (flash
verdicts), `firmware` (URL checks), `roms` (labels/suggestions) + `ttcore`
CLI (`image-kind`, `flash-verdict`, `check-url`) for later wrapper use.
PS1/bash integration (calling `ttcore` with fallback to native code) comes
only after the module set covers a full flow. CLI command names stay stable
across the migration.

## 7. Standing rules (never change)

Unknown is never compatible. No automatic unlock/wipe/format, no force or
verity-disable flags, no foreign patched images, no fake success paths.
New version = new immutable release (see [RELEASE.md](RELEASE.md)).

## 8. Pre-Rust era (v2.1.0 – v2.17.x)

Releases up to and including v2.17.x are script-only (PowerShell 5.1+,
bash, BAT launchers) with no Rust dependency and no cargo requirement.
Their documented behavior (wizard steps, safety gates, registries, CLI,
diagnostics) is frozen and stays valid; PORTING.md scores them as the
migration baseline (0 % = script reference, not a defect). Rust code
(`core/`, `gsi-root/`) ships from v2.18.0 on, strictly additive — no
script behavior was removed or altered to accommodate it.

### Deutsch

Releases bis einschließlich v2.17.x sind script-only (PowerShell 5.1+,
bash, BAT-Launcher) ohne Rust-Abhängigkeit und ohne Cargo-Pflicht. Ihr
dokumentiertes Verhalten (Wizard-Steps, Safety-Gates, Registries, CLI,
Diagnostik) ist eingefroren und bleibt gültig; PORTING.md wertet sie als
Migrations-Basis (0 % = Script-Referenz, kein Mangel). Rust-Code (`core/`,
`gsi-root/`) liegt ab v2.18.0 bei, strikt additiv — kein Script-Verhalten
wurde dafür entfernt oder verändert.
