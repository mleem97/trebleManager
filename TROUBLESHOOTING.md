# Troubleshooting — trebleManager

## Download (stock firmware)

- `URL rejected`: only `http(s)` + archive type (`zip/7z/tar/gz/app/rar`). `ftp/file/exe` are blocked.
- BITS stuck: the tool automatically falls back to WebClient with progress bar. Check free space (2–4 GB), target `data/firmware/` (cache, no re-download).
- `WARN < 100 MB / no UPDATE.APP in ZIP`: wrong package (delta/region). Pick another full firmware with matching CUST, then TUI step 4.
- Honesty note: in 2026 there is no guaranteed official direct link. Use HiSuite (official) for re-install or pick a community-archive link yourself + verify hash. Without `--yes` the CLI downloads nothing.

## ADB / fastboot

- `ADB not found`: put `p10-magisk-check-FIXED.bat` / PS1 next to `adb.exe` or set PATH (`C:\Program Files (x86)\Minimal ADB and Fastboot`).
- `No device detected`: change cable/port, check drivers (HiSuite/Kirin), `adb kill-server/start-server` (TUI tools).
- `ADB unauthorized`: confirm the on-device dialog ("always allow"), replug cable.
- `Fastboot not found / waiting for device`: boot to fastboot manually (Power+Vol-Down or `adb reboot bootloader`), wait the 30s window.
- `FAILED (remote: Command not allowed)`: normal on Huawei. Not proof of lock. Continue with by-name + `ro.boot.*` + `/proc/cmdline`.
- Old Minimal fastboot `unknown command 2>&1`: wrongly escaped redirection. Use FIXED BAT / PS1 (correct `>> log 2>&1`, never as fastboot arg).

## Partitions / firmware

- `Unsupported partition layout` (no `recovery_ramdisk`): run TUI step 2 + tools `getvar` + by-name. Never guess, no `boot`/`recovery` substitute. Create a diagnostic ZIP.
- `Unsupported model`: only VTR-L29/L09, VKY-L29 are profiled. Other Kirin devices: add a profile in `$DeviceProfiles`, never flash with a foreign profile.
- `Firmware mismatch` (FAIL): model family (VTR vs VKY) is strict; check submodel/region (L29 vs L09, C432 vs C185) and EMUI (9.1 vs 9.0/8). Pick another full firmware with matching CUST.
- `UPDATE.APP` unclear: place file in `data/firmware/`, CLI `extract` / TUI step 4. Extractors go in `data/tools/` (huawei-update-extractor/splitupdate) or extract manually. Keep the exact name (`RECOVERY_RAMDIS.img` vs `RECOVERY_RAMDISK.img`).
- `Cannot safely flash image`: check size/hash/header (`extract` shows details). Never force foreign/renamed images.

## Magisk / root

- `Patch failed`: only real on-device patches (`Magisk → Select and Patch a File` with exactly `RECOVERY_RAMDIS(K).img`). Hash must differ from stock. Pull via `adb pull /sdcard/Download/`.
- `INCONCLUSIVE` (su present, no uid=0): open the Magisk app, grant root, repeat reboot with boot cheat, wait out first boot.
- `NOT_ROOTED` after flash: boot cheat forgotten (`Vol-Up + Power until logo`)? Booted stock instead of Magisk recovery. Repeat. Check Magisk package (`pm list packages`).
- `Boot verification failed`: stay in fastboot, TUI Restore / CLI `restore`, original hash is verified, then `fastboot flash recovery_ramdisk original.img`, `fastboot reboot`.

## Recovery export (custom ROMs)

- `No boot.img/recovery.img/payload.bin in ZIP`: probably a GSI system package — it has no recovery by design. Use the stock UPDATE.APP path.
- `payload.bin` without output: place `payload-dumper-go` in `data/tools/` or extract `boot.img` manually, then re-run `export`.
- Exported images are patch candidates, not flash-ready: validate target partition from the device profile, patch in Magisk, pass the safety gate. Rights are granted in the Magisk app (verified via `uid=0`).

## GSI preservation

- The tool never touches `system`/`vendor`/`userdata`. On accidental wipe use stock recovery only (per wiki, TWRP factory reset breaks userdata).

## Logs / diagnostics

- Every run writes `logs/toolkit-<stamp>.log`. TUI log preview (200 lines) + open folder.
- Full diagnosis: CLI `diagnostic [--anonymize] [--json]` or TUI tools. ZIP in `logs/` (10 files). Redact/anonymize serials before upload.
