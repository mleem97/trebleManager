# Changelog — trebleManager

## v2.11.0

- Online run = ZIP run: `Run-FromGitHub.bat` now bootstraps the FULL release ZIP (latest via API, SHA256-verified, cached in `%LOCALAPPDATA%\trebleManager`) and always chains into the central starter `Start-TrebleToolkit.bat` — no more single-PS1 degradeds; new `run-from-github.sh` does the same on Linux/macOS
- One central entry point documented everywhere: double-click `Start-TrebleToolkit.bat` (admin, setup, TUI); README download section + rewritten online-run docs, QUICKSTART points at the starter
- Launcher tests: 11 new checks (central starter, online chain, hash verify, sh syntax, docs)

## v2.10.0

- Full reinstall flow: `full_reinstall` goal + TUI `Full reinstall` menu (custom GSI/system.img via Install-ROM, honest stock path via HiSuite/service flow), optional guided wipe before flash
- Guided wipe: `wipe` CLI/TUI, double-confirmed (`WIPE` + `YES`/`JA`, or `--yes`), userdata only, eRecovery fallback when the device refuses `erase`
- Preflight ready display: green `PREFLIGHT READY (all green)` summary on pass, red `[NOT READY]` lines when blocked
- Starter hardening: `Start-TrebleToolkit.bat` never exits silently (UAC retry, setup call, `launcher.log`, pauses with exit codes)

## v2.9.0

- Guided preflight: missing tools no longer dead-end — `[1]` official install into PATH, `[2]` pick binary + folder scan, `[3]` pick each binary, `[4]` custom folder as PATH (+ symlinks into central tools), scrcpy finale question, `setup` CLI, saved config
- Zero-file pipe runs fixed (`BASH_SOURCE` fallback, `/dev/tty` reads)
- Preflight screen offers to run setup directly, then continues on success

## v2.8.0

- Root persistence: service.d boot scripts (aptouch + speaker, reboot-proof) via menu/CLI `persist` (needs live root once); persisted boot-mode state (cheat/persistent)
- Persistent Magisk boot guidance for no-ramdisk devices (discussion steps 13/14: set/clear + verify), double-confirmed, reversible
- Audio debugging after Magisk modules (Viper4Android): remove-modules rescue, check chain, prevention (wiki + FAQ)

## v2.7.0

- Orchestrator P0: tool registry, startup preflight gate (blocks menu, never silent), explicit ADB/Fastboot states (unauthorized/offline/multiple), multi-device target selection, per-step requirement gates
- Orchestrator P1: 8 workflow goals + planner with live gates, persistent state + resume, controlled failure flow (diagnostic/restore/abort), JSON status API
- CLI: `preflight|recon|status|workflow|resume|root` (+ `--goal`, `--mode`)
- Fixed bash value/command arg collision (`--goal root` etc.)

## v2.6.0

- Auto-elevation: script reopens itself in a new admin window (UAC) unless already admin (`-NoElevateCheck` or remote session opts out)
- First-run setup: missing adb/fastboot triggers path question or PATH install (tools + optional scrcpy) with saved `data/config.json`; launcher runs setup automatically
- Path quoting hardened (`Unquote-Path` for drag-drop/quoted paths with spaces, quoted `-File` invocations)

- Run modes: `--mode safe|unattended|developer` (unattended still enforces all safety gates; developer unlocks dump commands)
- Post-flash validation: `validate` (TUI Tools + CLI) with system/hardware checks + `validation-<stamp>.json` report
- Developer dumps: `dump-partitions|properties|vendor|logs` (developer mode, read-only, for ROM research)
- ROADMAP.md (state-machine, firmware repo, core extraction plan)
- Immutable releases enforced: version-consistency tests, RELEASE.md, per-version branches + tags

- Compatibility registry (`data/compatibility/huawei/p10/*.yaml` + JSON): researched ROM/firmware/TWRP/Magisk matrix, `compat` screen/CLI, broken-build hard block, vendor + storage advice
- Fixed `$Args` auto-variable shadowing (bare-adb help-text bug) + help-text guard

## v2.4.0

- TWRP path: guide (device-exact builds, XDA/wiki rules) + guided flash with slot backup, shared-slot warning, Vol-Up boot, CLI `twrp`
- Root methods by priority (Magisk preferred): recovery patch → via-TWRP → phh-su → KernelSU, selectable in TUI, `root-methods` CLI
- Generic device support: verified vs unverified profiles (flash blocked unless verified), `devices` CLI, Kirin 960 family (VTR-AL00, VKY-L09) + GENERIC-TREBLE fallback
- LineageOS as starting point (version detection, assisted baseline, export from Lineage zips)

## v2.3.0

- Full guidance: root wizard + ROM/GSI install (`flash-system`, TUI menu) + custom ROM flows
- Bootloader unlock + kernel/fix guides from the P10 wiki; Android 13 instability warning on P10/Plus
- LineageOS as starting point (version detection, assisted baseline, export from Lineage zips)
- Generic device support: verified vs unverified profiles (flash blocked unless verified), `devices` CLI, Kirin 960 family (VTR-AL00, VKY-L09) + GENERIC-TREBLE fallback

## v2.2.0

- Linux bash port (`scripts/treble-toolkit.sh`): same wizard/CLI logic, zero extra
  dependencies (only adb, fastboot, coreutils; optional unzip/curl/zip/python3),
  bilingual UI (EN default, DE if `$LANG` starts with `de`), `tests/test-parsers.sh`
- macOS bash port planned next (same script, kept mac-bash3 compatible: no assoc arrays)

## v2.1.0

- Bilingual TUI (English default, German if system language is German)
- Recovery export from compatible custom ROMs (direct `.img`, ROM `.zip`, `payload.bin` via payload-dumper-go; GSI system images refused honestly) + CLI `export`
- Stock firmware downloader with progress (BITS resume + WebClient fallback) and mandatory confirmation + CLI `download`
- Full CLI: `detect|analyze|firmware|download|extract|export|patch|backup|flash|verify|restore|diagnostic|wizard|help` with `--json`
- 9-point safety gate with `DO NOT FLASH`, derived flash command, double confirmation
- Backup/restore with `metadata.json` + SHA-256/512, diagnostic ZIP (10 files, `--anonymize`)
- Docs in gregCore style: README, INSTRUCTIONS, QUICKSTART, ARCHITECTURE, SECURITY, TROUBLESHOOTING, CONTRIBUTING, CODE_OF_CONDUCT

## v2.0.0

- Complete PowerShell TUI + CLI wizard (steps 1–9) for VTR-L29/VTR-L09/VKY-L29
- OS-independent analysis (Stock EMUI, TrebleDroid/Lineage GSI, custom ROMs)
- Fixed prerequisite checker BAT (auto Android/fastboot mode, desktop log)
- Smart launchers (local elevation + direct-from-GitHub loader)
