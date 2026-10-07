# gsi-root — spec coverage status

Single source of truth for what the spec demands vs. what exists.
Levels: `DONE` (implemented + tested) · `PARTIAL` (works, gaps noted) ·
`EXPERIMENTAL` (refuses honestly) · `PLANNED` (not started).

## Core / CLI

| Spec § | Area | State | Notes |
|---|---|---|---|
| 1, 6 | Binary `gsi-root` (win/linux/mac) | PARTIAL | CLI works; GUI pages live, file pickers/progress pending |
| 4 | Bootstrap launchers stay | DONE | `irm`/`curl`/BAT untouched in trebleManager |
| 5 | Cross-platform paths | DONE | `gsi-config`, no hardcoded homes |
| 10–11 | Workflow engine (typed steps) | DONE | plan/run/list; destructive need `--yes` + device layer |
| 12 | Image engine (raw/sparse/gzip) | PARTIAL | detect+parse work; writers later |
| 13 | RootEngine trait | DONE | resolve/plan; apply refused until POC |
| 18–23 | Updater (check/download/install/rollback) | PARTIAL | SHA-256 verified; **signatures: NOT checked** (no key infra — reported, never faked) |
| 24 | Installers (.deb/.dmg/NSIS) | PLANNED | portable archives via trebleManager releases today |
| 25–26 | Scripts stay as launchers | DONE | nothing deleted; migration rule documented |
| 28 | Full CLI tree | PARTIAL | analyze/inspect/workflow/update/config/install/version/device/adb/fastboot/tools; profiles planned |
| 29 | CLI/GUI parity | PARTIAL | core pages both sides; file pickers/progress pending |
| 30–32 | Logging/progress/errors | PARTIAL | ProgressEvent enum + honest errors; `tracing` later |
| 33 | Analyzer | PARTIAL | container/sparse today; SAR/AVB/SELinux later |
| 34–35 | Pipeline + manifest | PARTIAL | Manifest struct exists; patch refuses |
| 36–37 | Safety + backups | DONE | by construction (refuse-first, hash-verify, `.bak`) |
| 38 | Tests | DONE | unit per crate (dry run, no device); golden/hardware pending POC |
| 42 | Offline behavior | DONE | all local commands offline; only download/update-check need net |
| 47 | VERIFIED/DOCUMENTED/EXPERIMENTAL | DONE | Maturity enum enforced in resolve/plan |
| 48 | P10 + Lineage 20 target | EXPERIMENTAL | profile static facts VERIFIED; root path refused until POC |

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

## GUI (Slint + Lucide)

PARTIAL (was: PLANNED). Dashboard/GSI/Updates/Logs/Settings pages live,
Lucide icons vendored (ISC, currentColor→light transform documented in
`assets/` note below), wired to core (analyze real, patch refusal real,
update check real). Remaining: file picker dialogs, progress bars on long
ops, ADB/Fastboot pages (blocked on device layer), Settings persistence.

---

<a id="de"></a>
## Deutsch — Spec-Abdeckung

Single Source of Truth: was die Spec verlangt vs. was existiert.
Stufen: `DONE` (implementiert + getestet) · `PARTIAL` (geht, Lücken notiert) ·
`EXPERIMENTAL` (lehnt ehrlich ab) · `PLANNED` (nicht begonnen).

## Core / CLI

| Spec § | Bereich | Stand | Notizen |
|---|---|---|---|
| 1, 6 | Binary `gsi-root` (win/linux/mac) | PARTIAL | CLI geht; GUI-Seiten live, File-Picker/Progress offen |
| 4 | Bootstrap-Launcher bleiben | DONE | `irm`/`curl`/BAT in trebleManager unangetastet |
| 5 | Cross-Platform-Pfade | DONE | `gsi-config`, keine hardcodierten Homes |
| 10–11 | Workflow-Engine (typisierte Steps) | DONE | Plan/Run/List; destruktiv braucht `--yes` + Device-Layer |
| 12 | Image-Engine (raw/sparse/gzip) | PARTIAL | Detect+Parse geht; Writer später |
| 13 | RootEngine-Trait | DONE | Resolve/Plan; Apply abgelehnt bis POC |
| 18–23 | Updater (Check/Download/Install/Rollback) | PARTIAL | SHA-256 verifiziert; **Signaturen NICHT geprüft** (keine Key-Infra — gemeldet, nie gefakt) |
| 24 | Installer (.deb/.dmg/NSIS) | PLANNED | portable Archive via trebleManager-Releases heute |
| 25–26 | Scripts bleiben Launcher | DONE | nichts gelöscht; Migrations-Regel dokumentiert |
| 28 | Voller CLI-Baum | PARTIAL | analyze/inspect/workflow/update/config/install/version/device/adb/fastboot/tools; Profile geplant |
| 29 | CLI/GUI-Parität | PARTIAL | Core-Seiten beidseitig; File-Picker/Progress offen |
| 30–32 | Logging/Progress/Fehler | PARTIAL | ProgressEvent-Enum + ehrliche Fehler; `tracing` später |
| 33 | Analyzer | PARTIAL | Container/Sparse heute; SAR/AVB/SELinux später |
| 34–35 | Pipeline + Manifest | PARTIAL | Manifest-Struct existiert; Patch abgelehnt |
| 36–37 | Safety + Backups | DONE | per Konstruktion (erst ablehnen, Hash-Verify, `.bak`) |
| 38 | Tests | DONE | Units pro Crate (Dry Run Tests, ohne Gerät); Golden/Hardware bis POC offen |
| 42 | Offline-Verhalten | DONE | alle lokalen Commands offline; nur Download/Update-Check braucht Netz |
| 47 | VERIFIED/DOCUMENTED/EXPERIMENTAL | DONE | Maturity-Enum in Resolve/Plan erzwungen |
| 48 | P10 + Lineage-20-Ziel | EXPERIMENTAL | Profil-Fakten VERIFIED; Root-Weg abgelehnt bis POC |

## Test-Level (Namensregel)

- **Dry Run Tests** — Tests und Ergebnisse ohne Flashen oder Geräte-Berührung.
- **Anleitungen** stehen als normale Befehls-/GUI-Anweisungen da, nicht als Dry Run.
- **Hardware-Tests** — brauchen das physische Gerät, immer explizit, nie Teil
  eines Dry Runs. P10 remote gehalten — Hardware-Tests als koordinierte
  Remote-Sessions, Ergebnisse nach `VERIFICATION-*.md`.

## Device-Layer

- Slot-State (`magisk`/`twrp`/`stock`, gleiche `workflow-state.json`-Form wie
  Scripts): DONE in `gsi-device` (Read/Write-Roundtrip getestet,
  Script-Dateien parsen).
- Switch-Planung (One-Tap-Matrix + Shared-Slot-Warnungen): DONE
  (`device switch` printet Plan).
- Switch-*Ausführung* (nativer Fastboot-Flash): PLANNED (Phase 8).
- Gemanagtes Tool-Binding (`gsi-tool`): DONE. `device detect|info`,
  `adb devices`, `fastboot devices|getvar`, `tools` laufen gegen echte
  System-Binaries (verifiziert live, read-only). Native Protokolle: Phase 8.

## GUI (Slint + Lucide)

PARTIAL. Dashboard/GSI/Updates/Logs/Settings live, Lucide-Icons vendored
(ISC), am Core verdrahtet (Analyze echt, Patch-Absage echt, Update-Check
echt). Offen: File-Picker, Progress-Bars, ADB/Fastboot-Seiten, Settings-
Persistenz.

- Slot state (`magisk`/`twrp`/`stock`, same `workflow-state.json` shape as
  the scripts): DONE in `gsi-device` (read/write round-trip tested,
  script-written files parse).
- Switch planning (one-tap matrix + shared-slot warnings): DONE
  (`device switch` prints the plan).
- Switch *execution* (native fastboot flash): PLANNED (Phase 8).
  Scripts remain the executor until then — single source of planning truth
  is already Rust.
- Managed tool binding (`gsi-tool`: locate/version/run/timeout, no shell):
  DONE. `device detect|info`, `adb devices`, `fastboot devices|getvar`,
  `tools` execute against real system binaries (verified against live
  `adb`/`fastboot` + a real attached Android device, read-only).
  Native ADB/fastboot protocols: PLANNED (Phase 8).

## GUI (Slint + Lucide)

PARTIAL (was: PLANNED). Dashboard/GSI/Updates/Logs/Settings pages live,
Lucide icons vendored (ISC, currentColor→light transform documented in
`assets/` note below), wired to core (analyze real, patch refusal real,
update check real). Remaining: file picker dialogs, progress bars on long
ops, ADB/Fastboot pages (blocked on device layer), Settings persistence.
