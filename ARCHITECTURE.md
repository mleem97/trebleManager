# Architecture — trebleManager

Version: [`VERSION`](VERSION) (single source of truth).

The toolkit reference is a **single self-contained PowerShell file**
(`scripts/Treble-Toolkit.ps1`, PS 5.1 + 7) so it runs straight from GitHub via
`irm`/direct download with zero install (see [INSTRUCTIONS.md](INSTRUCTIONS.md)),
plus the Linux/macOS bash port (`scripts/treble-toolkit.sh`, same logic).
Pure logic is mirrored in Rust (`core/treble_core`, `gsi-root/` crates,
`cargo test`) as the migration target; the module regions below map 1:1.

## Module regions (-> `core/*` mapping)

- `core/device`: `$DeviceProfiles` (VTR-L29/L09, VKY-L29), `Update-TTMode` (android/fastboot/none, no useless reboots).
- `core/adb`: `Find-TTTools`, `Invoke-TTAdb`, `Get-TTProp`, prop set incl. `ro.product.*`, `ro.boot.*`, `ro.treble.enabled`.
- `core/fastboot`: `Invoke-TTFastboot`, `ConvertFrom-FastbootGetvar` (tolerant of `Command not allowed`), getvar set from spec.
- `core/partitions`: `ConvertFrom-ByNameListing`, `Invoke-TTAndroidAnalysis` + `Invoke-TTFastbootAnalysis`.
- `core/firmware`: `Get-TTFirmwareBaseline` (stock direct, GSI assisted), `Test-FirmwareCompatibility` (model family strict, submodel/region/EMUI as WARN, example firmware advisory only), `Invoke-TTUpdateAppAnalysis`, `Find-TTRecoveryImage` (exact names, never renamed).
- `core/download`: `Test-FirmwareUrl`, `Invoke-FirmwareDownload` (BITS resume + WebClient progress), `Test-DownloadedFirmware` (size/hash/UPDATE.APP check).
- `core/romexport`: `Test-BootImageMagic`, `Get-RomImageEntries`, `Export-RecoveryFromRom` (direct `.img`, ROM `.zip`, `payload.bin` via payload-dumper-go; GSI refused honestly) → `data/recovery/<rom>/` + metadata.
- `core/magisk`: `$MagiskCompatTable` (never blind latest), `Find-TTMagiskApk`, `Get-TTMagiskInfo` (version+SHA-256), `Prepare-TTMagiskPatch` (staging + instructions, detection via adb pull/file picker, hash inequality required).
- `core/backup`: `New-TTBackup` (`backups/<MODEL>/recovery_ramdisk/<stamp>/` + metadata.json/sha256/sha512/partition-probe; dd attempt documented honestly, never faked).
- `core/flashing`: `Test-TTFlashReadiness` (9 checks), `Invoke-TTSafeFlash` (command derived from profile, WARNING dialog, double confirmation).
- `core/verification`: `Invoke-TTRootVerification` (wait-for-device, `which su`, `su -c id` → only `uid=0` = ROOTED, boot alone ≠ root), `Invoke-TTRestoreFlow`, `New-TTDiagnostic` (ZIP with 10 files, `--anonymize`).
- `core/i18n`: `$TTLang` (system UI culture) + `L "en" "de"` helper. Repo and default UI are English; German UI if the system language is German.
- `orchestrator/P0`: tool registry (adb/fastboot required, scrcpy optional),
  explicit device states (ready/unauthorized/offline/multiple per transport),
  startup preflight gate (blocks the menu, never silent), per-step requirement
  gates, explicit multi-device target selection via `ANDROID_SERIAL`.
- `orchestrator/P1`: 8 workflow goals → planner (plan with live gate results,
  no execution) → persistent state (`logs/workflow-state.json`) → stepwise
  execution reusing all existing screens → controlled failure flow
  (diagnostic/restore/abort) → resume. JSON API: `preflight`, `recon`,
  `status` (full state), `workflow --goal` (plan), `resume`, `root`.
- `ui`: TUI (`Show-TTMenu` arrow keys, `Show-TTHeader` with mode/profile/OS, step screens, status, bootkeys, tools, logs, wizard).
- `cli`: dispatcher `detect|analyze|firmware|download|extract|export|patch|backup|flash|verify|restore|diagnostic|wizard|help` + `--json/--yes/--image/--firmware-file/--anonymize/--no-reboot`.

## OS independence

`Get-OSClassification` classifies from getprop (any OS): TrebleDroid GSI, Lineage GSI,
Pixel/Superior/AOSP GSI, Stock EMUI 8/9/9.1, custom ROM, unknown. GSI hides the Huawei
base → baseline assisted (user input + fastboot product + CUST). The flash profile stays
OS-independent (`recovery_ramdisk`); the GSI is preserved.

## Data flow

1. Detect → Analyze (props + by-name + getvar, OS class) → Firmware (baseline + compat).
2. Download/Extract (UPDATE.APP + image validation: size/hash/header) → Export (custom ROMs) → Patch (real, on-device).
3. Backup (mandatory) → safety gate → confirm → flash → reboot (Huawei boot keys) → verify (uid=0).
4. On FAIL/bootloop: diagnostic ZIP → restore (hash+partition+confirm).

## Why no native build here

Requirement is instant URL-run without a build toolchain. The Rust port maps each
PS region to a `core/*` / `gsi-root` module (see PORTING.md for per-function
coverage); parsers are pure functions with unit tests in
`tests/Test-Parsers.ps1` (simulated outputs only there) and `cargo test`.

---

<a id="de"></a>
## Deutsch — Architektur

Version: [`VERSION`](VERSION) (Single Source of Truth).

Referenz ist eine **einzelne, eigenständige PowerShell-Datei**
(`scripts/Treble-Toolkit.ps1`, PS 5.1 + 7), damit sie per `irm`/Direkt-Download
ohne Install läuft (siehe [INSTRUCTIONS.md](INSTRUCTIONS.md)), plus der
Linux/macOS-Bash-Port (`scripts/treble-toolkit.sh`, gleiche Logik). Pure Logik
ist in Rust gespiegelt (`core/treble_core`, `gsi-root/`-Crates, `cargo test`)
als Migrations-Ziel; die Modul-Regionen unten mappen 1:1.

## Modul-Regionen (-> `core/*`-Mapping)

- `core/device`: `$DeviceProfiles` (VTR-L29/L09, VKY-L29), `Update-TTMode`
  (android/fastboot/none, keine nutzlosen Reboots).
- `core/adb`: `Find-TTTools`, `Invoke-TTAdb`, `Get-TTProp`, Prop-Set inkl.
  `ro.product.*`, `ro.boot.*`, `ro.treble.enabled`.
- `core/fastboot`: `Invoke-TTFastboot`, `ConvertFrom-FastbootGetvar`
  (tolerant gegen `Command not allowed`), Getvar-Set aus Spec.
- `core/partitions`: `ConvertFrom-ByNameListing`, `Invoke-TTAndroidAnalysis`
  + `Invoke-TTFastbootAnalysis`.
- `core/firmware`: `Get-TTFirmwareBaseline` (Stock direkt, GSI assistiert),
  `Test-FirmwareCompatibility` (Modellfamilie strikt, Submodell/Region/EMUI
  als WARN, Beispiel-Firmware nur Advisory), `Invoke-TTUpdateAppAnalysis`,
  `Find-TTRecoveryImage` (exakte Namen, nie umbenannt).
- `core/download`: `Test-FirmwareUrl`, `Invoke-FirmwareDownload` (BITS-Resume
  + WebClient-Progress), `Test-DownloadedFirmware` (Size/Hash/UPDATE.APP-Check).
- `core/romexport`: `Test-BootImageMagic`, `Get-RomImageEntries`,
  `Export-RecoveryFromRom` (direkt `.img`, ROM `.zip`, `payload.bin` via
  payload-dumper-go; GSI ehrlich abgelehnt) → `data/recovery/<rom>/` + Metadata.
- `core/magisk`: `$MagiskCompatTable` (nie blind latest), `Find-TTMagiskApk`,
  `Get-TTMagiskInfo` (Version+SHA-256), `Prepare-TTMagiskPatch` (Staging +
  Anleitung, Erkennung via adb-Pull/File-Picker, Hash-Ungleichheit Pflicht).
- `core/backup`: `New-TTBackup`
  (`backups/<MODEL>/recovery_ramdisk/<stamp>/` + metadata.json/sha256/sha512/partition-probe;
  dd-Versuch ehrlich dokumentiert, nie gefakt).
- `core/flashing`: `Test-TTFlashReadiness` (9 Checks), `Invoke-TTSafeFlash`
  (Command aus Profil abgeleitet, WARNING-Dialog, Doppel-Bestätigung).
- `core/verification`: `Invoke-TTRootVerification` (wait-for-device,
  `which su`, `su -c id` → nur `uid=0` = ROOTED, Boot allein ≠ Root),
  `Invoke-TTRestoreFlow`, `New-TTDiagnostic` (ZIP mit 10 Dateien,
  `--anonymize`).
- `core/i18n`: `$TTLang` (System-UI-Culture) + `L "en" "de"`-Helper. Repo und
  Default-UI sind Englisch; deutsche UI bei deutschem System.
- `orchestrator/P0`: Tool-Registry (adb/fastboot Pflicht, scrcpy optional),
  explizite Device-States (ready/unauthorized/offline/multiple pro Transport),
  Startup-Preflight-Gate (blockiert Menü, nie still), Pro-Step-Gates,
  explizite Multi-Device-Auswahl via `ANDROID_SERIAL`.
- `orchestrator/P1`: 8 Workflow-Goals → Planner (Plan mit Live-Gate-Results,
  keine Ausführung) → persistenter State (`logs/workflow-state.json`) →
  Step-Ausführung mit existierenden Screens → kontrollierter Failure-Flow
  (Diagnose/Restore/Abbruch) → Resume. JSON-API: `preflight`, `recon`,
  `status` (voller State), `workflow --goal` (Plan), `resume`, `root`.
- `ui`: TUI (`Show-TTMenu` Pfeiltasten, `Show-TTHeader` mit Modus/Profil/OS,
  Step-Screens, Status, Bootkeys, Tools, Logs, Wizard).
- `cli`: Dispatcher + `--json/--yes/--image/--firmware-file/--anonymize/--no-reboot`.

## OS-Unabhängigkeit

`Get-OSClassification` klassifiziert via getprop (jedes OS): TrebleDroid-GSI,
Lineage-GSI, Pixel/Superior/AOSP-GSI, Stock-EMUI 8/9/9.1, Custom-ROM, unknown.
GSI versteckt die Huawei-Basis → Baseline assistiert (User-Input + Fastboot-
Product + CUST). Das Flash-Profil bleibt OS-unabhängig (`recovery_ramdisk`);
das GSI bleibt erhalten.

## Datenfluss

1. Detect → Analyze (Props + by-name + getvar, OS-Klasse) → Firmware
   (Baseline + Kompat).
2. Download/Extract (UPDATE.APP + Image-Validierung: Size/Hash/Header) →
   Export (Custom-ROMs) → Patch (echt, on-device).
3. Backup (Pflicht) → Safety-Gate → Confirm → Flash → Reboot
   (Huawei-Bootkeys) → Verify (uid=0).
4. Bei FAIL/Bootloop: Diagnose-ZIP → Restore (Hash+Partition+Confirm).

## Warum kein Native-Build hier

Anforderung ist Instant-URL-Run ohne Build-Toolchain. Der Rust-Port mappt
jede PS-Region auf ein `core/*`-/`gsi-root`-Modul (Coverage pro Funktion:
PORTING.md); Parser sind pure Funktionen mit Unit-Tests in
`tests/Test-Parsers.ps1` (nur dort simulierte Outputs) und `cargo test`.
