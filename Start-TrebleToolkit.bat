@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - Launcher
set "LOG=%~dp0launcher.log"

REM Usage: Start-TrebleToolkit.bat [/NOELEVATE]
REM Relaunches elevated (new window) unless already admin or /NOELEVATE.
REM Never exits silently: every failure pauses with a readable message.
set "SCRIPT=%~dp0scripts\Treble-Toolkit.ps1"
set "SETUP=%~dp0Setup-TrebleToolkit.bat"
set "CONFIG=%~dp0data\config.json"
if not exist "%SCRIPT%" (
  echo ERROR: "%SCRIPT%" not found.
  echo Keep the extracted folder complete: Start-*.bat + scripts\ + logs\ + data\ + backups\
  echo See INSTRUCTIONS.md for the offline release layout.
  pause
  exit /b 1
)
if not exist "%~dp0logs" mkdir "%~dp0logs" >nul 2>&1

if /i "%~1"=="/NOELEVATE" goto :SKIPELEV
net session >nul 2>&1
if errorlevel 1 (
  echo Requesting admin rights (new window, UAC) ...
  echo If no window appears, confirm the UAC dialog or re-run as admin.
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -ArgumentList '/NOELEVATE %*' -Verb RunAs" >>"%LOG%" 2>&1
  if errorlevel 1 (
    echo WARNING: could not start elevated copy - see "%LOG%".
    echo Continue WITHOUT admin? Fastboot drivers may fail. [Y/n]
    set /p NOADMIN="Choice: "
    if /i not "%NOADMIN%"=="Y" (
      if not "%NOADMIN%"=="" (
        echo Aborted. Log: "%LOG%"
        pause
        exit /b 1
      )
    )
    goto :SKIPELEV
  )
  echo Elevated window should be open now - this window can be closed.
  timeout /t 5 >nul
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
      if errorlevel 1 (
        echo Setup reported an error - continuing anyway, the TUI will guide you.
        pause
      )
    ) else (
      echo WARNING: adb.exe not found and no Setup available.
      echo Install platform-tools or run Setup-TrebleToolkit.bat once.
      pause
    )
  )
)

set "PSBIN="
where pwsh.exe >nul 2>&1 && set "PSBIN=pwsh.exe"
if not defined PSBIN (
  where powershell.exe >nul 2>&1 && set "PSBIN=powershell.exe"
)
if not defined PSBIN (
  echo ERROR: neither pwsh.exe nor powershell.exe found.
  echo Install PowerShell (Windows PowerShell 5.1 is built into Windows 10/11).
  pause
  exit /b 1
)

echo Starting: %PSBIN% -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%"
"%PSBIN%" -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit code: %EC% - log is in logs\
if not "%EC%"=="0" (
  echo Non-zero exit - check the newest file in logs\ for details.
  pause
)
exit /b %EC%
