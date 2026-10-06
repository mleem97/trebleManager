@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - GitHub Direct Run

REM ============================================================
REM  Run straight from GitHub - RAW_URL already points to
REM  mleem97/trebleManager main. No manual download needed
REM  (requires GitHub to be reachable; otherwise use the
REM  offline release ZIP, see INSTRUCTIONS.md).
REM ============================================================
set "RAW_URL=https://raw.githubusercontent.com/mleem97/trebleManager/main/scripts/Treble-Toolkit.ps1"
set "TMP_PS=%TEMP%\Treble-Toolkit.ps1"

echo Downloading TUI from GitHub ...
echo %RAW_URL%
echo.

powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; try { Invoke-WebRequest -Uri '%RAW_URL%' -OutFile '%TMP_PS%' -UseBasicParsing; Write-Host 'Download OK.' } catch { Write-Host ('ERROR: ' + $_.Exception.Message); exit 1 }"

if not exist "%TMP_PS%" (
  echo ERROR: download failed. Check network/URL (branch/path).
  pause
  exit /b 1
)

REM Start TUI (interactive, admin only on demand from inside the TUI)
set "PSBIN=powershell.exe"
where pwsh.exe >nul 2>&1 && set "PSBIN=pwsh.exe"

"%PSBIN%" -NoProfile -ExecutionPolicy Bypass -File "%TMP_PS%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit code: %EC%
if not "%EC%"=="0" pause
exit /b %EC%
