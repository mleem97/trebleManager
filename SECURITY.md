# SECURITY

## Grundsatz

Sicherheit und technische Korrektheit vor Automatisierung. Wenn Image/Partition/Geraet
nicht zweifelsfrei passen: **NICHT FLASHEN** (`DO NOT FLASH`), Diagnose zeigen, User-Input fordern.

## Safety-Gate (alle muessen PASS sein)

1. Model matches (VTR/VKY-Familie)
2. Partition exists (`recovery_ramdisk` in by-name oder fastboot size/type, FAILED != existiert)
3. Image exists (patched)
4. Image hash known (SHA-256)
5. Image size plausible (1-100 MB)
6. Firmware compatibility established (Status != FAIL, Baseline gesetzt)
7. Backup available (`original.img` vorhanden)
8. Fastboot device connected (live geprueft)
9. Patched != stock (kein Fake-Patch)

## Bestaetigungs-Matrix

- Automatisch erlaubt: Detect, getprop/getvar, Partitions-/Firmware-/UPDATE.APP-Analyse, Hashing, Backup-Anlage, Logs, Diagnose-ZIP.
- Bestaetigung erforderlich (TUI doppelt, CLI `--yes`): Patch-Staging ist harmlos, aber Flash/Restore brauchen `FLASHEN`+`JA` / `RESTORE`+`JA`.
- Niemals automatisch (auch nicht per Flag): `fastboot erase/format userdata`, `flashing unlock`, `oem unlock`, Bootloader-Unlock, `system`/`vendor`/`userdata`-Flash. Im Code kommen diese Befehle ausser in Doku-Strings nicht vor.

## Integritaet

- Jedes relevante Image: SHA-256 + SHA-512 (`Get-FileHashInfo`), Header (ANDROID!/gzip) dokumentiert, Gr night 1-100 MB.
- Backup: `metadata.json` (device/model/firmware/build/partition/size/sha256/sha512/timestamp/source/tool), `sha256.txt`, `sha512.txt`.
- Magisk nur aus offizieller Quelle (https://github.com/topjohnwu/Magisk/releases), Version + SHA-256 dokumentiert. Kein Auto-Download aus dubiosen Quellen; Firmware-Quellen nur als UI-Hinweise (Firm Finder, Discussion, Wiki).
- Keine Mock-Daten im Prod-Workflow: `fake success/mock device/fake patch/fake root` verboten. Unmoegliches -> echter Fehler + Exit-Code (flash=1, verify=2, extract/backup=3).

## Datenschutz / Netzwerk

- Offline-first: `data/firmware`, `data/magisk`, `data/tools`, `backups` werden wiederverwendet.
- Netzwerk nur fuer: GitHub-Run (User initiiert), Firmware-/Magisk-Download (User initiiert), Updatepruefung. Keine Telemetrie.
- Diagnose-ZIP: `--anonymize` ersetzt Seriennummer durch `***ANONYMIZED***`. `getvar all` kann geraetespezifische Werte enthalten -> vor Weitergabe schwaerzen.

## Huawei-Spezifika

- `Command not allowed` != locked. Bootloader-Status bleibt `Unknown`, solange nicht anders belegbar.
- Bootkeys exakt: `Vol-Up + Power bis Logo` (nicht persistent). `/dload` darf nicht vorhanden sein.
- GSI-Erhalt: nur `recovery_ramdisk` wird geschrieben (aus Profil abgeleitet).
