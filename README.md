# trebleManager

> Windows PowerShell TUI + CLI to root the **Huawei P10 (VTR-L29 and others)** on
> TrebleDroid/Lineage GSI via Magisk `recovery_ramdisk` patching — detect, analyze,
> extract, patch, backup, flash, verify, restore. No blind flashing, no touched GSI.

[![License](https://img.shields.io/badge/License-Apache%202.0-green?style=for-the-badge)](LICENSE) [![Version](https://img.shields.io/badge/Version-2.2.0-orange?style=for-the-badge)](CHANGELOG.md) [![Windows](https://img.shields.io/badge/Windows-PS%205.1%20%2B%207-blue?style=for-the-badge)](#installation) [![Linux](https://img.shields.io/badge/Linux-bash-green?style=for-the-badge)](#installation) [![Device](https://img.shields.io/badge/Device-Huawei%20P10%20VTR--L29-yellow?style=for-the-badge)](#compatibility)

## Links

- **Repository:** [github.com/mleem97/trebleManager](https://github.com/mleem97/trebleManager)
- **Issues:** [github.com/mleem97/trebleManager/issues](https://github.com/mleem97/trebleManager/issues)
- **Technical basis:** [phhusson Discussion #2542](https://github.com/phhusson/treble_experimentations/discussions/2542) · [Huawei P10 Wiki](https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus)
- **Magisk (official):** [github.com/topjohnwu/Magisk/releases](https://github.com/topjohnwu/Magisk/releases)
- **Sponsor:** [github.com/sponsors/mleem97](https://github.com/sponsors/mleem97) · PayPal: `mleem97`

## Overview

**trebleManager** drives the full Huawei P10 root workflow for TrebleDroid/LineageOS GSI
(Android 13, `lineage_arm64_bgN`) and other OS states (Stock EMUI 8/9/9.1, AOSP/Pixel/Superior
GSI, custom ROMs, Android 8–14). It detects the real device state first — ADB/Fastboot mode,
OS class, partition layout, firmware baseline — and only then guides patching and flashing:

```
original Huawei RECOVERY_RAMDISK.img
        ↓ Magisk "Select and Patch a File" (on-device)
magisk_patched.img
        ↓ fastboot flash recovery_ramdisk (after safety gate + double confirm)
root via Vol-Up + Power boot cheat → verify uid=0
```

The existing GSI (`system`/`vendor`) is never touched. `FAILED (remote: Command not allowed)`
from Huawei fastboot is treated as a Huawei quirk, never as proof of lock.

## Compatibility

| Device | Model | Arch | State |
|---|---|---|---|
| Huawei P10 | VTR-L29 (primary) | arm64 | Supported |
| Huawei P10 | VTR-L09 | arm64 | Supported |
| Huawei P10 Plus | VKY-L29 | arm64 | Supported |

| Host | Shell | State |
|---|---|---|
| Windows 10/11 | PowerShell 5.1 | Supported |
| Windows 10/11 | PowerShell 7+ | Supported |
| Linux x64 | bash 4+ (`scripts/treble-toolkit.sh`) | Supported |
| macOS (bash port) | bash | Planned (after Windows release) |

| Device OS | Detection | State |
|---|---|---|
| Stock EMUI 8.0 / 9.0 / 9.1 | getprop classification | Supported |
| TrebleDroid / Lineage GSI (bgN/bvS/bgS), Android 10–14 | getprop classification | Supported |
| PixelExperience / SuperiorOS / AOSP GSI, custom ROMs | getprop classification | Supported |

UI language: English by default, German if the system language is German.

## Features

- Auto device-mode detection (Android / fastboot / none, no useless reboots)
- OS-independent analysis: getprop set, `/dev/block/by-name`, `ro.boot.*`, `/proc/cmdline`, fastboot `getvar` set
- Firmware baseline + compatibility check (model family strict, submodel/region/EMUI as WARN; example firmware advisory only, never exclusive)
- Stock firmware downloader with progress (BITS resume + WebClient fallback) and mandatory `YES` confirmation
- `UPDATE.APP` analysis + `RECOVERY_RAMDIS(K).img` validation (size, SHA-256/512, header magic, exact filename kept)
- **Recovery export from compatible custom ROMs** (direct `.img`, ROM `.zip` with `boot/recovery.img`, `payload.bin` via payload-dumper-go; GSI system images honestly refused)
- Real on-device Magisk patch flow (never copy-and-claim, patched `!=` stock enforced by hash)
- Backup before every flash (`backups/<MODEL>/recovery_ramdisk/<stamp>/` + `metadata.json` + hashes)
- 9-point safety gate with `DO NOT FLASH`, derived flash command, double confirmation
- Huawei boot procedure built-in (Vol-Up + Power, non-persistent), root verify only on `uid=0`
- Full restore/unroot mode, diagnostic ZIP (10 files, `--anonymize`), CLI with `--json`
- No auto `erase/format userdata`, no `flashing unlock`, no bootloader unlock, no telemetry

## Installation

### Step 0 — Setup (fresh machines, run once)

Double-click `Setup-TrebleToolkit.bat`: requests admin automatically (UAC), allows
script execution, installs ADB/fastboot (official Google platform-tools, portable)
and — only if you agree — scrcpy into user PATH, saves `data/config.json`.
Afterwards all tools are globally reachable.

### Option A — CMD (quick check)

1. Put `p10-magisk-check-FIXED.bat` next to `adb.exe`
   (e.g. `C:\Program Files (x86)\Minimal ADB and Fastboot\`).
2. Double-click (confirm admin with `Y` if asked).
3. It auto-detects Android vs. fastboot and writes
   `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt`.

### Option B — PowerShell TUI/CLI (Windows)

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Or double-click starter: `Start-TrebleToolkit.bat` (asks for elevation if needed), or run straight
from GitHub (see [INSTRUCTIONS.md](INSTRUCTIONS.md)).

### Option B2 — Linux bash (same logic, no extras)

```bash
cd /path/to/trebleManager
chmod +x scripts/treble-toolkit.sh
./scripts/treble-toolkit.sh
./scripts/treble-toolkit.sh detect --json
./scripts/treble-toolkit.sh diagnostic --anonymize
```

Needs only `adb`, `fastboot` (`sudo apt install android-tools-adb android-tools-fastboot`
on Debian/Ubuntu). Optional: `unzip`, `curl`, `zip` (or `python3` as fallback each).

### Option C — Offline release ZIP (GitHub blocked)

1. Copy `trebleManager-v2.2.0.zip` + `.sha256` via USB stick.
2. Verify: `(Get-FileHash .\trebleManager-v2.2.0.zip -Algorithm SHA256).Hash -eq (Get-Content .\trebleManager-v2.2.0.zip.sha256)` must be `True` (Linux: `sha256sum -c trebleManager-v2.2.0.zip.sha256`).
3. Extract (path without spaces preferred), keep layout (`scripts\`, `data\`, `logs\`, `backups\`).
4. Optionally pre-place full firmware ZIP in `data\firmware\` and Magisk APK in `data\magisk\` — then no internet is needed at all.

## Run Directly from the Internet

No files needed — one command, straight from the prompt. TUI stays interactive.
First run on a fresh machine: use `Setup-TrebleToolkit.bat` once (auto-UAC,
execution policy, installs ADB/fastboot + optional scrcpy into user PATH,
saves `data/config.json`).

### CMD (Windows, zero files)

```cmd
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $f=$env:TEMP + '\Treble-Toolkit.ps1'; Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1' -OutFile $f -UseBasicParsing; & $f"
```

(No nested double quotes — `$env:TEMP + '\...'` avoids the classic
`TerminatorExpectedAtEndOfString` parser error.)

### PowerShell (Windows, zero files)

```powershell
irm https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1 | iex
```

(If your session blocks scripts, start PowerShell once via `Setup-TrebleToolkit.bat`
or `powershell -ExecutionPolicy Bypass`.)

### Bash (Linux, zero files)

```bash
curl -fsSL https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/treble-toolkit.sh | bash
```

Stays interactive (reads `/dev/tty`); with args: `curl -fsSL <url> | bash -s -- detect --json`.
Fallback without process substitution issues:

```bash
curl -fsSL https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/treble-toolkit.sh -o /tmp/treble-toolkit.sh \
&& chmod +x /tmp/treble-toolkit.sh && /tmp/treble-toolkit.sh
```

## Dependencies

### Runtime

- **ADB/Fastboot** — Minimal ADB and Fastboot or Android platform-tools (PATH or next to the scripts). Detected at startup with version/path; missing tools abort with a real error, never a guess.
- **scrcpy (optional)** — screen mirror while rooting ([Genymobile/scrcpy](https://github.com/Genymobile/scrcpy)). Detected at startup, launchable from Tools; absence only logs INFO.
- **USB drivers** — HiSuite/Kirin drivers, USB debugging on device
- **Magisk APK** — official releases only, placed in `data/magisk/`
- **Stock firmware** — full EMUI 9.1 package for `UPDATE.APP` (see downloader / [INSTRUCTIONS.md](INSTRUCTIONS.md))

### Optional tools (in `data/tools/`)

- `huawei-update-extractor` / `splitupdate` (UPDATE.APP extraction)
- `payload-dumper-go` (custom ROM `payload.bin` export)

### Test only

- Windows PowerShell 5.1+ to run `tests/Test-Parsers.ps1`

## Usage

TUI without args. CLI:

```
Treble-Toolkit.ps1 detect|analyze|firmware|download|extract|export|patch|backup|flash|verify|restore|diagnostic|wizard|help [--json] [--yes] [--image <path>] [--firmware-file <url|path>] [--anonymize] [--no-reboot]
```

Wizard order: Detect → Analyze → Firmware → Extract → Patch → Backup → Flash → Reboot+Verify.
Flash/restore need double confirmation (`FLASH`+`YES` / `RESTORE`+`YES`, CLI: `--yes`).
Details: [INSTRUCTIONS.md](INSTRUCTIONS.md), [QUICKSTART.md](QUICKSTART.md).

## Repository Layout

```
├── scripts/                # Treble-Toolkit.ps1 (Windows TUI+CLI, PS 5.1/7) + treble-toolkit.sh (Linux bash port) + Setup-Windows.ps1
├── tests/                  # Test-Parsers.ps1 + test-parsers.sh (parser/firmware/OS/hash/root/URL/magic tests)
├── data/firmware/          # Full firmware drops (UPDATE.APP source, offline cache)
├── data/magisk/            # Magisk APK + to-patch staging + pulled patched images
├── data/roms/              # Custom ROM packages for recovery export
├── data/recovery/          # Exported recovery/boot images + metadata
├── data/tools/             # Optional extractors (update extractor, payload dumper)
├── backups/                # Pre-flash backups per model/partition/stamp (git-ignored)
├── logs/                   # Session logs + diagnostic ZIPs (git-ignored)
├── p10-magisk-check-FIXED.bat
├── Start-TrebleToolkit.bat # Local smart launcher (finds pwsh/powershell, elevation)
├── Setup-TrebleToolkit.bat # First-run setup (auto-UAC, policy, tools+PATH, config)
├── Run-FromGitHub.bat      # Remote loader (RAW_URL, temp download + run)
├── README.md
├── INSTRUCTIONS.md         # CMD / PowerShell / offline-release guide
├── QUICKSTART.md
├── ARCHITECTURE.md
├── SECURITY.md
├── TROUBLESHOOTING.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── CHANGELOG.md
├── VERSION                 # Single source of truth for version
└── LICENSE                 # Apache-2.0
```

## Credits

| Role | Contributor |
|---|---|
| **Codebase** | [mleem97](https://github.com/mleem97) |
| Method reference | [michal25](https://github.com/phhusson/treble_experimentations/discussions/2542) (P10 GSI guide) |
| TrebleDroid/GSI | [phhusson](https://github.com/phhusson/treble_experimentations) |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Development follows `dev -> main`. See the branch note in [CONTRIBUTING.md](CONTRIBUTING.md)
before opening a pull request. Downloads are published on the GitHub Releases page;
development states are intentionally not presented as stable releases.

## License

This project is licensed under the **Apache License 2.0**. See [`LICENSE`](LICENSE).
