# FAQ — trebleManager

## Do I need the original RECOVERY or the one from my custom ROM?

The **original** one — always. Magisk is patched into the stock Huawei
`RECOVERY_RAMDISK.img` from the matching EMUI 9.1 full firmware (`UPDATE.APP`),
**not** into anything from your custom ROM or GSI. Reasons:

- GSI/system images contain **no** Huawei `recovery_ramdisk` at all.
- The P10 has no ramdisk in `boot` since EMUI 9, so `boot.img` patching cannot work.
- After flashing, the phone still boots your installed GSI — only the recovery
  ramdisk carries Magisk (`fastboot flash recovery_ramdisk magisk_patched.img`).

The **Recovery export** feature (`export`, TUI menu) extracts `boot`/`recovery.img`
from custom ROM packages for reference, backup, or other devices — for the P10
Magisk path the tool still routes you to the stock image and says so explicitly.

## Does the software guide me through the whole root process?

Yes — wizard steps 1–9 (Detect → Analyze → Firmware → Extract → Patch → Backup →
Flash → Reboot+Verify), or the same steps as CLI commands. It never skips silently:
every gate, hash, and confirmation is shown. Three things stay manual by design:

1. Patching happens **in the Magisk app on the phone** (`Select and Patch a File`)
   — only the on-device app builds a 100% fitting Kirin 960 patch.
2. Every boot **with** root needs **Vol-Up + Power until the Huawei logo**
   (Magisk boot cheat, not persistent).
3. Flash/restore need typed double confirmation (`FLASH`+`YES` / `RESTORE`+`YES`).

## Can I install a complete custom ROM with it, with or without root?

No — and that is intentional. The tool **never flashes `system`, `vendor`, or
`userdata`**, so it cannot install a full ROM and cannot wipe your GSI.
Install the GSI yourself per
[Discussion #2542](https://github.com/phhusson/treble_experimentations/discussions/2542)
and the [P10 wiki](https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus),
then trebleManager roots the running system (or verifies it stays unrooted —
root itself is always your choice at the confirmation prompt).

## Can I restore my original firmware with it?

Partly: **Restore / Unroot** flashes the backed-up original `recovery_ramdisk`
back (hash-verified, same safety rules) — that unroots the boot chain completely.
A **full** stock return (`system`/`vendor` reflash) is out of scope for the tool;
do that via HiSuite, eRecovery, or fastboot with the full firmware package
(see [INSTRUCTIONS.md](INSTRUCTIONS.md), downloader + `data/firmware/`).

## Touchscreen edges dead (APTouch issue) — how to fix?

Known Kirin-960/GSI quirk (see [P10 wiki](https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus)).
Needs root first (verify shows `ROOTED`, `uid=0`), then as root:

```
adb shell su -c "stop aptouch"
```

Persistent options: a Magisk boot script, or the community fix module
([GSI_Generic_Fix for Huawei P10](https://github.com/Coconutat/GSI_Generic_Fix_Magisk_Module_For_Huawei_P10),
also fixes the speaker issue). Speaker-only variant as root:

```
adb shell su -c "chown root:audio /dev/nxp_smartpa_dev; chmod 0660 /dev/nxp_smartpa_dev"
```

## Is an unlocked bootloader required?

Yes. The tool never unlocks anything itself (no `flashing unlock`, no `oem unlock`,
ever). If a TrebleDroid/Lineage GSI already boots on your P10, the unlock is done
(PotatoNV: [mashed-potatoes/PotatoNV](https://github.com/mashed-potatoes/PotatoNV)).
Fresh stock devices must be unlocked first — the tool detects the state but stops
before anything destructive.

## `FAILED (remote: Command not allowed)` — is my bootloader locked?

No conclusion possible from that line alone — Huawei fastboot refuses several
standard `getvar` queries on unlocked devices too. The tool treats it as a Huawei
quirk and continues with `/dev/block/by-name`, `ro.boot.*`, and `/proc/cmdline`
instead of guessing a lock state.

## Which Magisk version should I use?

No blind "latest". Orientation (always cross-check the
[official changelog](https://github.com/topjohnwu/Magisk/releases)):

| Android | Tested |
|---|---|
| 8.x (EMUI 8) | Magisk v25+ |
| 9.x (EMUI 9/9.1) | Magisk v25+ |
| 10–12 (GSI/custom) | Magisk v26+ |
| 13–14 (TrebleDroid/Lineage GSI) | Magisk v27/v28+ per changelog |

Only official GitHub APKs; version + SHA-256 are documented by the tool.

## Do I need Vol-Up + Power on every boot?

Only for boots **with** root. Normal power-on boots stock (unrooted) — that is the
documented Magisk-recovery behavior on this device, not a bug. No persistent
switch exists; the eRecovery byte tricks from #2542 are shown in the tool under
boot tricks.

## Will I lose data?

The tool itself never wipes (`erase`/`format`/`userdata` appear nowhere in its
flash path). Still: back up your data before touching the boot chain, and only
factory-reset from stock eRecovery if **you** choose to (never from TWRP on this
device — it breaks userdata per the wiki).

## What if Android no longer boots after flashing?

Stay calm, stay in fastboot: TUI **Restore / Unroot** (or CLI `restore`) writes
the hash-verified original `recovery_ramdisk` back, then `fastboot reboot`.
That is the Safe Restore Mode — one partition, nothing else.

## My GSI stays untouched, really?

Yes. The only partition the tool ever writes on the P10 profile is
`recovery_ramdisk`, derived from the device profile and reconfirmed by the
9-point safety gate (`DO NOT FLASH` on any FAIL).

## Where are logs? What do I send for help?

Every run writes `logs/toolkit-<stamp>.log`. For help requests create
`diagnostic` (CLI) or Tools → diagnostic ZIP (TUI) — 10 files, use
`--anonymize`, redact serials. See [SUPPORT.md](SUPPORT.md).

## Does it work offline?

Yes, after one-time placement: full firmware ZIP → `data/firmware/`, Magisk APK →
`data/magisk/`, ROM packages → `data/roms/`, extractors → `data/tools/`.
Internet is then only used for explicit downloads you confirm.

## Do I need scrcpy?

No — optional screen mirroring while rooting. The tool detects it at startup
(`detect` shows the path) and offers one-click mirroring from Tools. Absence is
just an INFO line. Source: [Genymobile/scrcpy](https://github.com/Genymobile/scrcpy).

## Which devices are supported?

Huawei P10 VTR-L29 (primary), VTR-L09, P10 Plus VKY-L29 — arm64, EMUI 9.1 base.
Other Kirin devices only via new profiles with real device data
(see the `device-support` issue template) — never assumed.
