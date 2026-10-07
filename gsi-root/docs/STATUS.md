# gsi-root — spec coverage status

Single source of truth for what the spec demands vs. what exists.
Levels: `DONE` (implemented + tested) · `PARTIAL` (works, gaps noted) ·
`EXPERIMENTAL` (refuses honestly) · `PLANNED` (not started).

## Core / CLI

| Spec § | Area | State | Notes |
|---|---|---|---|
| 1, 6 | Binary `gsi-root` (win/linux/mac) | PARTIAL | CLI works; GUI = Phase 9 |
| 4 | Bootstrap launchers stay | DONE | `irm`/`curl`/BAT untouched in trebleManager |
| 5 | Cross-platform paths | DONE | `gsi-config`, no hardcoded homes |
| 10–11 | Workflow engine (typed steps) | DONE | plan/run/list; destructive need `--yes` + device layer |
| 12 | Image engine (raw/sparse/gzip) | PARTIAL | detect+parse work; writers later |
| 13 | RootEngine trait | DONE | resolve/plan; apply refused until POC |
| 18–23 | Updater (check/download/install/rollback) | PARTIAL | SHA-256 verified; **signatures: NOT checked** (no key infra — reported, never faked) |
| 24 | Installers (.deb/.dmg/NSIS) | PLANNED | portable archives via trebleManager releases today |
| 25–26 | Scripts stay as launchers | DONE | nothing deleted; migration rule documented |
| 28 | Full CLI tree | PARTIAL | analyze/inspect/workflow/update/config/install/version; adb/fastboot/device/profile PLANNED |
| 29 | CLI/GUI parity | PLANNED | blocked on GUI (Phase 9) |
| 30–32 | Logging/progress/errors | PARTIAL | ProgressEvent enum + honest errors; `tracing` later |
| 33 | Analyzer | PARTIAL | container/sparse today; SAR/AVB/SELinux later |
| 34–35 | Pipeline + manifest | PARTIAL | Manifest struct exists; patch refuses |
| 36–37 | Safety + backups | DONE | by construction (refuse-first, hash-verify, `.bak`) |
| 38 | Tests | DONE | unit per crate (dry run, no device); golden/hardware pending POC |

## Test levels (naming rule)

- **Dry Run Tests** — tests and test results produced without flashing or
  touching any device: all `cargo test` suites, both
  `Test-Parsers`/`test-parsers` suites (simulated command outputs), static
  image analysis, export-refusal checks, and records like
  `VERIFICATION-*.md`. Safe anywhere.
- **Instructions / guides** are written as plain command/GUI instructions
  and are NOT labeled dry run — they describe actions to execute (some need
  a device, which the guide states where it matters).
- **Hardware tests** — need the physical device (flash, boot, root verify,
  cold boot, persistence). Always explicit, never part of a dry run.
  The P10 is currently held remotely (not on this machine) — hardware tests
  run as coordinated remote sessions, results land in `VERIFICATION-*.md`.
| 42 | Offline behavior | DONE | all local commands offline; only download/update-check need net |
| 47 | VERIFIED/DOCUMENTED/EXPERIMENTAL | DONE | Maturity enum enforced in resolve/plan |
| 48 | P10 + Lineage 20 target | EXPERIMENTAL | profile static facts VERIFIED; root path refused until POC |

## Device layer

- Slot state (`magisk`/`twrp`/`stock`, same `workflow-state.json` shape as
  the scripts): DONE in `gsi-device` (read/write round-trip tested,
  script-written files parse).
- Switch planning (one-tap matrix + shared-slot warnings): DONE
  (`device switch` prints the plan).
- Switch *execution* (native fastboot flash): PLANNED (Phase 8).
  Scripts remain the executor until then — single source of planning truth
  is already Rust.

## GUI (Slint + Lucide)

PARTIAL (was: PLANNED). Dashboard/GSI/Updates/Logs/Settings pages live,
Lucide icons vendored (ISC, currentColor→light transform documented in
`assets/` note below), wired to core (analyze real, patch refusal real,
update check real). Remaining: file picker dialogs, progress bars on long
ops, ADB/Fastboot pages (blocked on device layer), Settings persistence.
