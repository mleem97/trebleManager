@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - Launcher

REM Smart launcher: finds pwsh/powershell, requests admin if needed, starts the TUI.
set "SCRIPT=%~dp0scripts\Treble-Toolkit.ps1"
if not exist "%SCRIPT%" (
  echo ERROR: %SCRIPT% not found.
  echo Keep the extracted folder complete: Start-*.bat + scripts\ + logs\ + data\ + backups\
  pause
  exit /b 1
)
if not exist "%~dp0logs" mkdir "%~dp0logs" >nul 2>&1

REM Admin check: net session only succeeds as admin
net session >nul 2>&1
if errorlevel 1 (
  echo No admin. Fastboot/USB drivers often need admin.
  echo Restart with admin now (UAC) [Y] or continue without admin [N]?
  set /p ELEV="Choice Y/N: "
  if /i "%ELEV%"=="Y" (
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs" >nul 2>&1
    exit /b 0
  )
)

set "PSBIN="
where pwsh.exe >nul 2>&1 && set "PSBIN=pwsh.exe"
if not defined PSBIN (
  where powershell.exe >nul 2>&1 && set "PSBIN=powershell.exe"
)
if not defined PSBIN set "PSBIN=powershell.exe"

echo Starting: %PSBIN% -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%"
"%PSBIN%" -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit code: %EC% - log is in logs\
if not "%EC%"=="0" pause
exit /b %EC%
