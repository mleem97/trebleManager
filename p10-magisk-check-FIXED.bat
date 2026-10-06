@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul 2>&1
title Huawei P10 - Prerequisite Check (fixed)

REM ============================================================
REM  Huawei P10 - ADB / Fastboot / Magisk Prerequisite Check
REM  FIXED VERSION - fixes 2 bugs of the first draft:
REM
REM  BUG 1: 2^>^&1 written as command
REM   Was: fastboot getvar product 2^>^&1
REM   The ^ escapes the redirection, so fastboot receives
REM   literal "2>&1" as argument ->
REM   "fastboot: usage: unknown command 2>&1"
REM   CORRECT in .bat: >> "%LOG%" 2>&1  (no ^ on real calls)
REM
REM  BUG 2: no auto mode
REM   Script always assumed Android, even when the
REM   device was already in fastboot (6PQ0217B08003446 fastboot)
REM   -> "error: no devices/emulators found"
REM   This version auto-detects Android vs fastboot.
REM ============================================================

set "ADB=adb.exe"
set "FASTBOOT=fastboot.exe"
set "LOG=%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt"

REM Start log (overwrite)
> "%LOG%" echo ============================================================
>>"%LOG%" echo Huawei P10 - ADB / Fastboot / Magisk Prerequisite Check (FIXED)
>>"%LOG%" echo Started: %DATE% %TIME%
>>"%LOG%" echo Serial example: 6PQ0217B08003446
>>"%LOG%" echo ============================================================
>>"%LOG%" echo.

echo ============================================================
echo Huawei P10 - Prerequisite Check (FIXED)
echo ============================================================
echo Log: %LOG%
echo.

REM --- Tools present? ---
where adb.exe >nul 2>&1
if errorlevel 1 (
  if not exist "%~dp0adb.exe" (
    echo ERROR: adb.exe not found. Put .bat next to adb.exe or check PATH.
    >>"%LOG%" echo ERROR: adb.exe not found.
    pause
    exit /b 1
  ) else (
    set "ADB=%~dp0adb.exe"
    set "FASTBOOT=%~dp0fastboot.exe"
  )
)

REM --- Detect mode ---
echo [1] Detecting mode ...
>>"%LOG%" echo [1] Detecting mode ...

set "ADB_DEV="
for /f "skip=1 tokens=1,2" %%A in ('"%ADB%" devices') do (
  if "%%B"=="device" set "ADB_DEV=%%A"
)

if defined ADB_DEV (
  echo Android found: !ADB_DEV!
  >>"%LOG%" echo Android found: !ADB_DEV!
  goto :ANDROID
)

set "FB_DEV="
for /f "tokens=1,2" %%A in ('"%FASTBOOT%" devices') do (
  if "%%B"=="fastboot" set "FB_DEV=%%A"
)

if defined FB_DEV (
  echo Fastboot found: !FB_DEV! - ADB skipped.
  >>"%LOG%" echo Fastboot found: !FB_DEV! - ADB skipped.
  goto :FASTBOOT
)

echo.
echo ERROR: neither ADB nor fastboot found.
echo - USB debugging on? - Authorization confirmed on phone?
echo - Or boot manually to fastboot (Power+Vol-Down).
>>"%LOG%" echo ERROR: no device found.
pause
exit /b 1

:ANDROID
echo.
echo ============================================================
echo [2] Android properties
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo [2] Android properties
>>"%LOG%" echo ============================================================
call :PROP ro.product.model
call :PROP ro.product.name
call :PROP ro.product.device
call :PROP ro.build.version.release
call :PROP ro.build.display.id

echo.
echo ============================================================
echo [3] Boot properties (OS-independent, stock or GSI/custom)
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo [3] Boot properties
>>"%LOG%" echo ============================================================
call :PROP ro.boot.verifiedbootstate
call :PROP ro.boot.flash.locked
call :PROP ro.boot.vbmeta.device_state
call :PROP ro.boot.slot_suffix
call :PROP ro.product.cpu.abi
call :PROP ro.treble.enabled

echo.
echo ============================================================
echo [4] Kernel cmdline
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo [4] Kernel cmdline
>>"%LOG%" echo ============================================================
"%ADB%" shell cat /proc/cmdline >>"%LOG%" 2>&1
"%ADB%" shell cat /proc/cmdline

echo.
echo ============================================================
echo [5] Partitions by-name
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo [5] Partitions by-name
>>"%LOG%" echo ============================================================
"%ADB%" shell ls -l /dev/block/by-name/ >>"%LOG%" 2>&1
"%ADB%" shell ls -l /dev/block/by-name/ | findstr /i "boot recovery ramdisk system vendor vbmeta"

echo.
echo ============================================================
echo Switching to fastboot ...
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo Switching to fastboot ...
"%ADB%" reboot bootloader >>"%LOG%" 2>&1
echo Waiting max. 30s for fastboot ...
set /a CNT=0
:WAITFB
timeout /t 1 /nobreak >nul
set "FB_DEV="
for /f "tokens=1,2" %%A in ('"%FASTBOOT%" devices') do (
  if "%%B"=="fastboot" set "FB_DEV=%%A"
)
if defined FB_DEV goto :FASTBOOT
set /a CNT+=1
if !CNT! GEQ 30 (
  echo ERROR: no fastboot after 30s. Boot to fastboot manually.
  >>"%LOG%" echo ERROR: no fastboot after 30s.
  pause
  exit /b 1
)
goto :WAITFB

:FASTBOOT
echo.
echo ============================================================
echo FASTBOOT (Serial: !FB_DEV!)
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo FASTBOOT - !FB_DEV!
>>"%LOG%" echo ============================================================

REM IMPORTANT: no ^ here. >> LOG 2>&1 is redirection, not an argument.
echo [6] product ...
"%FASTBOOT%" getvar product >>"%LOG%" 2>&1
type "%LOG%" | findstr /i "product" >nul 2>&1

echo [7] secure ...
"%FASTBOOT%" getvar secure >>"%LOG%" 2>&1

echo [8] unlocked - FAILED (remote: Command not allowed) is NORMAL on Huawei, no proof of lock!
"%FASTBOOT%" getvar unlocked >>"%LOG%" 2>&1

echo [9] current-slot - P10 has no classic A/B, empty/FAILED output is fine ...
"%FASTBOOT%" getvar current-slot >>"%LOG%" 2>&1

echo [10] recovery_ramdisk - TARGET partition for Magisk path ...
"%FASTBOOT%" getvar partition-type:recovery_ramdisk >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:recovery_ramdisk >>"%LOG%" 2>&1

echo [11] boot ...
"%FASTBOOT%" getvar partition-type:boot >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:boot >>"%LOG%" 2>&1

echo [12] recovery - do NOT confuse with recovery_ramdisk ...
"%FASTBOOT%" getvar partition-type:recovery >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:recovery >>"%LOG%" 2>&1

echo [13] system - GSI stays intact, read-only ...
"%FASTBOOT%" getvar partition-type:system >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:system >>"%LOG%" 2>&1

echo [14] vendor ...
"%FASTBOOT%" getvar partition-type:vendor >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:vendor >>"%LOG%" 2>&1

echo [15] getvar all - many FAILED are Huawei-normal ...
"%FASTBOOT%" getvar all >>"%LOG%" 2>&1

echo.
echo ============================================================
echo CHECK COMPLETE - nothing flashed, nothing wiped
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo CHECK COMPLETE - %DATE% %TIME%
>>"%LOG%" echo Nothing was flashed or changed.
>>"%LOG%" echo ============================================================
echo Log: %LOG%
echo.
echo [Enter] = reboot to Android, [X] = stay in fastboot
set /p CHOICE="Choice: "
if /i "%CHOICE%"=="X" goto :END
"%FASTBOOT%" reboot >>"%LOG%" 2>&1
:END
echo Done. Upload the desktop log for evaluation.
pause
exit /b 0

:PROP
set "P=%~1"
for /f "delims=" %%V in ('"%ADB%" shell getprop %P%') do (
  echo %P% = %%V
  >>"%LOG%" echo %P% = %%V
)
exit /b 0
