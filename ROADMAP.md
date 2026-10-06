# Roadmap — trebleManager

Starting point: the working toolkit in this repo (TUI + CLI, device profiles,
compatibility registry, safety gate, Linux bash port). This plan extends it —
it does not rewrite it.

## 1. Where we are

- Wizard/CLI: detect → analyze → firmware → download → extract → export →
  patch → backup → flash → verify, plus flash-system, twrp, restore,
  diagnostic, compat, root-methods, devices.
- Rules already enforced: no blind flashing, `DO NOT FLASH` gate, backup
  before flash, on-device Magisk patch only, `uid=0` as the only root proof,
  Huawei `Command not allowed` is UNKNOWN (never LOCKED).

## 2. Run modes (done: v2.5.0)

- `safe` (default): every irreversible step is confirmed interactively.
- `unattended` + `--yes`: auto-confirms a **fully validated** run.
  Safety gates still run — `--yes` never skips checks.
- `developer`: unlocks `dump-partitions|properties|vendor|logs` for ROM research.

## 3. Post-flash validation (done: v2.5.0)

`validate` (TUI Tools + CLI) runs read-only system/hardware checks
(boot, ADB, OS, SELinux, root, mounts, Wi-Fi, Bluetooth, battery, sensors)
and writes `logs/validation-<stamp>.json`. Next: per-check remediation hints.

## 4. Wizard as state machine (planned)

`UNKNOWN → DETECTED → ANALYZED → COMPATIBLE → BACKED_UP → IMAGE_PREPARED →
READY_TO_FLASH → FLASHED → BOOTED → VERIFIED`, with
`BOOT_FAILED → DIAGNOSTIC → RESTORE_REQUIRED` recovery. Persist step state so
interrupted runs resume instead of restarting.

## 5. Firmware as a repository (planned)

Per-package metadata (model, CUST, region, EMUI, Android, build, source, URL,
SHA-256/512, extraction status, known compatibility) under `data/firmware/`,
so inputs are reproducible artifacts, not loose downloads.

## 6. Core extraction (planned, stepwise)

PowerShell stays the reference implementation (direct-from-GitHub run must
keep working). Extract pure logic toward `core/` (device, adb, fastboot,
firmware, partitions, images, magisk, flashing, backup, verification) first;
only then consider a native Rust core with PS1/bash as thin wrappers.
CLI command names stay stable across the migration.

## 7. Standing rules (never change)

Unknown is never compatible. No automatic unlock/wipe/format, no force or
verity-disable flags, no foreign patched images, no fake success paths.
New version = new immutable release (see [RELEASE.md](RELEASE.md)).
