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

## Can Magisk and TWRP persist at the same time?

No — on the P10 both live in the single `recovery_ramdisk` slot and
overwrite each other (this is a hardware fact, not a tool limit). What the
tool does instead: every flash records the slot occupant (magisk/twrp/stock,
shown in Status), and after each flash it offers the reverse flash as a
one-tap switch-back — no path re-entry. The APTouch fix (`service.d`) is
independent of the slot and persists across both.

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

It depends on the ROM type (the wizard asks which system is on your phone
and remembers it):

- **GSI / system-only ROM** (TrebleDroid, Lineage GSI, …): a GSI never touches
  `recovery_ramdisk`, so your recovery is still stock — the patch base **is**
  the stock Huawei `RECOVERY_RAMDISK.img`. Correct, not a workaround.
- **Full device ROM** (own boot/recovery in its package): the patch base
  **must** come from that exact ROM package via recovery export
  (`data/roms/` → `data/recovery/`). A stock-based patched image will NOT
  boot there.

Change the answer anytime: Analyze → `[C]`, or CLI `rom list` /
`rom set <number>`.

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

---

<a id="de"></a>
## Deutsch — FAQ

## Welche Root-Methode soll ich nehmen? Ist Magisk bevorzugt?

Ja — Magisk-gepatchtes `recovery_ramdisk` ist die bevorzugte Methode (Module,
systemless Hosts, verifizierter Pfad). Das Tool listet alle kompatiblen
Methoden nach Prio (TUI-Menü, `root-methods`-CLI):

1. **Magisk recovery_ramdisk** (bevorzugt) — verifiziert, volle Features.
2. **Magisk via TWRP-ZIP** — braucht exakt passendes TWRP; teilt sich den Slot
   (eines überschreibt das andere, Restore wechselt zurück).
3. **phh superuser** — legacy, reicht für AdAway per #2542-Kommentare, keine Module.
4. **KernelSU** — experimentell, Kirin nur v0.9.2.

## Wie bekomme ich TWRP aufs Gerät?

Menü **TWRP-Pfad** (oder CLI `twrp --image <twrp.img> --yes`): nur TWRP exakt
für dein Modell nehmen (z.B. XDA P10 Plus TWRP 3.2.1-0 Oreo-Thread). Das Tool
validiert das Image (Size, ANDROID!-Magic), sichert den aktuellen Slot
automatisch, warnt vor Shared-Magisk-Slot, braucht Doppel-Bestätigung, dann
`fastboot flash recovery_ramdisk twrp.img`. Booten mit Vol-Up-Halten; System
unverändert lassen wenn gefragt; **niemals** Factory-Reset aus TWRP (Stock-
Recovery nutzen).

## Können Magisk und TWRP gleichzeitig persistent sein?

Nein — auf dem P10 leben beide im selben `recovery_ramdisk`-Slot und
überschreiben einander (Hardware-Fakt, kein Tool-Limit). Stattdessen merkt
sich das Tool jeden Flash als Slot-Bewohner (magisk/twrp/stock, in Status)
und bietet nach jedem Flash den Gegen-Flash als One-Tap-Switch an — keine
Pfad-Neueingabe. Der APTouch-Fix (`service.d`) ist slot-unabhängig und
persistiert über beide.

## Brauche ich das originale RECOVERY oder das aus meiner Custom-ROM?

Kommt auf den ROM-Typ an (Wizard fragt, welches System auf dem Handy ist,
und merkt es sich):

- **GSI / System-only-ROM** (TrebleDroid, Lineage-GSI, …): Ein GSI fasst
  `recovery_ramdisk` nie an, dein Recovery ist weiter Stock — Patch-Basis
  **ist** das Stock-Huawei-`RECOVERY_RAMDISK.img`. Korrekt, kein Workaround.
- **Volles Device-ROM** (eigenes boot/recovery im Paket): Patch-Basis
  **muss** aus exakt diesem ROM-Paket kommen, via Recovery-Export
  (`data/roms/` → `data/recovery/`). Ein Stock-basiertes Image bootet dort
  NICHT.

Antwort jederzeit ändern: Analyze → `[C]`, oder CLI `rom list` /
`rom set <Nummer>`.

## Führt mich die Software durch den kompletten Root-Prozess?

Ja — Wizard (Detect → Analyze mit ROM-Frage → Ziel in Alltagssprache → Plan
mit sichtbaren `[SKIP]`s → laufen), oder dieselben Steps als CLI-Commands.
Nie wird still geskipt: Jedes Gate, jeder Hash, jede Bestätigung wird gezeigt.
Drei Dinge bleiben by design manuell:

1. Patchen passiert **in der Magisk-App am Handy** (`Select and Patch a File`)
   — nur die On-Device-App baut einen 100 % passenden Kirin-960-Patch.
2. Jeder Boot **mit** Root braucht **Vol-Up + Power bis Huawei-Logo**
   (Magisk-Boot-Cheat, nicht persistent).
3. Flash/Restore brauchen getippte Doppel-Bestätigung (`FLASH`+`YES` /
   `RESTORE`+`YES`).

## Kann ich damit ein komplettes Custom-ROM installieren, mit/ohne Root?

Ja — geführt, nicht blind. Menü **ROM / GSI installieren** (oder CLI
`flash-system --image <gsi.img> --yes`) prüft das Image (Size, arm64, A-only —
A/B-Images abgelehnt), braucht Doppel-Bestätigung, macht
`fastboot flash system`, danach eRecovery-Wipe + Erstboot-Anleitung. P10-Wiki-
Regeln gelten: EMUI-8/9/9.1-Basis, erst Storage sichern, Reset nur via
Stock-Recovery (nie TWRP-Wipe), Slim-Builds für kleine System-Partitionen,
MindTheGapps. Root danach optional via normalem Wizard (gleiche
Bestätigungen). Unverifizierte Profile können gar nicht flashen.

## Kann ich damit meine Original-Firmware restoren?

Teils: **Restore / Unroot** flasht das gesicherte originale
`recovery_ramdisk` zurück (hash-verifiziert, gleiche Safety-Regeln) — das
entrootet die Bootchain komplett. Ein **voller** Stock-Return
(`system`/`vendor`-Reflash) ist außerhalb Tool-Scope; via HiSuite, eRecovery
oder Fastboot mit Full-Firmware-Paket (siehe [INSTRUCTIONS.md](INSTRUCTIONS.md),
Downloader + `data/firmware/`).

## Touchscreen-Ränder tot (APTouch-Issue) — wie fixen?

Bekannter Kirin-960/GSI-Quirk (siehe [P10-Wiki](https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus)).
Braucht erst Root (Verify zeigt `ROOTED`, `uid=0`), dann als Root:

```
adb shell su -c "stop aptouch"
```

Persistente Optionen: Magisk-Boot-Skript, oder das Community-Fix-Modul
([GSI_Generic_Fix für Huawei P10](https://github.com/Coconutat/GSI_Generic_Fix_Magisk_Module_For_Huawei_P10),
fixt auch Speaker). Speaker-only als Root:

```
adb shell su -c "chown root:audio /dev/nxp_smartpa_dev; chmod 0660 /dev/nxp_smartpa_dev"
```

Hinweis: Das Tool versucht den permanenten APTouch-Fix automatisch nach jedem
verifizierten Root (sofort-`stop` + `service.d`-Skript).

## Ist ein unlockter Bootloader Pflicht?

Ja. Das Tool unlockt niemals selbst (kein `flashing unlock`, kein
`oem unlock`, nie). Wenn ein TrebleDroid/Lineage-GSI schon auf dem P10
bootet, ist Unlock erledigt (PotatoNV:
[mashed-potatoes/PotatoNV](https://github.com/mashed-potatoes/PotatoNV)).
Frische Stock-Geräte erst unlocken — das Tool erkennt den State, stoppt aber
vor allem Destruktiven.

## `FAILED (remote: Command not allowed)` — ist mein Bootloader locked?

Daraus allein keine Aussage möglich — Huawei-Fastboot verweigert mehrere
Standard-`getvar`-Abfragen auch auf unlockten Geräten. Das Tool wertet es als
Huawei-Quirk und arbeitet mit `/dev/block/by-name`, `ro.boot.*` und
`/proc/cmdline` weiter, statt einen Lock zu raten.

## Welche Magisk-Version soll ich nehmen?

Nicht blind "latest". Orientierung (immer [offizielles
Changelog](https://github.com/topjohnwu/Magisk/releases) gegenprüfen):

| Android | Getestet |
|---|---|
| 8.x (EMUI 8) | Magisk v25+ |
| 9.x (EMUI 9/9.1) | Magisk v25+ |
| 10–12 (GSI/Custom) | Magisk v26+ |
| 13–14 (TrebleDroid/Lineage-GSI) | Magisk v27/v28+ per Changelog |

Nur offizielle GitHub-APKs; Version + SHA-256 dokumentiert das Tool.

## Brauche ich Vol-Up + Power bei jedem Boot?

Nur für Boots **mit** Root. Normaler Power-On bootet Stock (ungerootet) — das
ist dokumentiertes Magisk-Recovery-Verhalten auf diesem Gerät, kein Bug.
Kein persistenter Schalter existiert; die eRecovery-Byte-Tricks aus #2542
stehen im Tool unter Boot-Tricks.

## Kann Root normale Reboots überleben (No-Ramdisk wie P10)?

Teils — ehrlich zweigeteilt:

- **Gerooteter Boot selbst**: kein sicherer Bypass existiert. Magisk lebt in
  `recovery_ramdisk`, normaler Power-On bootet also immer ungerootet. Das Tool
  dokumentiert die Community-**Persistent-Boot**-Option (Discussion-Step 13:
  persistentes Recovery-Boot-Byte via eRecovery-Wipe-Trick, via Step 14 wieder
  gelöscht) unter Boot-Tricks → persistenter Boot, mit Doppel-Bestätigung,
  Reversibilität und Verify-Step (normal rebooten → `uid=0`-Check). Ändert
  *jeden* Boot by design — bewusst entscheiden.
- **Was das Tool persistiert**: APTouch- + Speaker-Fixes als Magisk-
  `service.d`-Bootskripte (Menü Persist, braucht einmal live Root — kein
  manuelles adb mehr nach Reboot), plus verifizierter Root-/Bootmodus-State
  über Runs.
- **APTouch nach jedem Root**: ja, automatisch. Jeder verifizierte Root
  (Verify-Screen, Wizard-Root-Wege) versucht den permanenten APTouch-Fix:
  sofort-`stop aptouch` + `service.d`-Skript für jeden gerooteten Boot.
  Voll zerstoerungsfrei (ein Service gestoppt, Skripte entfernbar),
  ohne live Root sauber geskipt, fragt nächstes Mal erneut.

## Kein Ton nach Viper4Android / Magisk-Modulen?

Audio-Mods können das ganze GSI stumm schalten, auch nach App-Deinstall
(Modul bleibt aktiv). Gerootet booten,
`adb shell su -c "magisk --remove-modules"`, rebooten, Module einzeln neu
installieren. Debug-Kette + Prävention: Wiki Troubleshooting → No sound.
Letzter Ausweg: Tool-Restore.

## Verliere ich Daten?

Nur wenn **du** es wählst: TUI `Full reinstall` bietet optionalen geführten
Wipe vor Flash, CLI/TUI `wipe` löscht userdata nur nach Doppel-Bestätigung
(`WIPE` + `YES`/`JA`, oder `--yes`) — mit eRecovery-Fallback wenn das Gerät
`erase` ablehnt. Root-/Flash-Wege selbst wipen nie. Trotzdem: Daten sichern
bevor du die Bootchain anfasst, und nur aus Stock-eRecovery factory-resetten
wenn **du** es wählst (nie aus TWRP — zerstört userdata per Wiki).

## Ich habe ein Custom-ROM installiert. Welches Image patche ich mit Magisk?

Siehe oben: „Brauche ich das originale RECOVERY …" — GSI → Stock-Basis
(korrekt), volles Device-ROM → dessen Paket (niemals Stock).

## Was wenn Android nach Flash nicht mehr bootet?

Ruhig bleiben, in Fastboot bleiben: TUI **Restore / Unroot** (oder CLI
`restore`) schreibt das hash-verifizierte originale `recovery_ramdisk`
zurück, dann `fastboot reboot`. Das ist der Safe Restore Mode — eine
Partition, sonst nichts.

## Mein GSI bleibt wirklich unangetastet?

Auf dem **Root-Weg**: ja. Einzige Partition, die das Tool dort schreibt, ist
`recovery_ramdisk`, aus Device-Profil abgeleitet und per 9-Punkt-Safety-Gate
rebestätigt (`DO NOT FLASH` bei jedem FAIL). Der **ROM-Install-Weg**
(`flash-system`) ersetzt `system` nur nach eigenen Checks + Doppel-
Bestätigung — `userdata` wird nur je per explizitem geführtem `wipe`
(doppelt bestätigt) oder optionalem Wipe in `Full reinstall` gelöscht.

## Wo sind Logs? Was schicke ich für Hilfe?

Jeder Lauf schreibt `logs/toolkit-<stamp>.log`. Für Hilfe: `diagnostic`
(CLI) oder Tools → Diagnose-ZIP (TUI) — 10 Dateien, `--anonymize` nutzen,
Serials schwärzen. Siehe [SUPPORT.md](SUPPORT.md).

## Geht es offline?

Ja, nach einmaligem Ablegen: Full-Firmware-ZIP → `data/firmware/`, Magisk-APK
→ `data/magisk/`, ROM-Pakete → `data/roms/`, Extraktoren → `data/tools/`.
Internet nur noch für explizite Downloads mit deiner Bestätigung.

## Brauche ich scrcpy?

Nein — optionales Screen-Mirroring beim Rooten. Tool erkennt es beim Start
(`detect` zeigt Pfad) und bietet One-Click-Mirror aus Tools. Fehlen ist nur
INFO-Zeile. Quelle: [Genymobile/scrcpy](https://github.com/Genymobile/scrcpy).

## Welche Geräte sind supported?

Huawei P10 VTR-L29 (primär), VTR-L09, P10 Plus VKY-L29 — arm64, EMUI-9.1-Basis.
Andere Kirin-Geräte nur via neue Profile mit echten Gerätedaten
(siehe `device-support`-Issue-Template) — nie angenommen.
