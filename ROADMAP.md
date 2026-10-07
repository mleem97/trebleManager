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

## 4. Wizard as state machine (done: v2.14.0+, resolver v2.17.0)

ROM-aware wizard (installed-ROM question, plain-words goals, visible
`[SKIP]`s) + target-image resolver (device → Android → system → variant →
config). Step persistence via `logs/workflow-state.json` + resume exists
(orchestrator P1).

## 5. Firmware as a repository (started: v2.17.0)

Per-package metadata (`android`, `root_artifact`, verified `url`/`file`,
gated firmware builds with portal pages) in
`data/compatibility/huawei/p10/*.yaml` (+ JSON mirror); `download-rom` and
Magisk stable.json auto-fetch resolve artifacts automatically. Remaining:
UPDATE.APP auto-extraction, more verified direct URLs.

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

---

<a id="deutsch"></a>
## Deutsch — Roadmap (komplett)

Ausgangspunkt: das arbeitende Toolkit in diesem Repo (TUI + CLI,
Geräte-Profile, Kompatibilitäts-Registry, Safety-Gate, Linux-Bash-Port).
Dieser Plan erweitert es — schreibt es nicht neu.

## 0. Erledigt in v2.7.0 (Orchestrator-Fundament)

- Startup-Preflight-Gate (Tool-Registry, schreibbare Dirs, explizite Device-States)
- 8 Workflow-Goals + Planner mit Live-Gate-Results + persistenter State + Resume
- Kontrollierter Failure-Flow (Diagnose → Restore → Abbruch), JSON-Status-API
- CLI: `preflight|recon|status|workflow|resume|root` (+ `--goal`, `--mode`)
- Multi-Device-Zielwahl, Unauthorized/Offline als eigene States

## 1. Wo wir stehen

- Wizard/CLI: Detect → Analyze (+ ROM-Frage) → Ziel in Alltagssprache →
  Plan mit sichtbaren `[SKIP]`s → Run; dazu Resolver (Device → Android →
  System → Variante → Config), Download/Export/Patch/Backup/Flash/Verify,
  flash-system, TWRP (Shared-Slot), Restore, APTouch-Auto-Fix, Diagnose,
  Compat, Root-Methoden, Devices.
- Weiter enforcede Regeln: kein Blind-Flash, `DO NOT FLASH`-Gate, Backup vor
  Flash, On-Device-Magisk-Patch only, `uid=0` als einziger Root-Beweis,
  Huawei `Command not allowed` ist UNKNOWN (nie LOCKED).

## 2. Run-Modes (erledigt: v2.5.0)

- `safe` (Default): jeder irreversible Step wird interaktiv bestätigt.
- `unattended` + `--yes`: bestätigt einen **voll validierten** Run automatisch.
  Safety-Gates laufen trotzdem — `--yes` skippt nie Checks.
- `developer`: schaltet `dump-partitions|properties|vendor|logs` für ROM-Research frei.

## 3. Post-Flash-Validierung (erledigt: v2.5.0)

`validate` (TUI-Tools + CLI) fährt read-only System/Hardware-Checks
(Boot, ADB, OS, SELinux, Root, Mounts, Wi-Fi, Bluetooth, Batterie, Sensoren)
und schreibt `logs/validation-<stamp>.json`. Nächster Schritt:
Remediation-Hinweise pro Check.

## 4. Wizard als State Machine (erledigt: v2.14.0+, Resolver v2.17.0)

ROM-bewusster Wizard (Install-ROM-Frage, Ziele in Alltagssprache, sichtbare
`[SKIP]`s) + Target-Image-Resolver (Device → Android → System → Variante →
Config). Step-Persistenz via `logs/workflow-state.json` + Resume existiert
(Orchestrator P1).

## 5. Firmware als Repository (gestartet: v2.17.0)

Pro-Paket-Metadaten (`android`, `root_artifact`, geprüfte URLs, gated
Firmware-Builds mit Portal-Seiten) in `data/compatibility/huawei/p10/*.yaml`
(+ JSON-Spiegel); `download-rom` und Magisk-stable.json-Auto-Fetch lösen
Artefakte automatisch auf. Offen: UPDATE.APP-Auto-Extrakt, mehr geprüfte
Direkt-URLs.

## 6. Core-Extraktion (gestartet v2.18.0, stepwise)

PowerShell bleibt Referenz-Implementierung (Direct-from-GitHub-Run muss
laufen). `core/treble_core` hält erste pure Module mit `cargo test`-Suiten,
`gsi-root/` wächst Richtung Analyzer/Root-Engine/GUI. PS1/Bash-Integration
(Wrapper mit Fallback) erst wenn Module einen vollen Flow abdecken.
CLI-Command-Namen bleiben über Migration stabil.

## 7. Stehende Regeln (nie ändern)

Unbekannt ist nie kompatibel. Kein automatischer Unlock/Wipe/Format, keine
Force- oder Verity-Disable-Flags, keine fremden gepatchten Images, keine
Fake-Erfolgs-Pfade. Neue Version = neues unveränderliches Release (siehe
[RELEASE.md](RELEASE.md)).

## 8. Pre-Rust-Ära (v2.1.0 – v2.17.x)

Siehe oben (EN+DE): script-only, eingefroren, weiter gültig.
