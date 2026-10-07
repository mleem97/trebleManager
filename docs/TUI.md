# TUI — PowerShell menu + Rust terminal UI (ratatui)

Two different TUIs exist. The PowerShell TUI is the mature interactive frontend. The Rust `gsi-root-tui` binary (ratatui + crossterm, 21 hermetic tests) implements sidebar views (Dashboard, Device, Images, Root, Flash, Backup, Firmware, Diagnostics, Tools, Settings), a select wizard with typed-confirm display, and honest plan-only refusal for destructive paths.

Related: `docs/GUI.md`, `docs/CLI.md`, `docs/WORKFLOWS.md`.

## 1. Which TUI to use

- Daily use: PowerShell TUI (`scripts/Treble-Toolkit.ps1` with no args) or the bash port (`scripts/treble-toolkit.sh` with no args).
- `gsi-root-tui` (ratatui crate): `cargo run -p gsi-root-tui -- --help` shows usage/keys; arrows navigate, Enter selects, Esc back, Tab panel, `/` search, `?` shortcut dialog, `q` quit. Destructive flows render plans + exact CLI commands instead of executing.

## 2. Install / run (PowerShell TUI)

- Central entry: double-click `Start-TrebleToolkit.bat` (handles elevation, first-run setup, then TUI).
- Manual: `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1`.
- Linux: `./scripts/treble-toolkit.sh` (needs `adb`, `fastboot`; optional `unzip`, `curl`, `zip` or `python3` fallback).
- Online without files: `Run-FromGitHub.bat` (Windows) or `run-from-github.sh` (Linux); both fetch the full release ZIP SHA-verified and relaunch the same TUI.
- First run offers Setup (ADB/fastboot into user PATH, optional scrcpy, writes `data/config.json`). Missing global tools block the main menu with a fix path; they never fail later silently.

## 3. Layout

- Header (`Show-TTHeader`): title, TUI plus PS version, `Mode` (ANDROID/FASTBOOT/NONE), Admin YES/NO, `Profile` (for example `VTR-L29`), `OS` class, `ROM` label, state chip (`READY` or `WARNING`), log dir.
- Menu (`Show-TTMenu`): numbered options with highlight bar, hint line, status/bootkeys/tools/logs/wizard screens reachable from the main and sub menus.
- Step screens: Detect, Analyze, Firmware, Download, Extract, Export, Patch, Backup, Flash, Flash System, TWRP, Root Methods, Compat, Validate, Verify, Restore, Reinstall, Wipe, Persist, Devices, Tools, Logs, Goals, Resume, Wizard, Help, Status.
- Status screen states unknowns explicitly (`(no analysis yet)`, `UNKNOWN/NOT DETECTED (run analysis)`).

## 4. Full keymap (verified in `Show-TTMenu`)

| Key | Action |
|---|---|
| UpArrow | Move selection up (wraps) |
| DownArrow | Move selection down (wraps) |
| Enter | Confirm selection; in prompts: keep current / go back where labeled (`Enter keeps it`, `Enter = back`, `Enter = skip`) |
| Escape | Back / abort prompt (menu returns -1; multi-step prompts return null) |
| 1-9 | Direct option pick by number (only while a menu is shown) |
| Number + Enter | Numbered lists (ROM pick, target image, device pick): type number, confirm with Enter |
| Y / N style confirms | Typed words, not single keys: `FLASH` + `YES` for flash/system-flash, `RESTORE` + `YES` for restore, `W` + double confirm for wipe |
| Ctrl+C | Abort running download bar (documented on the progress line) |
| C / D / W in place | Context letters where shown: `[C] change system`, `[D] download firmware`, `[W] wipe first` |

Typed paths/URLs are pasted at prompts (firmware URL, UPDATE.APP path, ROM path, APK path, GSI image path); Enter alone skips/aborts where labeled.

## 5. Wizard

Order: Detect, Analyze (asks installed ROM: stock / supported / other, saved to `data/installed-rom.txt`), goal in plain words, plan with visible `[SKIP]`s, run.

- Stock path: Firmware, Extract, Patch, Backup, Flash (`FLASH`+`YES`), reboot with Vol-Up + Power until logo, Verify (`uid=0`).
- Custom-ROM path: ROM package, Recovery export, Patch, Backup, Flash, Verify. Stock firmware steps show as `[SKIP]`; the patch base must come from that ROM package, never from stock.
- Alternatives in the same menu: root-method priority (Magisk preferred), TWRP path (shared-slot warning), ROM/GSI install (verified profiles only), reinstall/wipe flows.

## 6. Safety

- Startup preflight gate blocks the menu when adb/fastboot are missing.
- Per-step gates (`Test-StepGate`): prerequisites checked before each step; multi-device requires explicit `ANDROID_SERIAL` target.
- 9-point flash readiness (`Test-TTFlashReadiness`) must pass; `DO NOT FLASH` otherwise.
- Backup (`backups/<MODEL>/recovery_ramdisk/<stamp>/` with `original.img`, `metadata.json`, SHA-256/512) is mandatory before flash.
- `--yes` / unattended never bypasses gates; it only documents consent for an already validated run.
- Failure flow is controlled: diagnostic ZIP, restore, abort; state persists in `logs/workflow-state.json` with resume.

## 7. On `--dry-run`

No `--dry-run` flag exists in the PowerShell CLI, the bash CLI, the Rust CLI, or the Rust TUI (verified by search). Dry-run equivalents that do exist:

- Planner: `workflow --goal <id>` (or TUI goal select) prints ordered steps with live gate results before executing.
- `validate` and `status`/`preflight`/`recon` are read-only checks.
- GUI pages are read-only previews and refuse live runs.

---

<a id="de"></a>
## Deutsch — TUI (Kurzspiegel)

Es gibt zwei TUIs: die reife PowerShell-TUI (Alltagsgebrauch) und den Rust-Stub `gsi-root-tui` (nur eine Zeile: `TUI not yet wired`).

- Start: `Start-TrebleToolkit.bat` (zentral), manuell per `Treble-Toolkit.ps1` ohne Args, Linux per `treble-toolkit.sh`; online via `Run-FromGitHub`-Loader (volles ZIP, SHA-geprüft).
- Layout: Header (Modus, Admin, Profil, OS, ROM, State), Menü mit Highlight, Step-Screens, Status mit explizitem UNKNOWN.
- Tasten (verifiziert): Hoch/Runter (Wrap), Enter (wählen/behalten/zurück laut Label), Esc (zurück/Abbruch), 1-9 (Direktwahl), Nummer+Enter (Listen), getippte Worte `FLASH`+`YES` / `RESTORE`+`YES`, kontextuelle Buchstaben C/D/W, Strg+C (Download-Abbruch). Pfade/URLs werden eingefügt; Enter allein skipt laut Label.
- Wizard: Detect, Analyze (mit ROM-Frage, gespeichert), Ziel in Alltagssprache, Plan mit `[SKIP]`s, Run. Stock-Weg vs. Custom-ROM-Weg (Patch-Basis aus ROM-Paket, nie Stock).
- Safety: Preflight blockiert Menü, Step-Gates, `ANDROID_SERIAL` bei Multi-Device, 9-Punkt-Readiness, Backup-Pflicht, `--yes` umgeht nie Gates, Failure-Flow Diagnose/Restore/Abbruch mit `workflow-state.json`.
- `--dry-run` existiert in keiner CLI/TUI; Ersatz sind Planner-Vorschau, read-only Checks (`validate`, `status`, `preflight`, `recon`) und lesende GUI-Seiten.
