@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - Launcher

REM Smart-Launcher: findet pwsh/powershell, fordert bei Bedarf Admin an, startet TUI.
set "SCRIPT=%~dp0scripts\Treble-Toolkit.ps1"
if not exist "%SCRIPT%" (
  echo FEHLER: %SCRIPT% nicht gefunden.
  echo Entpackten Ordner komplett lassen: Start-*.bat + scripts\ + logs\ + data\ + backups\
  pause
  exit /b 1
)
if not exist "%~dp0logs" mkdir "%~dp0logs" >nul 2>&1

REM Admin-Check: net session geht nur als Admin
net session >nul 2>&1
if errorlevel 1 (
  echo Kein Admin. Fastboot/USB-Treiber brauchen oft Admin.
  echo [J]etzt mit Admin neu starten (UAC) oder [N]ormal ohne Admin fortfahren?
  set /p ELEV="Auswahl J/N: "
  if /i "%ELEV%"=="J" (
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

echo Starte: %PSBIN% -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%"
"%PSBIN%" -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit-Code: %EC% - Log liegt in logs\
if not "%EC%"=="0" pause
exit /b %EC%
