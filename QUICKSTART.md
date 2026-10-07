# QUICKSTART — trebleManager

Fastest safe path from zero to verified root on Huawei P10 (VTR-L29).

## 1. Check (2 minutes, read-only)

CMD: put `p10-magisk-check-FIXED.bat` next to `adb.exe`, double-click.
It writes `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt` — check that
`recovery_ramdisk` shows up in by-name/getvar. Nothing is flashed.

## 2. Full TUI — ONE central entry point

**Double-click `Start-TrebleToolkit.bat`** (from the release ZIP). It handles
admin rights, first-run setup and then the TUI — nothing else needs opening.
No files yet? Online start: download + double-click `Run-FromGitHub.bat`
(Windows) or run `run-from-github.sh` (Linux, see README) — same result.
Manual alternative (experts only):

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Linux:

```bash
cd /path/to/trebleManager
./scripts/treble-toolkit.sh
```

Follow the wizard: **Detect → Analyze (says which system is on your phone) →
your goal in plain words → plan with visible `[SKIP]`s → run.**
Stock path: Firmware → Extract → Patch → Backup → Flash → Reboot+Verify.
Custom-ROM path: ROM package → Recovery export → Patch → Backup → Flash →
Reboot+Verify (stock firmware steps are skipped — your patch base MUST come
from your ROM, never from stock).

## 3. What you need in hand

- Full EMUI 9.1 firmware (for `UPDATE.APP` → TUI step 3 downloader or `data/firmware/` drop)
- Magisk APK from [official releases](https://github.com/topjohnwu/Magisk/releases) → `data/magisk/`
- The boot cheat after flashing: **Vol-Up + Power until Huawei logo**, then release.
- Root counts only with `uid=0` (`su -c id`); grant rights in the Magisk app.

## 4. If something breaks

Stay in fastboot → TUI **Restore / Unroot** (or CLI `restore`) flashes the
backed-up original back. See [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## 5. Tests

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\Test-Parsers.ps1
```

```bash
bash tests/test-parsers.sh
```

Details: [INSTRUCTIONS.md](INSTRUCTIONS.md), [CONTRIBUTING.md](CONTRIBUTING.md).

---

<a id="de"></a>
## Deutsch — Schnellstart

Schnellster sicherer Weg von null zu verifiziertem Root auf Huawei P10 (VTR-L29).

## 1. Check (2 Minuten, read-only)

CMD: `p10-magisk-check-FIXED.bat` neben `adb.exe` legen, doppelklicken.
Schreibt `%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt` — prüfen, dass
`recovery_ramdisk` in by-name/getvar auftaucht. Nichts wird geflasht.

## 2. Voll-TUI — EIN zentraler Einstieg

**`Start-TrebleToolkit.bat` doppelklicken** (aus dem Release-ZIP). Regelt
Admin-Rechte, Erst-Setup und dann die TUI — sonst muss nichts geöffnet werden.
Noch keine Dateien? Online-Start: `Run-FromGitHub.bat` laden + doppelklicken
(Windows) oder `run-from-github.sh` ausführen (Linux, siehe README) —
gleiches Ergebnis. Manuelle Alternative (nur Experten):

```powershell
cd "C:\path\to\trebleManager"
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Treble-Toolkit.ps1
```

Linux:

```bash
cd /path/to/trebleManager
./scripts/treble-toolkit.sh
```

Dem Wizard folgen: **Detect → Analyze (sagt, welches System auf dem Handy
ist) → Ziel in Alltagssprache → Plan mit sichtbaren `[SKIP]`s → laufen.**
Stock-Weg: Firmware → Extract → Patch → Backup → Flash → Reboot+Verify.
Custom-ROM-Weg: ROM-Paket → Recovery-Export → Patch → Backup → Flash →
Reboot+Verify (Stock-Firmware-Steps werden geskipt — die Patch-Basis MUSS
aus deinem ROM kommen, niemals aus Stock).

## 3. Was du bereit brauchst

- Full-EMUI-9.1-Firmware (für `UPDATE.APP` → TUI-Step-3-Downloader oder `data/firmware/`-Drop)
- Magisk-APK aus [offiziellen Releases](https://github.com/topjohnwu/Magisk/releases) → `data/magisk/`
- Der Boot-Cheat nach Flash: **Vol-Up + Power bis Huawei-Logo**, dann loslassen.
- Root zählt nur mit `uid=0` (`su -c id`); Rechte in der Magisk-App freigeben.

## 4. Wenn etwas bricht

In Fastboot bleiben → TUI **Restore / Unroot** (oder CLI `restore`) flasht
das gesicherte Original zurück. Siehe [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## 5. Tests (Dry Run Tests — ohne Gerät)

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\Test-Parsers.ps1
```

```bash
bash tests/test-parsers.sh
```

Details: [INSTRUCTIONS.md](INSTRUCTIONS.md), [CONTRIBUTING.md](CONTRIBUTING.md).
