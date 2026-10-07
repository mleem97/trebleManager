# UI Architecture — one core, three frontends

Frontends share one service/domain core. No frontend owns logic. Unproven work refuses honestly instead of faking success.

Related: `docs/GUI.md`, `docs/TUI.md`, `docs/CLI.md`, `docs/WORKFLOWS.md`.

## 1. Diagram (ASCII)

```text
PowerShell TUI/CLI ──┐
Slint GUI ───────────┼──> gsi-app (facade, stub) ──> domain crates ──> infra
Rust gsi-root CLI ───┘         │                        │                 │
Rust TUI (stub) ───────────────┘                        │                 │
                                                        │                 │
domain: gsi-workflow (typed steps)                      │                 │
        gsi-gates (preflight/gates/compat/patch-base)   │                 │
        gsi-registry (YAML/JSON matrix)                 │                 │
        gsi-state (workflow-state/installed-rom/slot)   │                 │
        gsi-root-core (Maturity/engines/plans)          │                 │
        gsi-device (parse/slot plan)                    │                 │
        gsi-image/gsi-android (format/props parse)      │                 │
        gsi-archive/gsi-fs (archives/ext4 read-only)    │                 │
        gsi-diag (logs/backup/diagnostic planning)      │                 │
        gsi-update (release/download/verify)            │                 │
        gsi-config (dirs/discovery/policy)              │                 │
        gsi-i18n (en/de table)                          │                 │
        gsi-edl (MSM8953 builders/refusing transport)   │                 │
                                                        │                 │
infra:  gsi-exec (injected Executor + confirm tokens) ─┘                 │
        gsi-tool (managed adb/fastboot/extractor call) ──────────────────┘
scripts reference: scripts/Treble-Toolkit.ps1 + treble-toolkit.sh stay the
reference implementation until native coverage spans a full flow.
```

`gsi-app` implements the shared application service layer (AppState, AppEvent/LogBus with serial masking, SafetyConfirm, plus Device/Image/Root/Flash/Backup/Firmware/Diagnostic/Tool/Update/Settings service facades delegating 1:1 to the domain crates). GUI/TUI/CLI build on it; destructive calls require caller-collected SafetyConfirm.

## 2. Crate map (21 workspace members, purpose from each `lib.rs` header)

- `gsi-image`: container/image detection (gzip, Android sparse, raw). Pure parsing, no shell-outs.
- `gsi-android`: `build.prop`/`default.prop` strict `key=value` parse; unknown keys stay unknown.
- `gsi-root-core`: pipeline orchestration (profiles, engines, plans, manifests) plus `Maturity` discipline.
- `gsi-root-cli`: thin CLI over core (`analyze`/`inspect`/`workflow`/`config` work; `patch`/`verify` EXPERIMENTAL-refused; no args starts GUI note).
- `gsi-config`: platform-correct dirs (override, then OS user location) plus image finders, tool-link rules, config JSON, compat resolution, image policy.
- `gsi-workflow`: typed native workflows (plan once, run everywhere); destructive steps refuse without explicit confirmation.
- `gsi-update`: GitHub release check/download/verify/install/rollback; SHA-256 verified, signatures `NotChecked` until keys exist; Magisk/ROM fetch helpers.
- `gsi-fs`: read-only ext4 inspection (superblock, extents, dirs, symlinks, files, xattrs); verified against a 2.8 GB LineageOS GSI via `debugfs`.
- `gsi-device`: `recovery_ramdisk` slot tracking plus switch planning (TWRP/Magisk share one slot); flashing needs Phase-8 layer and is refused here.
- `gsi-archive`: gzip/xz decompress, tar/zip list/extract, UPDATE.APP probe (detect only); zip-bomb cap.
- `gsi-registry`: compatibility registry access (model family strict, submodel/region/EMUI WARN, firmware advisory only).
- `gsi-state`: persistence (`installed-rom.txt` token, `workflow-state.json` with goal/steps/slot/last_root/updated, path helpers, goal table, plan struct).
- `gsi-gates`: pure gates/rules (step gates, flash readiness aggregation, preflight subset, patch base, firmware compat, failure outcomes, root-method priority, baseline normalize, target partition, OS classify, admin core).
- `gsi-diag`: diagnostic/backup/export/validation/dump planning plus ZIP assembly; device paths return honest refusals naming the missing live step.
- `gsi-i18n`: en/de table mirroring the PS1 `L` helper (German only when UI language is German; du-form; missing de falls back to en; unknown key returns `missing message`).
- `gsi-exec`: device execution with injected executors via managed `gsi-tool`; all destructive work needs double confirm tokens and is refused before any call without them.
- `gsi-edl`: MSM8953 EDL plus FRP/bootloader flows; owner-repair only (ownership proof plus two confirms); pure builders plus refusing live transport.
- `gsi-app`: application service facade over domain crates (stub; GUI/TUI/CLI build on it once filled in).
- `gsi-root-tui`: ratatui terminal entry (stub message today).
- `gsi-tool`: managed external-tool binding (lookup in PATH plus known dirs, version probe, timeout-guarded run, captured output); never shell scripts.
- `gsi-root-gui`: Slint GUI over the same core API (read-only pages plus honest EXPERIMENTAL refusals; refresh closures never touch network/device/subprocess).

## 3. State / events

- `logs/workflow-state.json`: `goal`, `steps`, `slot {occupant, detail, timestamp}`, `last_root`, `updated`; other keys preserved. Read/write via `gsi-state`; same shape in scripts and Rust.
- `data/installed-rom.txt`: single trimmed token (`stock`, `rom:<label>`, other).
- `gsi-workflow::ProgressEvent`: `Started`, `Progress {current, total}`, `Message`, `Warning`, `Completed`. `StepOutcome`: `Done`, `Skipped(reason)`, `RefusedExperimental(reason)`, `Failed(reason)`.
- `gsi-exec::GoalConfirms {first, second}`: empty token counts as missing; whole-goal gate before destructive dispatch.
- Logs: `logs/toolkit-<stamp>.log` per run; TUI 200-line preview; diagnostic ZIP (10 files) via `diagnostic [--anonymize]`; GUI Logs page lists names plus sizes, never content dumps.

## 4. No-parallel-logic rule

- `gsi-workflow` is the single definition of steps/plans. GUI pages, CLI dispatch, and scripts render or run those definitions; they do not reimplement checks.
- PORTING.md scores 258 script functions against 444 Rust `pub fn` at 100 percent; TUI plumbing entries are `n/a` by design (GUI nav plus CLI dispatch replace them); elevation/PATH mutation stay OS-side with explicit plans (`path-plan`, `link-plan`, `install-plan`) instead of silent mutation.
- PowerShell plus bash stay the reference until native modules cover a full flow with fallback wrappers; CLI command names stay stable across migration.

## 5. Honesty system

`Maturity`: `Verified` (proven on hardware), `Documented` (from authoritative docs, not proven here), `Experimental` (implemented to try, outcome unknown), `Unsupported` (known not to work / out of scope).

- Unknown is never compatible. Missing tools/devices/values render as `not found` / `MISSING` / `unknown`, never guessed.
- `patch`/`verify` (Rust) and unproven GUI actions return `EXPERIMENTAL — refused` with the POC pointer; exit 3 in Rust CLI.
- Signatures report `NotChecked` until key infrastructure exists; unverified downloads refuse staging.
- `Command not allowed` from Huawei fastboot is UNKNOWN (never LOCKED).
- Patched-equals-stock is a hard fail (no fake patch); foreign/renamed images are refused; no auto unlock/wipe/format and no force/verity-disable flags exist.

## 6. Testing strategy

- `cargo test`: unit plus fixture plus golden tests per crate (parsers, verdict matrix, goal plans, slot JSON roundtrip, preflight/report rendering, stub Executor runs, `StubTransport`/`RefusingTransport` for EDL, `MAX_OUTPUT` zip-bomb guard).
- `tests/Test-Parsers.ps1` plus `tests/test-parsers.sh`: parser/firmware/OS/hash/root/URL/magic suites, self-bootstrapping from the full ZIP, run with zero device.
- Live runs are not unit tests: stub-tested paths note `live run = FIELD-EVIDENCE`; `gsi-fs` cross-checks against `debugfs` in `gsi-root/docs/VERIFICATION-lineage20-20251021.md`.
- JSON fixtures: registry YAML plus generated JSON mirrors stay the source of truth; `load_roms`/`rom_options` filter `broken` from selectable UI.
- GUI refresh closures are total (never panic) and hermetic in tests (temp dirs only; no network/device/subprocess).

---

<a id="de"></a>
## Deutsch — UI-Architektur (Kurzspiegel)

Ein Core, drei Frontends; kein Frontend besitzt Logik; Unbewiesenes wird ehrlich abgelehnt.

- Diagramm oben: PS-TUI/CLI, Slint-GUI, Rust-CLI (plus Rust-TUI-Stub) auf `gsi-app`-Fassade (Stub) zu Domain-Crates zu Infra (`gsi-exec` mit injiziertem Executor plus Confirm-Tokens, `gsi-tool` als gemanagtes adb/fastboot-Binding). Scripts bleiben Referenz bis native Abdeckung einen vollen Flow trägt.
- Crate-Map (21 Member, Zwecke oben je `lib.rs`-Header).
- State/Events: `logs/workflow-state.json` (Goal/Steps/Slot/Last-Root/Updated), `data/installed-rom.txt`, `ProgressEvent`/`StepOutcome`, `GoalConfirms`, Logs plus 10-Datei-Diagnose-ZIP.
- No-Parallel-Logic: `gsi-workflow` als einzige Step/Plan-Definition; GUI/CLI/Scripts rendern oder führen nur aus. PORTING: 258 Script-Funktionen gegen 444 Rust-`pub fn`, 100 Prozent; TUI-Plumbing `n/a` by design; Elevation/PATH-Mutation nur als explizite Pläne.
- Ehrlichkeit: `Verified` / `Documented` / `Experimental` / `Unsupported`; unbekannt nie kompatibel; `EXPERIMENTAL — refused` (Rust-Exit 3); Signaturen `NotChecked`; `Command not allowed` UNKNOWN; gepatcht-gleich-Stock hart FAIL; keine Auto-Unlock/Wipe/Format- oder Force-Flags.
- Tests: `cargo test` (Unit/Fixture/Golden, Stub-Executor, EDL-Stub-Transports), `Test-Parsers.ps1`/`test-parsers.sh` ohne Gerät, Live nur via `FIELD-EVIDENCE.md`, `gsi-fs`-Cross-Check via `VERIFICATION-*.md`, Registry als Source of Truth, GUI-Refresh total und hermetisch.
