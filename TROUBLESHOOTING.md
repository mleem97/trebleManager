# TROUBLESHOOTING

## Download (Stock-Firmware)

- `URL abgelehnt`: nur `http(s)` + Archivtyp (`zip/7z/tar/gz/app/rar`). `ftp/file/exe` werden blockiert.
- BITS hängt: Tool fällt automatisch auf WebClient mit Balken zurück. Freien Platz prüfen (2-4 GB), Ziel `data/firmware/` (Cache, kein Re-Download).
- `WARN < 100 MB / keine UPDATE.APP im ZIP`: falsches Paket (Delta/Region). Andere Full-Firmware mit passender CUST wählen, danach TUI Step 4.
- Ehrlichkeit: 2026 gibt es keinen garantierten offiziellen Direktlink. HiSuite (offiziell) für Re-Install nutzen oder Community-Archiv-Link selbst wählen + Hash prüfen. Ohne `--yes` lädt CLI nichts.

## ADB/Fastboot

- `ADB not found`: `p10-magisk-check-FIXED.bat` / PS1 neben `adb.exe` legen oder PATH setzen (`C:\Program Files (x86)\Minimal ADB and Fastboot`).
- `No device detected`: Kabel/Port wechseln, Treiber (HiSuite/Kirin) pruefen, `adb kill-server/start-server` (TUI Tools).
- `ADB unauthorized`: Dialog am Geraet bestaetigen ("immer zulassen"), Kabel neu stecken.
- `Fastboot not found / waiting for device`: manuell in Fastboot (Power+Vol-Down oder `adb reboot bootloader`), 30s-Wait abwarten.
- `FAILED (remote: Command not allowed)`: Huawei-normal. Nicht als locked werten. Weiter mit by-name + `ro.boot.*` + `/proc/cmdline`.
- Altes Minimal-Fastboot `unknown command 2>&1`: Umleitung falsch escaped. FIXED-BAT / PS1 nutzen (dort korrekt `>> log 2>&1`, nie als Fastboot-Arg).

## Partitionen / Firmware

- `Unsupported partition layout` (kein `recovery_ramdisk`): TUI Step 2 + Tools `getvar` + by-name liefern. Nicht raten, kein `boot`/`recovery`-Ersatz. Diagnose-ZIP erzeugen.
- `Unsupported model`: Nur VTR-L29/L09, VKY-L29 profiliert. Andere Kirin-Geraete: Profil in `$DeviceProfiles` ergaenzen, nie mit fremdem Profil flashen.
- `Firmware mismatch` (FAIL): Modell-Familie (VTR vs VKY) strikt, Submodell/Region (L29 vs L09, C432 vs C185) und EMUI (9.1 vs 9.0/8) pruefen. Andere Full-Firmware mit passender CUST waehlen.
- `UPDATE.APP` unklar: Datei nach `data/firmware/` legen, CLI `extract` / TUI Step 4. Extractor in `data/tools/` (huawei-update-extractor/splitupdate) oder manuell. Exakten Namen (`RECOVERY_RAMDIS.img` vs `RECOVERY_RAMDISK.img`) behalten.
- `Cannot safely flash image`: Groesse/Hash/Header pruefen (`extract` zeigt Details). Nie umbenannte/fremde Images erzwingen.

## Magisk / Root

- `Patch failed`: Nur echte on-device-Patches (`Magisk -> Select and Patch a File` mit exakt `RECOVERY_RAMDIS(K).img`). Hash muss sich von Stock unterscheiden. `magisk_patched-*.img` via `adb pull /sdcard/Download/` holen.
- `INCONCLUSIVE` (su da, kein uid=0): Magisk-App oeffnen, Root-Freigabe erteilen, Reboot mit Boot-Cheat wiederholen, Erstboot abwarten.
- `NOT_ROOTED` nach Flash: Boot-Cheat (`Vol-Up + Power bis Logo`) vergessen? Stock gebootet statt Magisk-Recovery. Wiederholen. Magisk-Paket (`pm list packages`) pruefen.
- `Boot verification failed`: In Fastboot bleiben, TUI Restore / CLI `restore`, Original-Hash wird geprueft, dann `fastboot flash recovery_ramdisk original.img`, `fastboot reboot`.

## GSI-Erhalt

- Tool fasst `system`/`vendor`/`userdata` nie an. Bei versehentlichem Wipe nur via Stock-Recovery (nicht TWRP Factory-Reset laut Wiki, bricht userdata).

## Logs / Diagnose

- Jeder Run schreibt `logs/toolkit-<stamp>.log`. TUI Logs-Vorschau (200 Zeilen) + Ordner oeffnen.
- Volldiagnose: CLI `diagnostic [--anonymize] [--json]` oder TUI Tools. ZIP in `logs/` (10 Dateien). Seriennummer vor Upload schwaerzen/anonymisieren.
