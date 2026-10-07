# Troubleshooting — trebleManager

## Download (stock firmware)

- `URL rejected`: only `http(s)` + archive type (`zip/7z/tar/gz/app/rar`). `ftp/file/exe` are blocked.
- BITS stuck: the tool automatically falls back to WebClient with progress bar. Check free space (2–4 GB), target `data/firmware/` (cache, no re-download).
- `WARN < 100 MB / no UPDATE.APP in ZIP`: wrong package (delta/region). Pick another full firmware with matching CUST, then TUI step 4.
- Honesty note: in 2026 there is no guaranteed official direct link. Use HiSuite (official) for re-install or pick a community-archive link yourself + verify hash. Without `--yes` the CLI downloads nothing.

## ADB / fastboot

- `ADB not found`: put `p10-magisk-check-FIXED.bat` / PS1 next to `adb.exe` or set PATH (`C:\Program Files (x86)\Minimal ADB and Fastboot`).
- `No device detected`: change cable/port, check drivers (HiSuite/Kirin), `adb kill-server/start-server` (TUI tools).
- `ADB unauthorized`: confirm the on-device dialog ("always allow"), replug cable.
- `Fastboot not found / waiting for device`: boot to fastboot manually (Power+Vol-Down or `adb reboot bootloader`), wait the 30s window.
- `FAILED (remote: Command not allowed)`: normal on Huawei. Not proof of lock. Continue with by-name + `ro.boot.*` + `/proc/cmdline`.
- Old Minimal fastboot `unknown command 2>&1`: wrongly escaped redirection. Use FIXED BAT / PS1 (correct `>> log 2>&1`, never as fastboot arg).

## Partitions / firmware

- `Unsupported partition layout` (no `recovery_ramdisk`): run TUI step 2 + tools `getvar` + by-name. Never guess, no `boot`/`recovery` substitute. Create a diagnostic ZIP.
- `Unsupported model`: only VTR-L29/L09, VKY-L29 are profiled. Other Kirin devices: add a profile in `$DeviceProfiles`, never flash with a foreign profile.
- `Firmware mismatch` (FAIL): model family (VTR vs VKY) is strict; check submodel/region (L29 vs L09, C432 vs C185) and EMUI (9.1 vs 9.0/8). Pick another full firmware with matching CUST.
- `UPDATE.APP` unclear: place file in `data/firmware/`, CLI `extract` / TUI step 4. Extractors go in `data/tools/` (huawei-update-extractor/splitupdate) or extract manually. Keep the exact name (`RECOVERY_RAMDIS.img` vs `RECOVERY_RAMDISK.img`).
- `Cannot safely flash image`: check size/hash/header (`extract` shows details). Never force foreign/renamed images.

## Magisk / root

- `Patch failed`: only real on-device patches (`Magisk → Select and Patch a File` with exactly `RECOVERY_RAMDIS(K).img`). Hash must differ from stock. Pull via `adb pull /sdcard/Download/`.
- `INCONCLUSIVE` (su present, no uid=0): open the Magisk app, grant root, repeat reboot with boot cheat, wait out first boot.
- `NOT_ROOTED` after flash: boot cheat forgotten (`Vol-Up + Power until logo`)? Booted stock instead of Magisk recovery. Repeat. Check Magisk package (`pm list packages`).
- `Boot verification failed`: stay in fastboot, TUI Restore / CLI `restore`, original hash is verified, then `fastboot flash recovery_ramdisk original.img`, `fastboot reboot`.

## Recovery export (custom ROMs)

- `No boot.img/recovery.img/payload.bin in ZIP`: probably a GSI system package — it has no recovery by design. Use the stock UPDATE.APP path.
- `payload.bin` without output: place `payload-dumper-go` in `data/tools/` or extract `boot.img` manually, then re-run `export`.
- Exported images are patch candidates, not flash-ready: validate target partition from the device profile, patch in Magisk, pass the safety gate. Rights are granted in the Magisk app (verified via `uid=0`).

## GSI preservation

- The tool never touches `system`/`vendor`/`userdata`. On accidental wipe use stock recovery only (per wiki, TWRP factory reset breaks userdata).

## Logs / diagnostics

- Every run writes `logs/toolkit-<stamp>.log`. TUI log preview (200 lines) + open folder.
- Full diagnosis: CLI `diagnostic [--anonymize] [--json]` or TUI tools. ZIP in `logs/` (10 files). Redact/anonymize serials before upload.

---

<a id="de"></a>
## Deutsch — Troubleshooting

## Download (Stock-Firmware)

- `URL rejected`: nur `http(s)` + Archivtyp (`zip/7z/tar/gz/app/rar`).
  `ftp/file/exe` sind blockiert.
- BITS hängt: Tool fällt automatisch auf WebClient mit Progress zurück.
  Freien Platz prüfen (2–4 GB), Ziel `data/firmware/` (Cache, kein Re-Download).
- `WARN < 100 MB / no UPDATE.APP in ZIP`: falsches Paket (Delta/Region).
  Andere Full-Firmware mit passendem CUST wählen, dann TUI-Step 4.
- Ehrlichkeitshinweis: 2026 gibt es keinen garantierten offiziellen
  Direktlink. HiSuite (offiziell) für Reinstall nutzen oder
  Community-Archiv-Link selbst wählen + Hash verifizieren. Ohne `--yes`
  lädt die CLI nichts.

## ADB / Fastboot

- `ADB not found`: `p10-magisk-check-FIXED.bat` / PS1 neben `adb.exe` legen
  oder PATH setzen (`C:\Program Files (x86)\Minimal ADB and Fastboot`).
- `No device detected`: Kabel/Port wechseln, Treiber prüfen (HiSuite/Kirin),
  `adb kill-server/start-server` (TUI-Tools).
- `ADB unauthorized`: Dialog am Gerät bestätigen ("immer erlauben"), Kabel neu.
- `Fastboot not found / waiting for device`: manuell nach Fastboot booten
  (Power+Vol-Down oder `adb reboot bootloader`), 30s-Fenster abwarten.
- `FAILED (remote: Command not allowed)`: normal bei Huawei. Kein Lock-Beweis.
  Weiter mit by-name + `ro.boot.*` + `/proc/cmdline`.
- Altes Minimal-Fastboot `unknown command 2>&1`: falsch escaptes Redirect.
  FIXED-BAT / PS1 nutzen (korrekt `>> log 2>&1`, nie als Fastboot-Arg).

## Partitionen / Firmware

- `Unsupported partition layout` (kein `recovery_ramdisk`): TUI-Step 2 +
  Tools-`getvar` + by-name laufen lassen. Nie raten, kein `boot`/`recovery`-
  Ersatz. Diagnose-ZIP erzeugen.
- `Unsupported model`: nur VTR-L29/L09, VKY-L29 profiliert. Andere
  Kirin-Geräte: Profil in `$DeviceProfiles` ergänzen, nie mit fremdem Profil
  flashen.
- `Firmware mismatch` (FAIL): Modellfamilie (VTR vs VKY) ist strikt;
  Submodell/Region (L29 vs L09, C432 vs C185) und EMUI (9.1 vs 9.0/8) prüfen.
  Andere Full-Firmware mit passendem CUST wählen.
- `UPDATE.APP` unklar: Datei nach `data/firmware/`, CLI `extract` / TUI-Step 4.
  Extraktoren nach `data/tools/` (huawei-update-extractor/splitupdate) oder
  manuell extrahieren. Exakten Namen behalten (`RECOVERY_RAMDIS.img` vs
  `RECOVERY_RAMDISK.img`).
- `Cannot safely flash image`: Size/Hash/Header prüfen (`extract` zeigt
  Details). Nie fremde/umbenannte Images forcen.

## Magisk / Root

- `Patch failed`: nur echte On-Device-Patches (`Magisk → Select and Patch a
  File` mit exakt `RECOVERY_RAMDIS(K).img`). Hash muss sich von Stock
  unterscheiden. Via `adb pull /sdcard/Download/` holen.
- `INCONCLUSIVE` (su da, kein uid=0): Magisk-App öffnen, Root freigeben,
  Reboot mit Boot-Cheat wiederholen, ersten Boot abwarten.
- `NOT_ROOTED` nach Flash: Boot-Cheat vergessen (`Vol-Up + Power bis Logo`)?
  Stock statt Magisk-Recovery gebootet. Wiederholen. Magisk-Paket prüfen
  (`pm list packages`).
- `Boot verification failed`: in Fastboot bleiben, TUI Restore / CLI
  `restore`, Original-Hash wird verifiziert, dann
  `fastboot flash recovery_ramdisk original.img`, `fastboot reboot`.

## Recovery-Export (Custom-ROMs)

- `No boot.img/recovery.img/payload.bin in ZIP`: wahrscheinlich GSI-
  System-Paket — hat by design kein Recovery. Stock-UPDATE.APP-Weg nutzen.
- `payload.bin` ohne Output: `payload-dumper-go` nach `data/tools/` legen
  oder `boot.img` manuell extrahieren, dann `export` wiederholen.
- Exportierte Images sind Patch-Kandidaten, nicht flash-fertig: Zielpartition
  aus Device-Profil validieren, in Magisk patchen, Safety-Gate passieren.
  Rechte werden in Magisk-App vergeben (via `uid=0` verifiziert).

## GSI-Erhalt

- Das Tool fasst `system`/`vendor`/`userdata` nie an. Bei versehentlichem Wipe
  nur Stock-Recovery nutzen (per Wiki: TWRP-Factory-Reset zerstört userdata).

## Logs / Diagnose

- Jeder Lauf schreibt `logs/toolkit-<stamp>.log`. TUI-Log-Vorschau (200
  Zeilen) + Ordner öffnen.
- Voll-Diagnose: CLI `diagnostic [--anonymize] [--json]` oder TUI-Tools. ZIP
  in `logs/` (10 Dateien). Serials vor Upload schwärzen/anonymisieren.
