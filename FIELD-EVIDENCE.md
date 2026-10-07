# Field evidence — P10 root via stock recovery_ramdisk + Magisk (Dry Run Tests + Fremd-Berichte)

<a id="de"></a>
> (Deutscher Teil zuerst, [English below](#en).)

> Status mariking: everything **we** measured is **Dry Run Tests** (no
> device flashed). Fremd-Berichte sind dokumentierte Geräte-Ergebnisse
> anderer Nutzer. "100% working" heißt dort: System bootet + Root per
> Boot-Cheat verifiziert, **inkl. bekannter Quirks** (siehe unten).

## 1. Was wir selbst verifiziert haben (Dry Run Tests, hier ausgeführt)

- GSI-Datei `lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz`
  (1.245.119.669 Bytes, SHA-256 `48c7c379…97de787`): SourceForge-Listing +
  302-Redirect geprüft, Download vollständig, Hash notiert
  (siehe `gsi-root/docs/VERIFICATION-lineage20-20251021.md`).
- Dekomprimiert: raw ext4, 2,8 GB, Android 13 (SDK 33),
  `ro.lineage.version=20.0-20251021-UNOFFICIAL-arm64_bgN` (passt zu Dateiname
  + Registry), SAR-Layout (`/init`-Symlink), `init.environ.rc` mit
  `on early-init` vorhanden, kein AVB-Footer.
- Tool-Verhalten auf genau dieser Datei: `export` verweigert sie als
  SYSTEM-Image mit Stock-Recovery-Ansage; `analyze` meldet Engine
  EXPERIMENTAL (kein Fake-Erfolg). Exakt das erwartete Verhalten.
- Suiten: Bash 114 PASS, PS1 63 PASS, Cargo 19+15 PASS (alle Dry Run Tests).

## 2. Was andere Nutzer auf Geräten berichtet haben (Abgleich)

| Quelle | Gerät + System | Methode | Ergebnis |
|---|---|---|---|
| michal25, [Discussion #2542](https://github.com/phhusson/treble_experimentations/discussions/2542) | VTR-L29, Lineage 18.1/19.1/**20.x** | Stock-`recovery_ramdisk` aus Full-Firmware (9.1.0.297 / 9.1.0.275) → Magisk → `fastboot flash recovery_ramdisk` | **"full working"**, Root nur per Vol-Up+Power-Cheat; permanenter Boot per eRecovery-Trick (Steps 13/14) dokumentiert |
| XDA-Thread (Apr 2023, derselbe Guide) | VTR-L29 | identisch | **"full working"**, gleicher Cheat-Vorbehalt; Kommentare: KernelSU-Alternative auf vtr |
| TrebleDroid-Wiki (P10-Seite) | VKY-L29: LineageOS; VTR-L29: AOSP 10/11/12, PixelExperience A13, SuperiorOS ("working well"); Light=−bootloop, HavocOS=VTR-L09 defekt | GSI + Fixes | WiFi/Mobile Daten ok, Fingerprint ok (außer A12 v400.e), **aptouch-Ränder fix = `stop aptouch`** (unser Auto-Fix), SIM-Reinject ablehnen, USSD defekt |
| 0nsec/Huawei-p10-custom-rom | VTR-L29, PixelExperience 12.1 | UPDATE.APP-Extraktion, Partition-Transplant, Stock-Recovery-Wipe-Trick, Magisk, SIM/WiFi/Touch/Speaker-Fixes | Komplett-Guide, gleiche Kette wie unser Tool |
| mohammadhosin/p10_root | VTR-L29 | `fastboot flash recovery_ramdisk Magisk.img` | Root-Bestätigung (grob, ohne GSI) |

## 3. Ehrliche Lücken (muss der Remote-User wissen)

1. **Exakter Build 20251021**: in keinem Fremd-Bericht namentlich getestet
   (neuer als die Guides). Gleiche Linie (Lineage 20 bgN), gleiche Methode —
   aber Build-Quirks unbekannt. Genau dafür ist unser `uid=0`-Gate da.
2. **"100% working" schließt Quirks ein**: SIM-Reinject-Abfrage ablehnen,
   USSD geht nicht, APTouch-Ränder brauchen den Fix (Tool macht ihn
   automatisch), **jeder Root-Boot braucht Vol-Up+Power** (außer
   Persistent-Byte-Trick, bewusste Entscheidung).
3. **Magisk-Version**: Berichte stammen aus der v24-Ära; aktuelle Stable
   (30.x) muss am Gerät per `uid=0`-Check bestehen — das Tool fälscht hier
   nichts (Boot allein ≠ Root).
4. **Nicht verwechseln**: Lineage 20 **Light** = Bootloop, HavocOS auf
   VTR-L09 = defekt (Registry blockt beide hart).

## 4. Handoff-Checkliste für das Remote-Gerät

1. Release-ZIP + `.sha256` prüfen, `Start-TrebleToolkit.bat` starten.
2. Wizard → System wählen (LineageOS 20 UNOFFICIAL steht in der Liste) →
   Ziel „Nur Root".
3. `diagnostic --anonymize` erzeugen und zurückschicken **vor** dem Flash.
4. Erst bei `FLASH RESULT: OK` + `uid=0` gilt Root als verifiziert.
5. APTouch-Ränder + SIM-Verhalten am Gerät gegenprüfen, Ergebnis melden.

---

<a id="en"></a>
## English — field evidence

> Marking: everything **we** measured is **Dry Run Tests** (no device
> flashed). External reports are documented on-device results from other
> users. "100% working" there means: system boots + root verified via boot
> cheat, **including known quirks** (see below).

## 1. What we verified ourselves (Dry Run Tests, executed here)

- GSI file `lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz`
  (1,245,119,669 bytes, SHA-256 `48c7c379…97de787`): SourceForge listing +
  302 redirect checked, download complete, hash recorded
  (see `gsi-root/docs/VERIFICATION-lineage20-20251021.md`).
- Decompressed: raw ext4, 2.8 GB, Android 13 (SDK 33),
  `ro.lineage.version=20.0-20251021-UNOFFICIAL-arm64_bgN` (matches filename
  + registry), SAR layout (`/init` symlink), `init.environ.rc` with
  `on early-init` present, no AVB footer.
- Tool behavior on exactly this file: `export` refuses it as a SYSTEM image
  with the stock-recovery guidance; `analyze` reports the engine as
  EXPERIMENTAL (no fake success). Exactly the expected behavior.
- Suites: Bash 114 PASS, PS1 63 PASS, Cargo 19+15 PASS (all Dry Run Tests).

## 2. What other users reported on devices (cross-check)

| Source | Device + system | Method | Result |
|---|---|---|---|
| michal25, [Discussion #2542](https://github.com/phhusson/treble_experimentations/discussions/2542) | VTR-L29, Lineage 18.1/19.1/**20.x** | Stock `recovery_ramdisk` from full firmware (9.1.0.297 / 9.1.0.275) → Magisk → `fastboot flash recovery_ramdisk` | **"full working"**, root only via Vol-Up+Power cheat; permanent boot via eRecovery trick (steps 13/14) documented |
| XDA thread (Apr 2023, same guide) | VTR-L29 | identical | **"full working"**, same cheat caveat; comments: KernelSU alternative on vtr |
| TrebleDroid wiki (P10 page) | VKY-L29: LineageOS; VTR-L29: AOSP 10/11/12, PixelExperience A13, SuperiorOS ("working well"); Light=bootloop, HavocOS=VTR-L09 broken | GSI + fixes | WiFi/mobile data fine, fingerprint fine (except A12 v400.e), **aptouch edges fix = `stop aptouch`** (our auto-fix), reject SIM reinsert, USSD broken |
| 0nsec/Huawei-p10-custom-rom | VTR-L29, PixelExperience 12.1 | UPDATE.APP extraction, partition transplant, stock-recovery wipe trick, Magisk, SIM/WiFi/touch/speaker fixes | Complete guide, same chain as our tool |
| mohammadhosin/p10_root | VTR-L29 | `fastboot flash recovery_ramdisk Magisk.img` | Root confirmation (crude, no GSI) |

## 3. Honest gaps (the remote user must know)

1. **Exact build 20251021**: not tested by name in any external report
   (newer than the guides). Same line (Lineage 20 bgN), same method —
   but build quirks unknown. Exactly what our `uid=0` gate is for.
2. **"100% working" includes quirks**: reject SIM-reinsert prompt, no USSD,
   APTouch edges need the fix (tool does it automatically), **every rooted
   boot needs Vol-Up+Power** (except persistent-byte trick, conscious choice).
3. **Magisk version**: reports are from the v24 era; current stable (30.x)
   must pass the on-device `uid=0` check — the tool fakes nothing here
   (boot alone ≠ root).
4. **Do not confuse**: Lineage 20 **Light** = bootloop, HavocOS on
   VTR-L09 = broken (registry hard-blocks both).

## 4. Handoff checklist for the remote device

1. Verify release ZIP + `.sha256`, start `Start-TrebleToolkit.bat`.
2. Wizard → pick system (LineageOS 20 UNOFFICIAL is listed) → goal "Root only".
3. Create `diagnostic --anonymize` and send it back **before** flashing.
4. Root counts as verified only at `FLASH RESULT: OK` + `uid=0`.
5. Cross-check APTouch edges + SIM behavior on device, report back.
