# trebleManager

> Windows PowerShell TUI + CLI to root the **Huawei P10 (VTR-L29 and others)** on
> TrebleDroid/Lineage GSI via Magisk `recovery_ramdisk` patching — detect, analyze,
> extract, patch, backup, flash, verify, restore. No blind flashing, no touched GSI.

[![License](https://img.shields.io/badge/License-Apache%202.0-green?style=for-the-badge)](LICENSE) [![Version](https://img.shields.io/badge/Version-2.12.1-orange?style=for-the-badge)](CHANGELOG.md) [![Windows](https://img.shields.io/badge/Windows-PS%205.1%20%2B%207-blue?style=for-the-badge)](#installation) [![Linux](https://img.shields.io/badge/Linux-bash-green?style=for-the-badge)](#installation) [![Device](https://img.shields.io/badge/Device-Huawei%20P10%20VTR--L29-yellow?style=for-the-badge)](#compatibility)

## Links

- **Repository:** [github.com/mleem97/trebleManager](https://github.com/mleem97/trebleManager)
- **Wiki (user guide, EN+DE):** [github.com/mleem97/trebleManager/wiki](https://github.com/mleem97/trebleManager/wiki) — mirrored as `wiki/` subrepo (`git clone --recurse-submodules`; push both via `./push-all.sh` / `Push-All.bat`)
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
| Huawei P10 (test device) | VTR-L29 (primary, verified) | arm64 | Supported |
| Huawei P10 | VTR-L09 (verified) | arm64 | Supported |
| Huawei P10 | VTR-AL00 / VTR-TL00 (unverified) | arm64 | Analyze + export only (flash blocked) |
| Huawei P10 Plus (secondary, no test device) | VKY-L29 (verified) | arm64 | Supported |
| Huawei P10 Plus | VKY-L09 / VKY-AL00 / VKY-TL00 (unverified) | arm64 | Analyze + export only (flash blocked) |

Unverified = same Kirin 960 hypothesis, but flash stays blocked until device data
is submitted (see `device-support` issue template). `devices` CLI lists all profiles.

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
- **Compatibility registry** (`data/compatibility/huawei/p10/*.yaml` + generated `.json`): researched ROM/firmware/TWRP/Magisk matrix per variant (working / working-slim / working-with-fixes / broken / variant-dependent). TUI screen + `compat` CLI show recommendations; `flash-system` hard-blocks researched-broken builds (e.g. Lineage 20 Light, HavocOS 3.12); vendor (Oreo vs Pie) + storage (eMMC vs UFS) advice in analysis
- **Guided ROM/GSI install** (`flash-system`, TUI menu): image checks (size, arm64, A-only), double confirmation, `fastboot flash system`, eRecovery wipe guidance — never auto-wipes userdata
- **TWRP path** (guide + guided flash): device-exact builds, image validation, automatic slot backup, explicit shared-slot warning (TWRP ↔ Magisk overwrite each other), Vol-Up boot, never TWRP userdata wipe
- **Root methods by priority** (Magisk preferred): patched recovery_ramdisk → Magisk-via-TWRP → phh-su → KernelSU (v0.9.2 only), selectable in TUI / `root-methods` CLI
- **Bootloader unlock + kernel/fix guides** straight from the P10 wiki (PotatoNV USER/BL LOCK flow, permissive kernels, speaker/APTouch fixes, TWRP rules)
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
Afterwards all tools are globally reachable. `Start-TrebleToolkit.bat` reopens
itself elevated automatically and runs the setup first if tools are missing.
Relative script calls always use `.\` prefix (some PowerShell versions require it).

Linux/bash: no setup file needed — the first start **guides you**: missing
`adb`/`fastboot` offer `[1]` official download into PATH, `[2]` pick a binary
(folder gets scanned), `[3]` pick each binary, `[4]` custom folder as PATH
(+ symlinks into central `tools/`), then the scrcpy question (install hint /
pick / skip). Works identically from `curl ... | bash` (reads `/dev/tty`,
zero files needed).

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

See [Download (offline ZIP — step by step)](#download-offline-zip--step-by-step):
download + verify + extract, then double-click `Start-TrebleToolkit.bat`.

## Instant Execute (zero files — paste one command)

No download, no ZIP: paste one command, the rest is automatic. Every variant
downloads the **full release ZIP** (same layout as the offline ZIP: `scripts\`,
`data\` registry, setup, TUI), verifies SHA256, and launches the **central
starter `Start-TrebleToolkit.bat`** — an online run is 100% identical to a ZIP
run. Nothing ever exits silently; every failure pauses with plain text.

### CMD (Windows Eingabeaufforderung) — ein Befehl, Enter, fertig

```cmd
curl -fsSL -o "%TEMP%\Run-FromGitHub.bat" https://raw.githubusercontent.com/mleem97/trebleManager/main/Run-FromGitHub.bat && "%TEMP%\Run-FromGitHub.bat"
```

(`curl.exe` is built into Windows 10/11. Without it, use the PowerShell
one-liner below — it does the same via `iwr`.) Caches under
`%LOCALAPPDATA%\trebleManager\<tag>`; delete that folder to force a fresh
download.

### PowerShell (Windows) — ein Befehl

```powershell
$b="$env:TEMP\Run-FromGitHub.bat"; iwr -UseBasicParsing -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/Run-FromGitHub.bat' -OutFile $b; & $b
```

Same result as the CMD variant (runs inline in your console).

### PowerShell `irm | iex` (no temp file at all)

```powershell
irm https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1 | iex
```

The script self-bootstraps (fetches the full ZIP, SHA256-verified, relaunches
from it). With CLI args this way? Download the file first, then call it with
`.\Treble-Toolkit.ps1 detect --json` — `iex` takes no arguments.

### Bash (Linux/macOS) — ein Befehl

```bash
curl -fsSL https://raw.githubusercontent.com/mleem97/trebleManager/main/run-from-github.sh -o /tmp/run-from-github.sh \
&& chmod +x /tmp/run-from-github.sh && /tmp/run-from-github.sh
```

Caches under `~/.local/share/trebleManager/<tag>`, verifies SHA256, then
`exec`s `scripts/treble-toolkit.sh`. Stays interactive (reads `/dev/tty`).
Pure pipe form (also self-bootstrapping):

```bash
curl -fsSL https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/treble-toolkit.sh | bash -s -- detect --json
```

First run on a fresh machine: use `Setup-TrebleToolkit.bat` once (auto-UAC,
execution policy, installs ADB/fastboot + optional scrcpy into user PATH,
saves `data/config.json`). The central starter offers this automatically.

## Download (offline ZIP — step by step)

For machines without GitHub access (USB-stick transfer).

1. Open <https://github.com/mleem97/trebleManager/releases/latest> and
   download **both** files of the top release:
   `trebleManager-vX.Y.Z.zip` **+** `trebleManager-vX.Y.Z.zip.sha256`.
2. Verify the hash (must match, otherwise delete and re-download):
   - PowerShell: `(Get-FileHash .\trebleManager-vX.Y.Z.zip -Algorithm SHA256).Hash -eq ((Get-Content .\trebleManager-vX.Y.Z.zip.sha256) -split '\s+')[0]` → must print `True`
   - CMD: `certutil -hashfile trebleManager-vX.Y.Z.zip SHA256` → compare the
     printed hash visually with the content of the `.sha256` file
   - Linux: `sha256sum -c trebleManager-vX.Y.Z.zip.sha256` → must print `OK`
3. Extract anywhere (path without spaces preferred), keep the folder layout
   (`scripts\`, `data\`, `logs\`, `backups\`):
   - Windows Explorer: right-click → Extract all, or PowerShell:
     `Expand-Archive -Path .\trebleManager-vX.Y.Z.zip -DestinationPath C:\trebleManager`
   - Linux: `unzip trebleManager-vX.Y.Z.zip -d trebleManager`
4. **Double-click `Start-TrebleToolkit.bat`** — the one central entry point
   (admin rights, setup, TUI). Nothing else needs opening.
5. Optional for fully offline use: pre-place the full EMUI firmware ZIP in
   `data\firmware\` and the Magisk APK in `data\magisk\` — then no internet
   is needed at all.

Read-only quick check without the toolkit: put `p10-magisk-check-FIXED.bat`
next to `adb.exe` and double-click it.

## Releases (immutable)

New version = new release — published artifacts are never modified.
Each release lives on its own branch + tag:

- [`release/v2.12.1`](https://github.com/mleem97/trebleManager/tree/release/v2.12.1) ([tag `v2.12.1`](https://github.com/mleem97/trebleManager/releases/tag/v2.12.1))
- [`release/v2.12.0`](https://github.com/mleem97/trebleManager/tree/release/v2.12.0) ([tag `v2.12.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.12.0))
- [`release/v2.11.0`](https://github.com/mleem97/trebleManager/tree/release/v2.11.0) ([tag `v2.11.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.11.0))
- [`release/v2.10.0`](https://github.com/mleem97/trebleManager/tree/release/v2.10.0) ([tag `v2.10.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.10.0))
- [`release/v2.9.0`](https://github.com/mleem97/trebleManager/tree/release/v2.9.0) ([tag `v2.9.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.9.0))
- [`release/v2.8.0`](https://github.com/mleem97/trebleManager/tree/release/v2.8.0) ([tag `v2.8.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.8.0))
- [`release/v2.7.0`](https://github.com/mleem97/trebleManager/tree/release/v2.7.0) ([tag `v2.7.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.7.0))
- [`release/v2.6.0`](https://github.com/mleem97/trebleManager/tree/release/v2.6.0) ([tag `v2.6.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.6.0))
- [`release/v2.4.0`](https://github.com/mleem97/trebleManager/tree/release/v2.4.0) ([tag `v2.4.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.4.0))
- [`release/v2.2.0`](https://github.com/mleem97/trebleManager/tree/release/v2.2.0) ([tag `v2.2.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.2.0))
- [`release/v2.1.0`](https://github.com/mleem97/trebleManager/tree/release/v2.1.0) ([tag `v2.1.0`](https://github.com/mleem97/trebleManager/releases/tag/v2.1.0))

Process: [RELEASE.md](RELEASE.md). Run an old version via tag checkout or the
release ZIP attached to its GitHub Release (verify `.sha256` first).

## Orchestrator (goals, planner, state)

Instead of memorizing commands, pick a **goal** (TUI menu or
`workflow --goal <id> [--json]`): `root`, `custom_rom`, `stock_rom`,
`root_custom_rom`, `root_stock_rom`, `root_custom_rom_recovery`,
`root_stock_rom_recovery`, `restore_original`, `full_reinstall`. The planner prints the ordered
steps with live gate results first; running executes them with persisted state
(`logs/workflow-state.json`) and controlled stop on failure
(diagnostic → restore → abort). Resume with the menu entry or CLI `resume`.

- **Preflight** runs first: missing global tools (adb/fastboot) **block the
  main menu** with a fix path (Setup/install), they never fail silently later.
- **Device states are explicit**: `ADB_READY`, `ADB_UNAUTHORIZED`,
  `ADB_OFFLINE`, `ADB_MULTIPLE_DEVICES`, `FASTBOOT_READY`, ... — unauthorized
  or offline is never reported as "absent"; multiple devices require explicit
  target selection (`ANDROID_SERIAL`).
- **`status [--json]`** exposes the complete machine-readable state
  (preflight, devices, mode, goal, saved steps) for automation.
- `--yes` / unattended never bypass prerequisites or safety gates.

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
Treble-Toolkit.ps1 detect|devices|analyze|firmware|download|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|validate|verify|restore|diagnostic|dump-partitions|dump-properties|dump-vendor|dump-logs|preflight|recon|status|workflow|resume|root|wizard|help [--goal <id>] [--mode safe|unattended|developer] [--json] [--yes] [--image <path>] [--firmware-file <url|path>] [--anonymize] [--no-reboot]
```

Wizard order: Detect → Analyze → Firmware → Extract → Patch → Backup → Flash → Reboot+Verify.
ROM path additionally: `export` → `flash-system` (guided, verified profiles only).
Flash/restore/system-flash need double confirmation (`FLASH`+`YES` / `RESTORE`+`YES`, CLI: `--yes`).
Details: [INSTRUCTIONS.md](INSTRUCTIONS.md), [QUICKSTART.md](QUICKSTART.md), [FAQ.md](FAQ.md).

## Repository Layout

```
├── scripts/                # Treble-Toolkit.ps1 (Windows TUI+CLI, PS 5.1/7) + treble-toolkit.sh (Linux bash port) + Setup-Windows.ps1
├── tests/                  # Test-Parsers.ps1 + test-parsers.sh (parser/firmware/OS/hash/root/URL/magic tests)
├── data/firmware/          # Full firmware drops (UPDATE.APP source, offline cache)
├── data/magisk/            # Magisk APK + to-patch staging + pulled patched images
├── data/roms/              # Custom ROM packages for recovery export
├── data/recovery/          # Exported recovery/boot images + metadata
├── data/compatibility/     # Researched per-variant matrix (YAML source + JSON mirror)
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
├── FAQ.md
├── ARCHITECTURE.md
├── SECURITY.md
├── TROUBLESHOOTING.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SUPPORT.md
├── .github/                # FUNDING, issue templates, PR template, CODEOWNERS
├── CHANGELOG.md
├── ROADMAP.md
├── RELEASE.md              # Immutable release process (branch+tag per version)
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
