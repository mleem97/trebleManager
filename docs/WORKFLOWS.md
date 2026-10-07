# Workflows — goal flows end-to-end

Goals are plans first, execution second. The planner prints ordered steps with live gate results; running reuses the same screens/runners with persisted state and stops controlled on failure.

Related: `docs/CLI.md`, `docs/TUI.md`, `docs/GUI.md`.

## 1. Goal table (PowerShell, source `$WorkflowGoals`)

| Goal | Steps |
|---|---|
| `root` | reconnaissance, compatibility, firmware, extract, magisk_patch, backup, flash, reboot, root_verify, validate |
| `custom_rom` | reconnaissance, compatibility, firmware, rom_validation, backup_if_required, flash_system, reboot, validate |
| `stock_rom` | reconnaissance, firmware_selection, firmware_validation, artifact_extraction, backup, flash_plan, safety_gate, flash, reboot, validate |
| `root_custom_rom` | reconnaissance, custom_rom_compatibility, firmware, rom_installation, boot, root_preparation, backup, root_flash, root_verify, validate |
| `root_stock_rom` | reconnaissance, stock_firmware_validation, root_image_preparation, backup, root_flash, reboot, root_verify, validate |
| `root_custom_rom_recovery` | reconnaissance, rom_compatibility, recovery_compatibility, firmware, backup, rom_flash, recovery_flash, root_preparation, root_flash, boot, verify, validate |
| `root_stock_rom_recovery` | reconnaissance, stock_firmware_validation, recovery_validation, backup, recovery_flash, root_preparation, root_flash, boot, verify, validate |
| `restore_original` | reconnaissance, identify_original_artifact, validate_backup, rollback_plan, safety_gate, restore, reboot, validate |
| `full_reinstall` | reconnaissance, compatibility, firmware, rom_validation, backup, wipe, flash_system, reboot, validate |

State file: `logs/workflow-state.json` (`goal`, `steps`, `slot`, `last_root`, `updated`; unknown keys preserved). `resume` re-enters the saved goal. ROM choice persists in `data/installed-rom.txt`.

## 2. Root (`root`)

Detect, Analyze (with installed-ROM question), Firmware baseline plus compat, Extract (`UPDATE.APP` to `RECOVERY_RAMDIS(K).img`, exact names kept), on-device Magisk patch (`Select and Patch a File`, pulled via `adb pull /sdcard/Download/`, patched hash must differ from stock), Backup, Flash, Vol-Up + Power boot cheat, Verify (`su -c id` equals `uid=0` only), Validate.

- Gates: preflight, per-step `Test-StepGate`, model family strict (VTR vs VKY), 9-point readiness, `DO NOT FLASH` on any fail.
- Refusals: unverified profile allows analyze/export only (flash blocked); missing backup blocks flash; patched equals stock blocks flash; `FAILED (remote: Command not allowed)` is a Huawei quirk (UNKNOWN, never LOCKED) and routes to by-name plus `ro.boot.*` plus `/proc/cmdline`, not to a lock verdict.

## 3. Flash (`flash`, `flash-system`, TWRP)

- `flash` (recovery_ramdisk): derived command from profile, WARNING dialog, typed `FLASH` + `YES` (CLI `--yes`), `FLASH RESULT: OK/FAILED/UNCLEAR` with FAILED veto. Readiness items: model match, profile verified, partition present, patched image present, hash known, size 1-100 MB, firmware compat not FAIL, `original.img` backup present, fastboot device live, patched differs from stock.
- `flash-system` (ROM/GSI install): image checks (size, arm64, A-only), researched-broken builds hard-blocked (for example Lineage 20 Light, HavocOS 3.12), double confirm, `fastboot flash system`, eRecovery wipe guidance. Never auto-wipes userdata.
- TWRP: device-exact build, image validation, automatic slot backup, shared-slot warning (TWRP and Magisk overwrite each other), Vol-Up boot, never TWRP userdata wipe.
- Rust parity: `gsi-exec` runners check op id (`safe-flash`, `system-flash`, `twrp-flash`, `restore`, `wipe`) and both confirm tokens before any call; GUI pages render the same plans read-only and refuse live runs.

## 4. Backup

`New-TTBackup` writes `backups/<MODEL>/recovery_ramdisk/<stamp>/` with `original.img`, `metadata.json`, SHA-256/512, and partition probe. A `dd` attempt without device data is documented, never faked. CLI `backup` refuses without a verifiable stock image. Restore verifies original hash plus partition plus confirm, then `fastboot flash recovery_ramdisk original.img` and `fastboot reboot`.

## 5. Firmware

`Get-TTFirmwareBaseline` (stock direct, GSI assisted via user input plus fastboot product plus CUST), `Test-FirmwareCompatibility` (model family strict FAIL; submodel/region/EMUI as WARN; example firmware advisory only, never exclusive), `Test-FirmwareUrl` (only `http(s)` plus archive type `zip/7z/tar/gz/app/rar`), `Invoke-FirmwareDownload` (BITS resume plus WebClient fallback, 2-4 GB, cache in `data/firmware/`, no traffic without `--yes`), `Test-DownloadedFirmware` (size/hash/UPDATE.APP check).

- Refusals: `URL rejected` for ftp/file/exe; `WARN < 100 MB / no UPDATE.APP` routes to another full firmware with matching CUST; 2026 honesty note (no guaranteed official direct link; HiSuite or self-picked community archive plus hash verify).
- Registry: `data/compatibility/huawei/p10/*.yaml` plus generated `.json` per variant (`VTR-L29/L09/AL00/TL00`, `VKY-L29/L09`) and `data/compatibility/qualcomm/bq-aquaris-x-pro.yaml/json`; fields cover hardware, vendor advice (Oreo vs Pie), storage (eMMC vs UFS), firmware builds (gated portal pages, never faked as direct), ROM matrix (`working` / `working-slim` / `working-with-fixes` / `broken` / `variant-dependent`).

## 6. Validate

`validate` (TUI Tools plus CLI) is read-only: boot, ADB, OS, SELinux, root, mounts, Wi-Fi, Bluetooth, battery, sensors. Writes `logs/validation-<stamp>.json`. Root counts only on `uid=0`; `su` present without `uid=0` is `INCONCLUSIVE`; otherwise `NOT_ROOTED`. Per-check remediation hints are planned, not present (see `ROADMAP.md`).

## 7. Rust reference workflow (`p10-lineage20`)

`gsi-workflow::p10_lineage20` defines Analyze, Patch, Repack, VerifyOutput, Backup, Flash. `run_analyze` measures the container and completes; `run_patch` returns `RefusedExperimental` until a hardware POC exists; Flash/Reboot/Test need `--yes` and a device layer and are refused without it. `workflow list` shows the steps; `workflow run p10-lineage20 [--yes]` walks them with `ProgressEvent` lines (`Started`, `Progress`, `Message`, `Warning`, `Completed`) and `StepOutcome` (`Done`, `Skipped`, `RefusedExperimental`, `Failed`).

## 8. EDL / Qualcomm (honesty note)

`gsi-edl` covers Snapdragon 626 / MSM8953 (reference `bq-aquaris-x-pro`): pure packet builders, `StubTransport` / `RefusingTransport`, ownership proof plus double confirm before any I/O, backup-first for persist/modemst1/modemst2, full `frp`+`config` erase only via EDL 9008 deep flash. Stub-tested (20 tests per `TASKS.md`); live proof stays in `FIELD-EVIDENCE.md`, never claimed here.

---

<a id="de"></a>
## Deutsch — Workflows (Kurzspiegel)

Goals sind erst Plan, dann Ausführung (Planner mit Live-Gates, persistenter State, kontrollierter Stopp, Resume).

- Goal-Tabelle (neun Goals, oben): `root`, `custom_rom`, `stock_rom`, `root_custom_rom`, `root_stock_rom`, `root_custom_rom_recovery`, `root_stock_rom_recovery`, `restore_original`, `full_reinstall`. State: `logs/workflow-state.json`; ROM-Wahl: `data/installed-rom.txt`.
- Root: Detect, Analyze (mit ROM-Frage), Firmware-Baseline plus Kompat, Extract (`UPDATE.APP`, exakte Namen), On-Device-Magisk-Patch (Hash-Ungleichheit Pflicht), Backup, Flash, Vol-Up+Power-Boot, Verify nur bei `uid=0`, Validate. Gates: Preflight, Step-Gates, Modellfamilie strikt, 9-Punkt-Readiness, `DO NOT FLASH`. Refusals: unverifiziert nur Analyze/Export, Backup-Pflicht, gepatcht-gleich-Stock-Block, `Command not allowed` als Quirk (UNKNOWN, nie LOCKED).
- Flash: `flash` (Profil-Command, Warnung, `FLASH`+`YES`, `FLASH RESULT` mit FAILED-Veto, 9 Checks), `flash-system` (Image-Checks, kaputte Builds hart geblockt, Doppel-Confirm, nie Auto-Wipe), TWRP (gerätgenau, Slot-Backup, Shared-Slot-Warnung, nie userdata-Wipe). Rust-Runner prüfen Op-Id plus beide Tokens; GUI nur lesende Vorschau.
- Backup: `backups/<MODEL>/recovery_ramdisk/<stamp>/` mit `original.img`, `metadata.json`, Hashes; `dd` ohne Gerät nur dokumentiert. Restore mit Hash plus Partition plus Confirm.
- Firmware: Baseline (Stock direkt, GSI assistiert), Kompat (Familie strikt, Rest WARN, Beispiel nur Advisory), URL-Check (nur `http(s)` plus Archivtyp), Download (BITS/WebClient, Cache, kein Traffic ohne `--yes`), Downloaded-Check (Size/Hash/UPDATE.APP). Refusals und 2026-Hinweis (kein garantierter Direktlink) wie oben. Registry: `data/compatibility/huawei/p10/*` plus `qualcomm/bq-aquaris-x-pro.*` (YAML plus JSON).
- Validate: read-only Checks, `logs/validation-<stamp>.json`, Root nur bei `uid=0`. Remediation-Hinweise geplant, nicht vorhanden.
- Rust-Referenz `p10-lineage20`: Analyze misst, Patch `RefusedExperimental`, Flash/Reboot/Test brauchen `--yes` plus Device-Layer. Events/Outcomes wie oben.
- EDL/Qualcomm: MSM8953-Paket-Builder, Ownership plus Doppel-Confirm, Backup-first, Full-Erase nur via EDL-9008-Deep-Flash; stub-getestet, Live-Nachweis nur `FIELD-EVIDENCE.md`.
