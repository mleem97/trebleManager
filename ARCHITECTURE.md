# ARCHITECTURE

TrebleToolkit v2.0.0 ist bewusst als **eine autarke PowerShell-Datei** (`scripts/Treble-Toolkit.ps1`, PS 5.1 + 7)
umgesetzt, damit sie per `irm`/`Run-FromGitHub.bat` direkt von GitHub lauffaehig ist
(User-Vorgabe: 100% TUI per URL). Die vom Auftrag bevorzugte Rust/Tauri-Trennung ist
als Modul-Regionen in der Datei abgebildet und laesst sich 1:1 nach Rust portieren.

## Modul-Regionen (-> Rust-core-Mapping)

- `core/device`: `$DeviceProfiles` (VTR-L29/L09, VKY-L29), `Update-TTMode` (android/fastboot/none, kein unnoetiger Reboot).
- `core/adb`: `Find-TTTools`, `Invoke-TTAdb`, `Get-TTProp`, Prop-Liste inkl. `ro.product.*`, `ro.boot.*`, `ro.treble.enabled`.
- `core/fastboot`: `Invoke-TTFastboot`, `ConvertFrom-FastbootGetvar` (tolerant gegen `Command not allowed`), Var-Liste aus Spec.
- `core/partitions`: `ConvertFrom-ByNameListing`, `Invoke-TTAndroidAnalysis` + `Invoke-TTFastbootAnalysis`.
- `core/firmware`: `Get-TTFirmwareBaseline` (Stock direkt, GSI assistiert), `Test-FirmwareCompatibility` (Modell-Familie strikt, Submodell/Region/EMUI als WARN, Beispiel-Firmware nur Advisory), `Invoke-TTUpdateAppAnalysis`, `Find-TTRecoveryImage` (exakte Namen, kein Umbenennen).
- `core/magisk`: `$MagiskCompatTable` (nicht blind latest), `Find-TTMagiskApk`, `Get-TTMagiskInfo` (Version+SHA256), `Prepare-TTMagiskPatch` (staging + Anleitung, Erkennung via adb pull/Filepicker, Hash-Ungleichheit Pflicht).
- `core/backup`: `New-TTBackup` (`backups/<MODEL>/recovery_ramdisk/<stamp>/` + metadata.json/sha256/sha512/partition-probe; dd-Versuch ehrlich dokumentiert, nie vorgetaeuscht).
- `core/flashing`: `Test-TTFlashReadiness` (8 Checks), `Invoke-TTSafeFlash` (Befehl aus Profil abgeleitet, WARNING-Dialog, Doppel-Bestaetigung).
- `core/verification`: `Invoke-TTRootVerification` (wait-for-device, `which su`, `su -c id` -> nur `uid=0` = ROOTED, Boot allein != Root), `Invoke-TTRestoreFlow`, `New-TTDiagnostic` (ZIP mit 10 Dateien, `--anonymize`).
- `ui`: TUI (`Show-TTMenu` Pfeiltasten, `Show-TTHeader` mit Modus/Profil/OS, Screens Step 1-9, Status, Bootkeys, Tools, Logs, Wizard).
- `cli`: Dispatch `detect|analyze|firmware|extract|patch|backup|flash|verify|restore|diagnostic|wizard|help` + `--json/--yes/--image/--firmware-file/--anonymize/--no-reboot`.

## OS-Unabhaengigkeit

`Get-OSClassification` klassifiziert aus getprop (jedes OS): TrebleDroid-GSI, Lineage-GSI,
Pixel/Superior/AOSP-GSI, Stock-EMUI-8/9/9.1, Custom-ROM, Unknown. GSI versteckt Huawei-Basis
-> Baseline assistiert (User-Angabe + fastboot product + CUST). Flash-Profil bleibt OS-unabhaengig
(`recovery_ramdisk`), GSI bleibt erhalten.

## Datenfluesse

1. Detect -> Analyze (Props + by-name + getvar, OS-Klasse) -> Firmware (Baseline + Compat).
2. Extract (UPDATE.APP + Image-Validierung: Size/Hash/Header) -> Patch (echt, on-device).
3. Backup (Pflicht) -> Safety-Gate -> Confirm -> Flash -> Reboot (Huawei-Bootkeys) -> Verify (uid=0).
4. Bei FAIL/Bootloop: Diagnose-ZIP -> Restore (Hash+Partition+Confirm).

## Warum kein Rust-Build hier

User-Endanforderung ueberschreibt Rust-Wunsch: sofortiger URL-Run ohne Build.
Rust-Port: jede PS-Region entspricht einem `core/*`-Modul, Parser sind pure Functions mit Unit-Tests
in `tests/Test-Parsers.ps1` (simulierte Ausgaben nur dort).
