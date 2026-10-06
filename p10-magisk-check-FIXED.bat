@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul 2>&1
title Huawei P10 - Prerequisite Check (fixed)

REM ============================================================
REM  Huawei P10 - ADB / Fastboot / Magisk Prerequisite Check
REM  FIXED VERSION - erklaert eure 2 Fehler:
REM
REM  FEHLER 1: 2^>^&1 als Befehl geschrieben
REM   Ihr hattet: fastboot getvar product 2^>^&1
REM   Das ^ maskiert die Umleitung, fastboot bekommt
REM   dadurch literal "2>&1" als Argument ->
REM   "fastboot: usage: unknown command 2>&1"
REM   RICHTIG in .bat: >> "%LOG%" 2>&1  (ohne ^ beim echten Aufruf)
REM
REM  FEHLER 2: kein Auto-Modus
REM   Skript ging immer von Android aus, auch wenn das
REM   Geraet schon in Fastboot war (6PQ0217B08003446 fastboot)
REM   -> "error: no devices/emulators found"
REM   Diese Version erkennt automatisch Android vs Fastboot.
REM ============================================================

set "ADB=adb.exe"
set "FASTBOOT=fastboot.exe"
set "LOG=%USERPROFILE%\Desktop\Huawei-P10-Magisk-Check.txt"

REM Log starten (ueberschreiben)
> "%LOG%" echo ============================================================
>>"%LOG%" echo Huawei P10 - ADB / Fastboot / Magisk Prerequisite Check (FIXED)
>>"%LOG%" echo Started: %DATE% %TIME%
>>"%LOG%" echo Serial Beispiel: 6PQ0217B08003446
>>"%LOG%" echo ============================================================
>>"%LOG%" echo.

echo ============================================================
echo Huawei P10 - Prerequisite Check (FIXED)
echo ============================================================
echo Log: %LOG%
echo.

REM --- Tools vorhanden? ---
where adb.exe >nul 2>&1
if errorlevel 1 (
  if not exist "%~dp0adb.exe" (
    echo FEHLER: adb.exe nicht gefunden. .bat neben adb.exe legen oder PATH pruefen.
    >>"%LOG%" echo FEHLER: adb.exe nicht gefunden.
    pause
    exit /b 1
  ) else (
    set "ADB=%~dp0adb.exe"
    set "FASTBOOT=%~dp0fastboot.exe"
  )
)

REM --- Modus erkennen ---
echo [1] Modus erkennen ...
>>"%LOG%" echo [1] Modus erkennen ...

set "ADB_DEV="
for /f "skip=1 tokens=1,2" %%A in ('"%ADB%" devices') do (
  if "%%B"=="device" set "ADB_DEV=%%A"
)

if defined ADB_DEV (
  echo Android gefunden: !ADB_DEV!
  >>"%LOG%" echo Android gefunden: !ADB_DEV!
  goto :ANDROID
)

set "FB_DEV="
for /f "tokens=1,2" %%A in ('"%FASTBOOT%" devices') do (
  if "%%B"=="fastboot" set "FB_DEV=%%A"
)

if defined FB_DEV (
  echo Fastboot gefunden: !FB_DEV! - ADB wird uebersprungen.
  >>"%LOG%" echo Fastboot gefunden: !FB_DEV! - ADB wird uebersprungen.
  goto :FASTBOOT
)

echo.
echo FEHLER: Weder ADB noch Fastboot gefunden.
echo - USB-Debugging an? - Autorisierung am Handy bestaetigt?
echo - Oder manuell in Fastboot booten (Power+Vol-Down).
>>"%LOG%" echo FEHLER: kein Geraet gefunden.
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
echo [3] Boot properties (OS-unabhaengig, Stock wie GSI/Custom)
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
>>"%LOG%" echo [4] Kernel cmdline
>>"%LOG%" echo ============================================================
"%ADB%" shell cat /proc/cmdline >>"%LOG%" 2>&1
"%ADB%" shell cat /proc/cmdline

echo.
echo ============================================================
echo [5] Partitionen by-name
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo [5] Partitionen by-name
>>"%LOG%" echo ============================================================
"%ADB%" shell ls -l /dev/block/by-name/ >>"%LOG%" 2>&1
"%ADB%" shell ls -l /dev/block/by-name/ | findstr /i "boot recovery ramdisk system vendor vbmeta"

echo.
echo ============================================================
echo Wechsle nach Fastboot ...
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo Wechsle nach Fastboot ...
"%ADB%" reboot bootloader >>"%LOG%" 2>&1
echo Warte max. 30s auf Fastboot ...
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
  echo FEHLER: Kein Fastboot nach 30s. Manuell in Fastboot booten.
  >>"%LOG%" echo FEHLER: Kein Fastboot nach 30s.
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

REM WICHTIG: Hier KEIN ^ verwenden. >> LOG 2>&1 ist Umleitung, kein Argument.
echo [6] product ...
"%FASTBOOT%" getvar product >>"%LOG%" 2>&1
type "%LOG%" | findstr /i "product" >nul 2>&1

echo [7] secure ...
"%FASTBOOT%" getvar secure >>"%LOG%" 2>&1

echo [8] unlocked - FAILED (remote: Command not allowed) ist bei Huawei NORMAL, kein Lock-Beweis!
"%FASTBOOT%" getvar unlocked >>"%LOG%" 2>&1

echo [9] current-slot - P10 hat kein klassisches A/B, leere/FAILED Ausgabe ist ok ...
"%FASTBOOT%" getvar current-slot >>"%LOG%" 2>&1

echo [10] recovery_ramdisk - ZIELPARTITION fuer Magisk-Weg ...
"%FASTBOOT%" getvar partition-type:recovery_ramdisk >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:recovery_ramdisk >>"%LOG%" 2>&1

echo [11] boot ...
"%FASTBOOT%" getvar partition-type:boot >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:boot >>"%LOG%" 2>&1

echo [12] recovery - NICHT mit recovery_ramdisk verwechseln ...
"%FASTBOOT%" getvar partition-type:recovery >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:recovery >>"%LOG%" 2>&1

echo [13] system - GSI bleibt erhalten, nur lesen ...
"%FASTBOOT%" getvar partition-type:system >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:system >>"%LOG%" 2>&1

echo [14] vendor ...
"%FASTBOOT%" getvar partition-type:vendor >>"%LOG%" 2>&1
"%FASTBOOT%" getvar partition-size:vendor >>"%LOG%" 2>&1

echo [15] getvar all - viele FAILED sind Huawei-normal ...
"%FASTBOOT%" getvar all >>"%LOG%" 2>&1

echo.
echo ============================================================
echo CHECK COMPLETE - nichts geflasht, nichts geloescht
echo ============================================================
>>"%LOG%" echo.
>>"%LOG%" echo ============================================================
>>"%LOG%" echo CHECK COMPLETE - %DATE% %TIME%
>>"%LOG%" echo Es wurde nichts geflasht oder veraendert.
>>"%LOG%" echo ============================================================
echo Log: %LOG%
echo.
echo [Enter] = nach Android rebooten, [X] = in Fastboot bleiben
set /p CHOICE="Auswahl: "
if /i "%CHOICE%"=="X" goto :END
"%FASTBOOT%" reboot >>"%LOG%" 2>&1
:END
echo Fertig. Log auf Desktop hochladen fuer Auswertung.
pause
exit /b 0

:PROP
set "P=%~1"
for /f "delims=" %%V in ('"%ADB%" shell getprop %P%') do (
  echo %P% = %%V
  >>"%LOG%" echo %P% = %%V
)
exit /b 0
