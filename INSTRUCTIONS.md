# Instructions: CMD, PowerShell, offline release

Covers repo `mleem97/trebleManager` (branch `main`). Everything also works **without
GitHub access** once the ZIP has been transferred once (USB stick). Internet is only
needed afterwards for optional firmware/Magisk downloads.

Suite docs (same core, GUI-first + TUI + CLI): [docs/GUI.md](docs/GUI.md),
[docs/TUI.md](docs/TUI.md), [docs/CLI.md](docs/CLI.md) (full command reference),
[docs/WORKFLOWS.md](docs/WORKFLOWS.md), [docs/UI-ARCHITECTURE.md](docs/UI-ARCHITECTURE.md).
Steps below stay valid.

## 0. Requirements (all paths)

- Windows 10/11, PowerShell 5.1 (built in) or PowerShell 7
- First run: double-click `Setup-TrebleToolkit.bat` — auto-UAC, execution policy,
  installs ADB/fastboot (+ optional scrcpy with your consent) into user PATH,
  saves `data/config.json`. Skip only if tools are already on PATH.
- USB drivers (HiSuite/Kirin), USB debugging on the P10, cable straight into the PC
- Keep the folder layout intact: `scripts\`, `data\firmware`, `data\magisk`, `data\roms`, `logs\`, `backups\`

## A. Via CMD (quickest check, no PowerShell TUI)

1. Put `p10-magisk-check-FIXED.bat` next to `adb.exe`
   (or into the extracted release folder if `adb.exe` is on PATH).
2. Double-click (confirm admin with `Y` if asked).
3. The script auto-detects Android vs. fastboot and writes
   `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt`.
4. Evaluate/upload that TXT first — only then continue towards root.

## B. Via PowerShell (full TUI + CLI, Windows)

Interactive TUI (recommended):

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Or double-click starter: `Start-TrebleToolkit.bat` (asks for elevation if needed).

## B2. Via bash (Linux, same logic)

```bash
cd /path/to/trebleManager
chmod +x scripts/treble-toolkit.sh
./scripts/treble-toolkit.sh                 # TUI
./scripts/treble-toolkit.sh detect --json   # CLI
./scripts/treble-toolkit.sh diagnostic --anonymize
```

Install: `sudo apt install android-tools-adb android-tools-fastboot`
(Debian/Ubuntu) or your distro's platform-tools. udev rules for Huawei
(`12d1`) may be needed for non-root USB access.

Straight from GitHub (only if GitHub is reachable):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $f=\"$env:TEMP\Treble-Toolkit.ps1\"; Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1' -OutFile $f -UseBasicParsing; & $f"
```

CLI examples (append after `& $f` for remote runs):

```powershell
.\scripts\Treble-Toolkit.ps1 detect --json
.\scripts\Treble-Toolkit.ps1 analyze
.\scripts\Treble-Toolkit.ps1 download --firmware-file "<https-URL-to-full-ZIP>" --yes
.\scripts\Treble-Toolkit.ps1 extract
.\scripts\Treble-Toolkit.ps1 export --image ".\data\roms\custom-rom.zip"
.\scripts\Treble-Toolkit.ps1 backup
.\scripts\Treble-Toolkit.ps1 flash --image ".\data\magisk\magisk_patched.img" --yes
.\scripts\Treble-Toolkit.ps1 verify
.\scripts\Treble-Toolkit.ps1 diagnostic --anonymize
```

## C. As release ZIP (GitHub blocked / manual upload)

1. Copy `trebleManager-vX.Y.Z.zip` + `trebleManager-vX.Y.Z.zip.sha256` from any source
   (USB stick) — no git, no GitHub needed.
2. Verify (PowerShell):
   ```powershell
   (Get-FileHash .\trebleManager-vX.Y.Z.zip -Algorithm SHA256).Hash -eq ((Get-Content .\trebleManager-vX.Y.Z.zip.sha256) -split '\s+')[0]
   ```
   Must be `True`, otherwise transfer again.
3. Extract (path without spaces preferred, e.g. `C:\trebleManager\`),
   check layout: `scripts\Treble-Toolkit.ps1`, `Start-TrebleToolkit.bat`,
   `p10-magisk-check-FIXED.bat`, `INSTRUCTIONS.md`, `data\`, `logs\`, `backups\`.
4. Continue with A/B. For offline use, pre-place via stick:
   - full firmware ZIP into `data\firmware\`
   - Magisk APK (official GitHub release) into `data\magisk\`
   - custom ROM packages into `data\roms\` (for recovery export)
   Then the tool needs no internet at all.

## Manual release upload (for whoever cuts the release)

1. Build ZIP + `.sha256` as above (content = repo root without `.git`, without `logs/*`, without `backups/*`).
2. On GitHub: Releases → Draft new release → tag `vX.Y.Z` → attach both files → Publish.
3. Blocked users download both files from the release page directly (or via stick).

## Device order (short)

1. Check via A, evaluate TXT (`recovery_ramdisk` in by-name/fastboot?).
2. Optional: unlock status (TUI unlock guide), ROM install first if no GSI yet
   (`Install ROM / GSI`, verified profiles only, eRecovery wipe after).
3. TUI wizard: Detect → Analyze (says which system is on the phone: Stock/supported ROM/other) → goal in plain words (root only / install ROM / back to stock) → plan with visible `[SKIP]`s → run. Stock path: Firmware → Extract → Patch → Backup → Flash (`FLASH`+`YES`) → reboot with `Vol-Up + Power until logo` → Verify (`uid=0`). Custom-ROM path: ROM package → Recovery export → Patch → Backup → Flash → Verify (stock steps skipped — patch base MUST come from that ROM, never stock).
   Alternatives in the same menu: root-method priority (Magisk preferred), TWRP path (shared slot warning), ROM/GSI install (verified profiles only).
4. LineageOS (or any custom ROM) running? Valid start — tell the wizard which ROM it is; the patch base comes from that ROM's package, never from stock firmware.
5. On problems: Restore (original from `backups\`) instead of experiments.

---

<a id="de"></a>
## Deutsch — Anleitung: CMD, PowerShell, Offline-Release

Gilt für Repo `mleem97/trebleManager` (Branch `main`). Alles geht auch **ohne
GitHub-Zugang**, sobald das ZIP einmal übertragen wurde (USB-Stick). Internet
ist danach nur noch für optionale Firmware-/Magisk-Downloads nötig.

Suite-Docs (gleicher Core, GUI-first + TUI + CLI): [docs/GUI.md](docs/GUI.md),
[docs/TUI.md](docs/TUI.md), [docs/CLI.md](docs/CLI.md) (volle Command-Referenz),
[docs/WORKFLOWS.md](docs/WORKFLOWS.md), [docs/UI-ARCHITECTURE.md](docs/UI-ARCHITECTURE.md).
Steps unten bleiben gültig.

## 0. Voraussetzungen (alle Wege)

- Windows 10/11, PowerShell 5.1 (eingebaut) oder PowerShell 7
- Erster Start: `Setup-TrebleToolkit.bat` doppelklicken — Auto-UAC,
  Execution-Policy, installiert ADB/fastboot (+ optional scrcpy mit deiner
  Zustimmung) in User-PATH, speichert `data/config.json`. Nur überspringen,
  wenn Tools schon in PATH sind.
- USB-Treiber (HiSuite/Kirin), USB-Debugging am P10 an, Kabel direkt an den PC
- Ordner-Layout intakt halten: `scripts\`, `data\firmware`, `data\magisk`,
  `data\roms\`, `logs\`, `backups\`

## A. Via CMD (schnellster Check, keine PowerShell-TUI)

1. `p10-magisk-check-FIXED.bat` neben `adb.exe` legen
   (oder in den entpackten Release-Ordner, wenn `adb.exe` in PATH ist).
2. Doppelklicken (ggf. Admin mit `Y` bestätigen).
3. Das Script erkennt Android vs. Fastboot automatisch und schreibt
   `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt`.
4. Erst dieses TXT auswerten/hochladen — dann Richtung Root weiter.

## B. Via PowerShell (volle TUI + CLI, Windows)

Interaktive TUI (empfohlen):

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Oder Starter doppelklicken: `Start-TrebleToolkit.bat` (fragt ggf. nach Elevation).

## B2. Via Bash (Linux, gleiche Logik)

```bash
cd /path/to/trebleManager
chmod +x scripts/treble-toolkit.sh
./scripts/treble-toolkit.sh                 # TUI
./scripts/treble-toolkit.sh detect --json   # CLI
./scripts/treble-toolkit.sh diagnostic --anonymize
```

Install: `sudo apt install android-tools-adb android-tools-fastboot`
(Debian/Ubuntu) oder Platform-Tools deiner Distro. udev-Regeln für Huawei
(`12d1`) ggf. für USB ohne Root nötig.

Direkt von GitHub (nur wenn GitHub erreichbar ist) — empfohlen: Starter laden:

```cmd
curl -fsSL -o "%TEMP%\Run-FromGitHub.bat" https://raw.githubusercontent.com/mleem97/trebleManager/main/Run-FromGitHub.bat && "%TEMP%\Run-FromGitHub.bat"
```

```powershell
$b="$env:TEMP\Run-FromGitHub.bat"; iwr -UseBasicParsing -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/Run-FromGitHub.bat' -OutFile $b; & $b
```

GUI: `& $b gui`. Reiner Einzeiler (installiert + startet):

```powershell
irm https://raw.githubusercontent.com/mleem97/trebleManager/main/Install-Online.ps1 | iex
```

CLI-Beispiele (`--image` / `--firmware-file` anhängen):

```powershell
.\scripts\Treble-Toolkit.ps1 detect --json
.\scripts\Treble-Toolkit.ps1 analyze
.\scripts\Treble-Toolkit.ps1 download --firmware-file "<https-URL-zum-Full-ZIP>" --yes
.\scripts\Treble-Toolkit.ps1 extract
.\scripts\Treble-Toolkit.ps1 export --image ".\data\roms\custom-rom.zip"
.\scripts\Treble-Toolkit.ps1 backup
.\scripts\Treble-Toolkit.ps1 flash --image ".\data\magisk\magisk_patched.img" --yes
.\scripts\Treble-Toolkit.ps1 verify
.\scripts\Treble-Toolkit.ps1 diagnostic --anonymize
```

## C. Als Release-ZIP (GitHub blockiert / manueller Upload)

1. `trebleManager-vX.Y.Z.zip` + `trebleManager-vX.Y.Z.zip.sha256` aus beliebiger
   Quelle kopieren (USB-Stick) — kein Git, kein GitHub nötig.
2. Verifizieren (PowerShell):
   ```powershell
   (Get-FileHash .\trebleManager-vX.Y.Z.zip -Algorithm SHA256).Hash -eq ((Get-Content .\trebleManager-vX.Y.Z.zip.sha256) -split '\s+')[0]
   ```
   Muss `True` sein, sonst erneut übertragen.
3. Entpacken (Pfad ohne Leerzeichen bevorzugt, z.B. `C:\trebleManager\`),
   Layout prüfen: `scripts\Treble-Toolkit.ps1`, `Start-TrebleToolkit.bat`,
   `p10-magisk-check-FIXED.bat`, `INSTRUCTIONS.md`, `data\`, `logs\`, `backups\`.
4. Weiter mit A/B. Für Offline-Nutzung vorab per Stick ablegen:
   - Full-Firmware-ZIP nach `data\firmware\`
   - Magisk-APK (offizielles GitHub-Release) nach `data\magisk\`
   - Custom-ROM-Pakete nach `data\roms\` (für Recovery-Export)
   Dann braucht das Tool gar kein Internet.

## Manueller Release-Upload (für Release-Ersteller)

1. ZIP + `.sha256` wie oben bauen (Inhalt = Repo-Root ohne `.git`, ohne `logs/*`, ohne `backups/*`).
2. Auf GitHub: Releases → Draft new release → Tag `vX.Y.Z` → beide Dateien anhängen → Publish.
3. Blockierte User laden beide Dateien direkt von der Release-Seite (oder per Stick).

## Geräte-Reihenfolge (kurz)

1. Check via A, TXT auswerten (`recovery_ramdisk` in by-name/getvar?).
2. Optional: Unlock-Status (TUI-Unlock-Anleitung), ggf. erst ROM installieren
   wenn noch kein GSI da (`ROM / GSI installieren`, nur verifizierte Profile,
   danach eRecovery-Wipe).
3. TUI-Wizard: Detect → Analyze (sagt, welches System auf dem Handy ist:
   Stock/supported ROM/other) → Ziel in Alltagssprache (nur Root / ROM
   installieren / zurück zu Stock) → Plan mit sichtbaren `[SKIP]`s → laufen.
   Stock-Weg: Firmware → Extract → Patch → Backup → Flash (`FLASH`+`YES`) →
   Reboot mit `Vol-Up + Power bis Logo` → Verify (`uid=0`). Custom-ROM-Weg:
   ROM-Paket → Recovery-Export → Patch → Backup → Flash → Verify
   (Stock-Steps geskipt — Patch-Basis MUSS aus diesem ROM kommen, niemals
   aus Stock).
   Alternativen im Menü: Root-Methoden-Prio (Magisk bevorzugt), TWRP-Pfad
   (Shared-Slot-Warnung), ROM/GSI-Installation (nur verifizierte Profile).
4. LineageOS (oder Custom-ROM) läuft? Gültiger Start — dem Wizard sagen,
   welches ROM es ist; die Patch-Basis kommt aus diesem ROM-Paket, niemals
   aus Stock-Firmware.
5. Bei Problemen: Restore (Original aus `backups\`) statt Experimente.
