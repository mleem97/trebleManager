# FAQ — trebleManager

## Which root method should I use? Is Magisk preferred?

Yes — Magisk patched `recovery_ramdisk` is the preferred method (modules,
systemless hosts, verified path). The tool lists all compatible methods by
priority (TUI menu, `root-methods` CLI):

1. **Magisk recovery_ramdisk** (preferred) — verified, full features.
2. **Magisk via TWRP zip** — needs a device-exact TWRP; shares the same slot
   (flashing one overwrites the other, restore switches back).
3. **phh superuser** — legacy, enough for AdAway per #2542 comments, no modules.
4. **KernelSU** — experimental, Kirin supports v0.9.2 only.

## How do I get TWRP on the device?

Menu **TWRP path** (or CLI `twrp --image <twrp.img> --yes`): use only a TWRP
built for your exact model (e.g. XDA P10 Plus TWRP 3.2.1-0 oreo thread). The tool
validates the image (size, ANDROID! magic), backs up the current slot
automatically, warns about the shared Magisk slot, requires the double
confirmation, then runs `fastboot flash recovery_ramdisk twrp.img`. Boot it by
holding Vol-Up; leave system unmodified when asked; **never** factory-reset
userdata from TWRP (use stock recovery).

## Do I need the original RECOVERY or the one from my custom ROM?

The **original** one — always. Magisk is patched into the stock Huawei
`RECOVERY_RAMDISK.img` from the matching EMUI 9.1 full firmware (`UPDATE.APP`),
**not** into anything from your custom ROM or GSI. Reasons:

- GSI/system images contain **no** Huawei `recovery_ramdisk` at all.
- The P10 has no ramdisk in `boot` since EMUI 9, so `boot.img` patching cannot work.
- After flashing, the phone still boots your installed GSI — only the recovery
  ramdisk carries Magisk (`fastboot flash recovery_ramdisk magisk_patched.img`).

A running LineageOS is a fully valid **starting point** (the tool reads its
Lineage version and treats the hidden Huawei base as assisted baseline) — but the
Magisk **source** stays the stock image. Lineage/zips can additionally feed the
**Recovery export** (`boot.img` from ROM zips) for reference, backup, or other
devices.

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

Yes — guided, not blind. Menu **Install ROM / GSI system image** (or CLI
`flash-system --image <gsi.img> --yes`) checks the image (size, arm64, A-only —
A/B images are refused), requires the double confirmation, runs
`fastboot flash system`, then walks you through eRecovery wipe + first boot.
Rules from the P10 wiki apply: EMUI 8/9/9.1 base, back up storage first, reset
only via stock recovery (never TWRP wipe), slim builds for small system
partitions, MindTheGapps. Root afterwards is optional via the normal wizard
(same confirmations). Unverified device profiles cannot flash at all.

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

## Can root persist across normal reboots (no-ramdisk devices like P10)?

Partly — honestly split in two:

- **Rooted boot itself**: no safe bypass exists. Magisk lives in
  `recovery_ramdisk`, so a normal power-on boot is always unrooted. The tool
  documents the community **persistent-boot** option (discussion step 13: set a
  persistent recovery-boot byte via the eRecovery wipe trick, cleared again via
  step 14) under Boot tricks → persistent boot, with double confirmation,
  reversibility, and a verify step (normal reboot → `uid=0` check). It changes
  *every* boot by design — decide consciously.
- **What the tool does persist**: aptouch + speaker fixes as Magisk
  `service.d` boot scripts (menu Persist, needs live root once — no more manual
  adb after reboot), plus the verified root/boot-mode state across runs.
- **APTouch after every root**: yes, automatically. Every verified root
  (verify screen, wizard root paths) attempts the permanent APTouch fix:
  immediate `stop aptouch` now + `service.d` script for every rooted boot.
  Fully non-destructive (one service stopped, scripts removable), skips
  cleanly without live root and asks again next time.

## No sound after Viper4Android / Magisk modules?

Audio mods can silence the whole GSI even after app uninstall (module stays
active). Boot rooted, run `adb shell su -c "magisk --remove-modules"`,
reboot, reinstall working modules one by one. Debug chain + prevention:
wiki Troubleshooting → No sound. Last resort: tool Restore.

## Will I lose data?

Only if **you** choose it: TUI `Full reinstall` offers an optional guided wipe
before flashing, and CLI/TUI `wipe` erases userdata only after double
confirmation (`WIPE` + `YES`/`JA`, or `--yes`) — with an eRecovery fallback if
the device refuses `erase`. The root/flash paths themselves never wipe.
Still: back up your data before touching the boot chain, and only
factory-reset from stock eRecovery if **you** choose to (never from TWRP on this
device — it breaks userdata per the wiki).

## I have a custom ROM installed. Which image do I patch with Magisk?

The one **from that exact ROM** — never from stock firmware. The wizard asks
which system is on your phone (Stock EMUI, a supported ROM from the list, or
other) and remembers it. On a custom ROM it skips the stock firmware/download
steps visibly (`[SKIP]`) and takes the patch base from your ROM package via
recovery export (`data/roms/` → `data/recovery/`). A stock-based patched image
will NOT boot on a custom ROM. Change the answer anytime: Analyze → `[C]`, or
CLI `rom list` / `rom set <number>`.

## What if Android no longer boots after flashing?

Stay calm, stay in fastboot: TUI **Restore / Unroot** (or CLI `restore`) writes
the hash-verified original `recovery_ramdisk` back, then `fastboot reboot`.
That is the Safe Restore Mode — one partition, nothing else.

## My GSI stays untouched, really?

On the **root path**: yes. The only partition the tool writes there is
`recovery_ramdisk`, derived from the device profile and reconfirmed by the
9-point safety gate (`DO NOT FLASH` on any FAIL). The **ROM install path**
(`flash-system`) replaces `system` only after its own checks + double
confirmation — `userdata` is only ever wiped by the explicit guided `wipe`
(double-confirmed) or the optional wipe inside `Full reinstall`.

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
