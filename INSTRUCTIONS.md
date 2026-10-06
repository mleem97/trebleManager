# Instructions: CMD, PowerShell, offline release

Covers repo `mleem97/trebleManager` (branch `main`). Everything also works **without
GitHub access** once the ZIP has been transferred once (USB stick). Internet is only
needed afterwards for optional firmware/Magisk downloads.

## 0. Requirements (all paths)

- Windows 10/11, PowerShell 5.1 (built in) or PowerShell 7
- ADB/Fastboot: `C:\Program Files (x86)\Minimal ADB and Fastboot\` or platform-tools
- USB drivers (HiSuite/Kirin), USB debugging on the P10, cable straight into the PC
- Keep the folder layout intact: `scripts\`, `data\firmware`, `data\magisk`, `data\roms`, `logs\`, `backups\`

## A. Via CMD (quickest check, no PowerShell TUI)

1. Put `p10-magisk-check-FIXED.bat` next to `adb.exe`
   (or into the extracted release folder if `adb.exe` is on PATH).
2. Double-click (confirm admin with `Y` if asked).
3. The script auto-detects Android vs. fastboot and writes
   `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt`.
4. Evaluate/upload that TXT first — only then continue towards root.

## B. Via PowerShell (full TUI + CLI)

Interactive TUI (recommended):

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Or double-click starter: `Start-TrebleToolkit.bat` (asks for elevation if needed).

Straight from GitHub (only if GitHub is reachable):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $f=\"$env:TEMP\Treble-Toolkit.ps1\"; Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1' -OutFile $f -UseBasicParsing; & $f"
```

CLI examples (append after `& $f` for remote runs):

```powershell
.\scripts\Treble-Toolkit.ps1 detect --json
.\scripts\Treble-Toolkit.ps1 analyze
.\scripts\Treble-Toolkit.ps1 download --firmware-file "<https-URL-to-full-ZIP>" --yes
.\scripts\Treble-Toolkit.ps1 extract
.\scripts\Treble-Toolkit.ps1 export --image ".\data\roms\custom-rom.zip"
.\scripts\Treble-Toolkit.ps1 backup
.\scripts\Treble-Toolkit.ps1 flash --image ".\data\magisk\magisk_patched.img" --yes
.\scripts\Treble-Toolkit.ps1 verify
.\scripts\Treble-Toolkit.ps1 diagnostic --anonymize
```

## C. As release ZIP (GitHub blocked / manual upload)

1. Copy `trebleManager-v2.1.0.zip` + `trebleManager-v2.1.0.zip.sha256` from any source
   (USB stick) — no git, no GitHub needed.
2. Verify (PowerShell):
   ```powershell
   (Get-FileHash .\trebleManager-v2.1.0.zip -Algorithm SHA256).Hash -eq (Get-Content .\trebleManager-v2.1.0.zip.sha256)
   ```
   Must be `True`, otherwise transfer again.
3. Extract (path without spaces preferred, e.g. `C:\trebleManager\`),
   check layout: `scripts\Treble-Toolkit.ps1`, `Start-TrebleToolkit.bat`,
   `p10-magisk-check-FIXED.bat`, `INSTRUCTIONS.md`, `data\`, `logs\`, `backups\`.
4. Continue with A/B. For offline use, pre-place via stick:
   - full firmware ZIP into `data\firmware\`
   - Magisk APK (official GitHub release) into `data\magisk\`
   - custom ROM packages into `data\roms\` (for recovery export)
   Then the tool needs no internet at all.

## Manual release upload (for whoever cuts the release)

1. Build ZIP + `.sha256` as above (content = repo root without `.git`, without `logs/*`, without `backups/*`).
2. On GitHub: Releases → Draft new release → tag `v2.1.0` → attach both files → Publish.
3. Blocked users download both files from the release page directly (or via stick).

## Device order (short)

1. Check via A, evaluate TXT (`recovery_ramdisk` in by-name/fastboot?).
2. TUI wizard steps 1–9: Detect → Analyze → Firmware → Extract → Patch → Backup → Flash (`FLASH`+`YES`) → reboot with `Vol-Up + Power until logo` → Verify (`uid=0`).
3. On problems: Restore (original from `backups\`) instead of experiments.
