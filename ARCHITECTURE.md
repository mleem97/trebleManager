# Architecture — trebleManager

Version: [`VERSION`](VERSION) (single source of truth).

The toolkit is deliberately a **single self-contained PowerShell file**
(`scripts/Treble-Toolkit.ps1`, PS 5.1 + 7) so it runs straight from GitHub via
`irm`/direct download with zero install (see [INSTRUCTIONS.md](INSTRUCTIONS.md)).
The module regions below map 1:1 to a future native port (e.g. Rust `core/*`),
and to the planned Linux/macOS bash port (after the Windows release).

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

Requirement is instant URL-run without a build toolchain. The future Rust port maps each
PS region to a `core/*` module; parsers are pure functions with unit tests in
`tests/Test-Parsers.ps1` (simulated outputs only there).
