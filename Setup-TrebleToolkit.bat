@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title trebleManager - First-run Setup

REM First-run setup: auto UAC elevation, execution policy, ADB/fastboot +
REM scrcpy install into user PATH, saved config (data\config.json).
set "SCRIPT=%~dp0scripts\Setup-Windows.ps1"
if not exist "%SCRIPT%" (
  echo ERROR: %SCRIPT% not found.
  pause
  exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit code: %EC%
if not "%EC%"=="0" pause
exit /b %EC%
