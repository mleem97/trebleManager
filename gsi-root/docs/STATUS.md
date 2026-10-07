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
| 38 | Tests | DONE | unit per crate; golden/hardware pending POC |
| 42 | Offline behavior | DONE | all local commands offline; only download/update-check need net |
| 47 | VERIFIED/DOCUMENTED/EXPERIMENTAL | DONE | Maturity enum enforced in resolve/plan |
| 48 | P10 + Lineage 20 target | EXPERIMENTAL | profile static facts VERIFIED; root path refused until POC |

## GUI (Slint + Lucide)

PLANNED (Phase 9). No stub GUI exists — an honest refusal beats a fake window.
