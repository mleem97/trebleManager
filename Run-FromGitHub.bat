@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - GitHub Direct Run

REM ============================================================
REM  Direkt von GitHub ausfuehren - nur diese URL anpassen:
REM  Nach dem Push: https://raw.githubusercontent.com/USER/REPO/main/TrebleToolkit/scripts/Treble-Toolkit.ps1
REM ============================================================
set "RAW_URL=https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1"
set "TMP_PS=%TEMP%\Treble-Toolkit.ps1"

echo Lade TUI von GitHub ...
echo %RAW_URL%
echo.

powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; try { Invoke-WebRequest -Uri '%RAW_URL%' -OutFile '%TMP_PS%' -UseBasicParsing; Write-Host 'Download OK.' } catch { Write-Host ('FEHLER: ' + $_.Exception.Message); exit 1 }"

if not exist "%TMP_PS%" (
  echo FEHLER: Download fehlgeschlagen. URL oben pruefen (USER/REPO/Branch/Pfad).
  pause
  exit /b 1
)

REM TUI starten (interaktiv, Admin nur bei Bedarf aus der TUI heraus)
set "PSBIN=powershell.exe"
where pwsh.exe >nul 2>&1 && set "PSBIN=pwsh.exe"

"%PSBIN%" -NoProfile -ExecutionPolicy Bypass -File "%TMP_PS%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit-Code: %EC%
if not "%EC%"=="0" pause
exit /b %EC%
