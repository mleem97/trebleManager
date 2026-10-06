# QUICKSTART — trebleManager

Fastest safe path from zero to verified root on Huawei P10 (VTR-L29).

## 1. Check (2 minutes, read-only)

CMD: put `p10-magisk-check-FIXED.bat` next to `adb.exe`, double-click.
It writes `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt` — check that
`recovery_ramdisk` shows up in by-name/getvar. Nothing is flashed.

## 2. Full TUI

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Linux:

```bash
cd /path/to/trebleManager
./scripts/treble-toolkit.sh
```

Follow the wizard: **Detect → Analyze → Firmware → Extract → Patch → Backup →
Flash → Reboot+Verify**.

## 3. What you need in hand

- Full EMUI 9.1 firmware (for `UPDATE.APP` → TUI step 3 downloader or `data/firmware/` drop)
- Magisk APK from [official releases](https://github.com/topjohnwu/Magisk/releases) → `data/magisk/`
- The boot cheat after flashing: **Vol-Up + Power until Huawei logo**, then release.
- Root counts only with `uid=0` (`su -c id`); grant rights in the Magisk app.

## 4. If something breaks

Stay in fastboot → TUI **Restore / Unroot** (or CLI `restore`) flashes the
backed-up original back. See [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## 5. Tests

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\Test-Parsers.ps1
```

```bash
bash tests/test-parsers.sh
```

Details: [INSTRUCTIONS.md](INSTRUCTIONS.md), [CONTRIBUTING.md](CONTRIBUTING.md).
