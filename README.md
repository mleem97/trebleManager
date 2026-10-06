# Huawei P10 Root Manager (TrebleToolkit v2.0.0)

PowerShell-TUI + CLI fuer Huawei P10 (primaer VTR-L29, modular erweiterbar auf VTR-L09/VKY-L29),
OS-unabhaengig: Stock-EMUI 8/9/9.1, TrebleDroid/Lineage-GSI, Pixel/Superior/AOSP-GSI, Custom-ROMs, Android 8-14.

Technische Basis: [Discussion #2542](https://github.com/phhusson/treble_experimentations/discussions/2542),
[Wiki Huawei P10](https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus),
Magisk: <https://github.com/topjohnwu/Magisk>, Unlock: <https://github.com/mashed-potatoes/PotatoNV>.

## Design-Entscheidung (verifiziert)

- Zielpartition P10 = `recovery_ramdisk` (NICHT `boot`, NICHT `recovery`). Aus Profil abgeleitet, nicht hart geraten.
- Quelle = `RECOVERY_RAMDIS(K).img` aus `UPDATE.APP` der passenden EMUI-9.1-Full-Firmware.
  Beispiel (NICHT exklusiv): `VTR-L29 9.1.0.297(C432E5R1P9)`. Region/CUST muss zum Geraet passen.
- Magisk-Weg: Stock-Image -> Magisk-App "Select and Patch a File" (on-device) -> `fastboot flash recovery_ramdisk magisk_patched.img`.
- Boot: `Vol-Up + Power bis Huawei-Logo` (Magisk boot cheat, nicht persistent). Ohne Trick kein Root.
- Huawei-Fastboot `FAILED (remote: Command not allowed)` ist KEIN Lock-Beweis.
- GSI (`system`/`vendor`) bleibt erhalten. Es wird nur `recovery_ramdisk` angefasst.

## Schnellstart (lokal)

```
TrebleToolkit/
  Start-TrebleToolkit.bat   # Doppelklick (fragt bei Bedarf nach Admin)
  scripts/Treble-Toolkit.ps1
  logs/ data/firmware/ data/magisk/ backups/
```

## Direkt von GitHub ausfuehren (100% TUI, kein Download)

Direkt von GitHub ausfuehren (Repo: mleem97/trebleManager, Branch main):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $f=\"$env:TEMP\Treble-Toolkit.ps1\"; Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1' -OutFile $f -UseBasicParsing; & $f"
```

Oder `Run-FromGitHub.bat` (oben `RAW_URL` eintragen) per Doppelklick.
CLI remote: `& $f detect --json`, `& $f diagnostic --anonymize`, etc.

## Wizard (TUI) / CLI

TUI ohne Args. CLI:

```
Treble-Toolkit.ps1 detect|analyze|firmware|download|extract|patch|backup|flash|verify|restore|diagnostic|wizard|help [--json] [--yes] [--image <pfad>] [--firmware-file <url|pfad>] [--anonymize] [--no-reboot]
```

- `firmware`: Baseline + Kompatibilitaet.
- `download --firmware-file <https-URL> [--yes] [--json]`: **Stock-Firmware (original)** mit Fortschritt.
  Ohne `--yes` nur Kandidat, kein Traffic. Mit `--yes` BITS (Resume) oder WebClient mit Balken
  (MB/Prozent), danach Groesse/SHA-256 + UPDATE.APP-Check + `.sha256`-Sidecar.
  Quellen in TUI Step 3 (HiSuite offiziell, Consumer-Suche, Firm Finder historisch,
  androidhost.ru Community-Archiv, Wiki/Doku). Kein Dubios-Auto-Download. Ablage `data/firmware/` (Cache).
- `extract`: UPDATE.APP analysieren + RECOVERY-Images validieren (read-only).
- `patch`: Stock validieren + `data/magisk/to-patch/` staging + Anleitung (kein Fake-Patch).
- `backup`: `backups/<MODEL>/recovery_ramdisk/<stamp>/` mit `original.img`, `metadata.json`, `sha256.txt`, `sha512.txt`.
- `flash`/`restore`: nur nach Safety-Gate + Doppel-Bestaetigung (`FLASHEN`+`JA` / `RESTORE`+`JA`, CLI: `--yes`).
- `diagnostic`: `logs/Huawei-P10-Diagnostic-<stamp>.zip` mit `device.json, adb.txt, fastboot.txt, partitions.txt, properties.txt, boot.txt, firmware.txt, hashes.txt, tool-version.txt, log.txt`.

## Voraussetzungen / Risiken / Restore

- Windows PowerShell 5.1 oder PowerShell 7, ADB/Fastboot (Minimal ADB oder platform-tools), USB-Treiber, USB-Debugging.
- Risiken: Bootchain-Eingriff, Bootloop bei falschem Image/Partition. Deshalb Safety-Gate (8 Checks) + Backup-Pflicht + `DO NOT FLASH`.
- Restore: TUI "Restore / Unroot" oder CLI `restore` (Hash + Partition geprueft, dann Original zurueckflashen).
- Niemals automatisch: `erase/format userdata`, `flashing unlock`, Bootloader-Unlock.

## Tests

```
powershell -ExecutionPolicy Bypass -File tests/Test-Parsers.ps1
```

Prueft ADB/Fastboot/getvar/Partition/getprop-Parser, OS-Klassifizierung, Firmware-Kompatibilitaet, Hashing, Root-Entscheidung (simulierte Ausgaben, nur fuer Tests, nie im Prod-Workflow).
