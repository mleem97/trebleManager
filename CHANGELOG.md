# Changelog — trebleManager

## v2.14.2

- Docs consistency: INSTRUCTIONS wizard flow rewritten ROM-aware (goal words, visible SKIPs, ROM patch-base rule); removed last stale "stock source still needed" statement; wiki synced (Root-Guide ROM question, FAQ patch-base rule, Troubleshooting verdicts + online-run, Home central starter)

## v2.14.1

- Fixed `irm | iex` (no file path): new `Get-TTScriptRoot` (checks before splitting, `$PSScriptRoot`-based so it also works inside functions — `Get-TTToolRoot` used `$MyInvocation` inside a function and failed even via `-File`); test bootstrap/import use the same guarded resolution
- Fixed relaunch splat `@$__fw` → `@__fw` (literal `@` was passed as command → "Unknown command: @")
- Fixed test imports: functions are now defined `script:`-scoped (previously they vanished after `Import-TTFunction` returned → "not recognized" cascade); missing imports abort immediately with file path instead of cascading

## v2.14.0

- Installed-ROM question: wizard/analyze ask which system is on the phone (Stock EMUI, supported registry ROM, other; researched-broken shown but not selectable), persisted in `data/installed-rom.txt`, shown in header/status/CLI (`rom`, `rom list/set/clear`)
- Patch-base rule enforced: custom ROMs patch ONLY from their own ROM package (recovery export), with unmistakable red rule boxes; backup/restore/fake-patch check use the same base; fixed wrong "stock source still needed" hint
- ROM-aware wizard: goal in plain words (root only / install ROM / back to stock), inapplicable steps shown as `[SKIP]` with reasons instead of running

## v2.13.2

- Bootstrap no longer needs the GitHub API in the normal path (fixes persistent 403s from missing UA or rate limits): tools fetch their own version's ZIP directly, tests/launchers read raw `VERSION`, API stays only as fallback; shared `bootstrap_fetch`/`Get-BootstrapReleaseFile` helpers

## v2.13.1

- Fixed API 403 in PowerShell bootstraps: `Net.WebClient` sent no User-Agent, which `api.github.com` rejects — both PS1 bootstraps (tool + tests) now send `User-Agent: trebleManager` (curl/BAT already did)
- Failed test bootstrap now exits 1 with a clear message instead of running degraded into cryptic follow-on errors; README gains robust download-then-run test variants

## v2.13.0

- Test suites run online: `Test-Parsers.ps1` (`irm ... | iex`) and `test-parsers.sh` (`curl ... | bash`) self-bootstrap the full SHA256-verified release ZIP and run from it (loop-guarded, mismatch aborts); README documents both one-liners

## v2.12.2

- Fixed faulty flash verdicts (all 8 flash/erase sites, PS1+bash): new `Get-FlashVerdict`/`flash_verdict` evaluator — FAILED lines veto everything, progress words (Writing/Erasing) alone are NOT success, OK requires OKAY + Finished line; screen now shows `FLASH RESULT: OK/FAILED/UNCLEAR` with OKAY count, total time, failing lines + cause hints (full raw output stays in the log)

## v2.12.1

- README execution guide: new `Instant Execute` section (paste-ready one-liners for CMD via curl.exe, PowerShell via iwr/iex, Bash incl. pipe form) and rewritten `Download (offline ZIP — step by step)` (numbered: download, verify per shell incl. certutil, extract commands, central starter); stale hardcoded ZIP filename removed

## v2.12.0

- 100% remote execution: both scripts self-bootstrap — `irm ... | iex` and `curl ... | bash` detect the missing layout, fetch the full SHA256-verified release ZIP (cached, loop-guarded via `TT_BOOTSTRAPPED`), forward CLI args, and relaunch from it; SHA mismatch deletes + aborts, offline warns degraded instead of crashing
- README fallback section rewritten (no longer degraded)

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
