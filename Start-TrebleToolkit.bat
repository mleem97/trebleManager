@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - Launcher

REM Usage: Start-TrebleToolkit.bat [/NOELEVATE]
REM Always relaunches elevated (new window) unless already admin or /NOELEVATE.
REM First run without tools/config: runs Setup-TrebleToolkit.bat first.
set "SCRIPT=%~dp0scripts\Treble-Toolkit.ps1"
set "SETUP=%~dp0Setup-TrebleToolkit.bat"
set "CONFIG=%~dp0data\config.json"
if not exist "%SCRIPT%" (
  echo ERROR: "%SCRIPT%" not found.
  echo Keep the extracted folder complete: Start-*.bat + scripts\ + logs\ + data\ + backups\
  pause
  exit /b 1
)
if not exist "%~dp0logs" mkdir "%~dp0logs" >nul 2>&1

if /i "%~1"=="/NOELEVATE" goto :SKIPELEV
net session >nul 2>&1
if errorlevel 1 (
  echo Requesting admin rights (new window, UAC) ...
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -ArgumentList '/NOELEVATE %*' -Verb RunAs" >nul 2>&1
  exit /b 0
)
:SKIPELEV

REM First run: no tools on PATH and no saved config -> setup first.
where adb.exe >nul 2>&1
if errorlevel 1 (
  if not exist "%CONFIG%" (
    if exist "%SETUP%" (
      echo First run: required tools missing - starting setup ...
      call "%SETUP%"
    )
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
