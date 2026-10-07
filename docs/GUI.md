# GUI — Slint desktop app (`gsi-root-gui`)

The Rust GUI is the GUI-first frontend of the suite. Same core as CLI/TUI, no own logic. Most pages are read-only previews; device work is planned, never executed here.

Related: `docs/UI-ARCHITECTURE.md`, `docs/CLI.md`, `docs/WORKFLOWS.md`.

## 1. Install / run

- From a built workspace: `gsi-root` with no args or with `gui` starts the Slint window (`gsi-root-gui::run`). Source: `gsi-root/crates/gsi-root-cli/src/main.rs` dispatch (`"" | "gui"`).
- Windows online: `Run-FromGitHub.bat gui` fetches `gsi-root-windows-x86_64.zip` (SHA-checked) into `%LOCALAPPDATA%\gsi-root\bin` and starts the GUI.
- Linux online: `run-from-github.sh gui` fetches `gsi-root-linux-x86_64.tar.gz` into `~/.local/bin` and starts the GUI.
- Needs adb/fastboot on PATH (or in `tools/platform-tools`, `data/tools`) for Detect/Tools pages to show anything beyond "not found".

## 2. Windows / pages

Sidebar in `gsi-root/crates/gsi-root-gui/ui/app.slint` uses five labeled groups (Overview, Check, Install, Device, More) with a collapsible icon-only mode below 1100px window width. Center views (`ui/center_*.slint`: Device, Diagnostics, Images, Root, Magisk, Flash, Flash Progress, Backup incl. Create/Restore, Firmware incl. Download/Extract/Export, Tools, Updates, Logs incl. file list, Settings, Help) mount next to the classic pages; destructive actions stay read-only plans with typed-confirm requirements.

Check: Dashboard, GSI, Detect, Analyze Device, Tools, Updates, Logs, Settings.
Steps: Flash, Flash System, Wipe, Unlock, Verify, Preflight, Backup, Restore, Reinstall, Resume, Bootkeys, Extract, Download, Firmware, Export, Kernel, TWRP, Compat, Goals, Root Methods, Patch, Persist, System (romselect), Help, Status, Wizard.

Every step page has a Refresh button plus a read-only result view. Refresh closures gather local values only (installed ROM via `gsi-state`, registry profile via `gsi-registry`, dirs via `gsi-config`, log names via `gsi-diag`).

## 3. Dashboard

Shows on start (`page: "dashboard"`):

- Title `Dashboard`, device line `Huawei P10 (VTR-L09 / VTR-L29) — first hardware target. Root engine: EXPERIMENTAL until hardware POC.`
- Reference GSI line `lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed`.
- Rule line `Analyze works locally (no writes). Patch/verify refuse honestly until proven on hardware.`

## 4. Wizard flow

Wizard page is a read-only preview (`live::wizard_refresh`):

1. Reads installed ROM id (`stock`, `rom:<label>`, empty) and current goal from `logs/workflow-state.json`.
2. Resolves goal steps via `gsi-state::goal_steps` and renders them as text with `[SKIP]` marks where the ROM path skips stock steps.
3. Appends the patch-base note (`Your system decides the patch base`, see Patch page).
4. Live run is refused by design; execution stays in the script wizard or a future runner with device attached.

## 5. Safety model (typed confirms)

- Destructive Rust runners (`gsi-exec::run_safe_flash`, `run_system_flash`, `run_twrp_flash`, `run_restore`, `run_guided_wipe`) require two non-empty token strings. Tests use `GoalConfirms::new("FLASH", "YES")` for flash and `RESTORE` + `YES` for restore. Missing tokens are refused before any tool call.
- Script parity: TUI flash/restore require typing `FLASH` + `YES` (`RESTORE` + `YES`); CLI uses `--yes` as documented consent.
- GUI never collects those tokens and never calls the runners. Flash/Flash System/Wipe/Restore pages render plan previews with notes such as `Live flashing needs a device, backup and double confirm — refused without.`
- Unlock page is guidance only and states `Never executes by design`.

## 6. Honesty states

Maturity comes from `gsi-root-core::Maturity`: `Verified` (proven on hardware), `Documented` (from authoritative docs, not proven here), `Experimental` (implemented to try, outcome unknown), `Unsupported` (known not to work / out of scope).

- GSI Patch button calls `patch_text`, which returns `EXPERIMENTAL — refused` with the file path and `nothing written, nothing faked`.
- CLI `patch`/`verify` print `EXPERIMENTAL — refused` and exit 3.
- Analyze/Detect/Tools render measured values; anything unmeasurable renders as text such as `no devices (or no tools)`, `MISSING (native protocol: Phase 8)`, or `unknown, never guessed`.

## 7. File-path inputs (no native picker)

- Imports in `app.slint` are exactly `Button, LineEdit, TextEdit, ListView, ComboBox` from `std-widgets.slint`. There is no file dialog import and no picker control.
- GSI page: image path is typed into a `LineEdit` (`gsi-path`), hint shows an example `arm64_bgN` image path; Analyze/Patch buttons act on the typed string. Empty input returns `no file given`.
- Updates page: repo is typed into a `LineEdit` (`owner/repo` hint). Channel is a `ComboBox` (`Stable`, `Beta`, `Nightly`).
- All result views are read-only `TextEdit`.

## 8. Language

- Slint page strings are English. `gsi-i18n` holds an en/de table (core flow keys plus `ui.*` chrome keys, German in du-form), but `pages_flows.rs` notes bilingual strings as future wiring, so the GUI does not switch language today.
- Contrast: the PowerShell TUI is bilingual via the `L "en" "de"` helper and follows the system UI culture (English default, German on German systems).

## 9. Responsive / minimum sizes

Observed in `app.slint` only:

- Window `preferred-width: 1024px`, `preferred-height: 640px`, with `minimum-width: 1024px` / `minimum-height: 640px` (window never smaller than content).
- Sidebar 210px with icon-only collapsed mode below 1100px window width, nav rows 36px. Content padding 20px inside a ScrollView (scrolling allowed on small windows).

---

<a id="de"></a>
## Deutsch — GUI (Kurzspiegel)

Die Rust-GUI (`gsi-root-gui`, Slint) ist das GUI-first-Frontend, ohne eigene Logik. Seiten sind lesende Vorschauen; Gerätezugriff findet hier nicht statt.

- Start: `gsi-root` ohne Args oder mit `gui`; online via `Run-FromGitHub.bat gui` (Windows, `%LOCALAPPDATA%\gsi-root\bin`) bzw. `run-from-github.sh gui` (Linux, `~/.local/bin`).
- Navigation: fünf beschriftete Gruppen (Overview, Check, Install, Device, More) mit einklappbarem Icon-Modus unter 1100px; Center-Ansichten (Device, Diagnostik, Images, Root, Magisk, Flash inkl. Progress, Backup inkl. Create/Restore, Firmware inkl. Download/Extract/Export, Tools, Updates, Logs inkl. Dateiliste, Settings, Help) neben den klassischen Seiten.
- Dashboard: P10 als erstes Hardware-Target, Referenz-GSI `lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed`, Analyze lokal ohne Writes, Patch/Verify EXPERIMENTAL.
- Wizard: lesende Vorschau (installiertes ROM + Goal + Steps mit `[SKIP]s` + Patch-Basis-Hinweis); Live-Run abgelehnt.
- Safety: destruktive Runner brauchen zwei Token (`FLASH`+`YES`, `RESTORE`+`YES`); GUI sammelt keine Tokens und führt nichts aus. Unlock nur Anleitung.
- Ehrlichkeit: `Verified` / `Documented` / `Experimental` / `Unsupported`; Patch-Antwort `EXPERIMENTAL — refused`; Unbekanntes wird als unbekannt gerendert, nie geraten.
- Pfade: getippte `LineEdit`-Felder (kein nativer Dateidialog); Ergebnisse lesend.
- Sprache: Slint-Texte Englisch; `gsi-i18n` en/de existiert, GUI-Verdrahtung ausstehend. PS-TUI bleibt zweisprachig per `L`-Helper und Systemsprache.
- Größen: bevorzugt 1024x640, Minimum 1024x640 (Fenster nie kleiner als Inhalt), Sidebar 210px (64px eingeklappt), Zeilen 36px; Seiteninhalt in ScrollView (Scrollen erlaubt).
