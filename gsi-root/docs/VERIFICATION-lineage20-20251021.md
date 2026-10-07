# Verification record — lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed

> **DRY RUN** — everything below runs without flashing or touching any
> device (downloaded bytes + static analysis only). Boot, root and hardware
> behavior need the P10 and are NOT covered here.

Reference GSI for the P10 permanent-root POC. **Static verification only**
(no phone here): everything below was measured from the downloaded bytes
with `ttcore`/`gsi-root`, `debugfs`, `file`, `xxd`. Boot, root and hardware
behavior are NOT proven by this document — that needs the P10.

## Artifact

- File: `lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz`
- Size: 1245119669 bytes (~1.2 GB)
- SHA-256: `48c7c37948c08a5109b599bd0f769d4b6ee69312b200d3005a0e3fa8b97de787`
- Source: SourceForge `andyyan-gsi`, folder `lineage-20-td` (listing +
  302-redirect verified 2026-10-07)
- Decompressed: `system.img`, 2805805056 bytes, **raw ext4** (not sparse)

## Measured structure

| Check | Result | Method |
|---|---|---|
| Container | gzip | `gsi-root analyze` |
| Filesystem | ext4, UUID 673e67c6-… | `file`, superblock |
| Sparse | no | header absent |
| Android | 13 (SDK 33) | `system/build.prop` |
| Lineage version | `20.0-20251021-UNOFFICIAL-arm64_bgN` | `ro.lineage.version` (matches filename + registry) |
| Device codename | `tdgsi_arm64_ab` | `ro.lineage.device` |
| SAR layout | yes (`/init` symlink, `/system/` dir, `ANDROID_ROOT /system`) | root listing |
| `init.environ.rc` | present, `on early-init` | file read (PHH hook vector exists) |
| AVB footer | **none** (`AVB0` absent in last 128 B) | tail scan — device vbmeta chain still applies |

## Tool behavior on this file (proves the GSI-vs-device logic)

- `gsi-root analyze` → container gzip, engine `MagiskStyleEarlyInit
  (Experimental)`, reason: hardware POC pending. Honest, no fake success.
- `export` (patch-base path) → **refused as SYSTEM** with the stock-recovery
  guidance. Correct: a system image can never be a Magisk patch base, and
  this GSI leaves `recovery_ramdisk` untouched stock.

## What "working" still requires (phone needed)

1. `fastboot flash system system.img` on VTR-L09/L29 (EMUI 9.1 base) → boots.
2. Normal power-boot → Android 13 usable (SIM/audio/camera per P10 notes).
3. POC root mechanism → `su` → `uid=0`, reboot + cold boot persistence.

Until then the registry status rests on the documented P10 reports
(discussion #2542, wiki) plus the user's running device — not on this file
analysis. This record exists so any future change can be diffed against
measured ground truth (hashes above).
