@echo off
setlocal EnableExtensions
chcp 65001 >nul 2>&1
title Huawei P10 Root Manager - Online Start (zentraler Einstieg)

REM ============================================================
REM  Zentraler Online-Einstieg / central online entry point.
REM  Laedt das VOLLE Release-ZIP (inkl. scripts\, data\Registry,
REM  Setup, TUI) und startet danach IMMER den zentralen Starter:
REM      Start-TrebleToolkit.bat  (= THE entry point)
REM  Kein stilles Beenden: jeder Fehler pausiert mit Klartext.
REM  Offline-Alternative: Release-ZIP per USB-Stick, siehe README.
REM ============================================================
set "REPO=mleem97/trebleManager"
set "API=https://api.github.com/repos/%REPO%/releases/latest"
set "BASE=%LOCALAPPDATA%\trebleManager"
if not exist "%BASE%" mkdir "%BASE%" >nul 2>&1
set "BOOTLOG=%BASE%\online-start.log"
echo [%DATE% %TIME%] online start >>"%BOOTLOG%" 2>&1

where powershell.exe >nul 2>&1
if errorlevel 1 (
  echo FEHLER / ERROR: powershell.exe nicht gefunden / not found.
  echo Windows PowerShell 5.1 ist in Windows 10/11 eingebaut.
  pause
  exit /b 1
)

echo Ermittle neueste Release-Version / resolving latest release ...
echo raw VERSION first (no API rate limit), GitHub API as fallback.
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $t=''; try { $t='v'+((Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/mleem97/trebleManager/main/VERSION' -UseBasicParsing).Content.Trim()) } catch {}; if ($t -eq '' -or $t -eq 'v') { try { $t=(Invoke-RestMethod -Uri '%API%' -UseBasicParsing).tag_name } catch { Write-Host ('ERROR: ' + $_.Exception.Message); exit 1 } }; $t" >"%BASE%\latest.txt" 2>>"%BOOTLOG%"
if errorlevel 1 (
  echo FEHLER: GitHub-API nicht erreichbar (Netzwerk/Proxy?). Siehe "%BOOTLOG%".
  echo Alternative: Release-ZIP manuell laden, entpacken, Start-TrebleToolkit.bat doppelklicken.
  pause
  exit /b 1
)
for /f "usebackq delims=" %%t in ("%BASE%\latest.txt") do set "TAG=%%t"
echo Version / version: %TAG%
if "%TAG%"=="" (
  echo FEHLER: leere Versionsantwort / empty version response.
  pause
  exit /b 1
)

set "DEST=%BASE%\%TAG%"
set "STARTER=%DEST%\Start-TrebleToolkit.bat"
if /i "%~1"=="gui" goto :GUI
if exist "%STARTER%" (
  echo Vorhanden / already present: %DEST%
  goto :LAUNCH
)

set "ZIPNAME=trebleManager-%TAG%.zip"
set "ZIPURL=https://github.com/%REPO%/releases/download/%TAG%/%ZIPNAME%"
set "ZIP=%BASE%\%ZIPNAME%"
set "SHAURL=%ZIPURL%.sha256"
set "SHA=%ZIP%.sha256"
echo Lade / downloading: %ZIPURL%
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; try { Invoke-WebRequest -Uri '%ZIPURL%' -OutFile '%ZIP%' -UseBasicParsing; Write-Host 'Download OK.' } catch { Write-Host ('ERROR: ' + $_.Exception.Message); exit 1 }" 2>>"%BOOTLOG%"
if errorlevel 1 (
  echo FEHLER: Download fehlgeschlagen. Pruefe Netzwerk und "%BOOTLOG%".
  pause
  exit /b 1
)
if not exist "%ZIP%" (
  echo FEHLER: ZIP fehlt nach Download / ZIP missing after download.
  pause
  exit /b 1
)

echo Pruefe SHA256 / verifying SHA256 ...
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; try { Invoke-WebRequest -Uri '%SHAURL%' -OutFile '%SHA%' -UseBasicParsing } catch { Write-Host 'WARN: no .sha256 asset, skipping verify.'; exit 0 }; $exp = ((Get-Content '%SHA%' -TotalCount 1) -split '\s+')[0]; $act = (Get-FileHash '%ZIP%' -Algorithm SHA256).Hash; if ($act -eq $exp) { Write-Host 'SHA256 OK.' } else { Write-Host ('SHA256 MISMATCH! exp=' + $exp + ' got=' + $act); exit 2 }" 2>>"%BOOTLOG%"
if errorlevel 2 (
  echo FEHLER: SHA256 stimmt NICHT - Datei geloescht, Abbruch (Sicherheit).
  del "%ZIP%" >nul 2>&1
  pause
  exit /b 1
)

echo Entpacke / extracting nach %DEST% ...
if not exist "%DEST%" mkdir "%DEST%" >nul 2>&1
powershell -NoProfile -ExecutionPolicy Bypass -Command "try { Expand-Archive -Path '%ZIP%' -DestinationPath '%DEST%' -Force; Write-Host 'Extract OK.' } catch { Write-Host ('ERROR: ' + $_.Exception.Message); exit 1 }" 2>>"%BOOTLOG%"
if errorlevel 1 (
  echo FEHLER: Entpacken fehlgeschlagen / extract failed. Siehe "%BOOTLOG%".
  pause
  exit /b 1
)
if not exist "%STARTER%" (
  echo FEHLER: zentraler Starter fehlt im ZIP / central starter missing in ZIP:
  echo   %STARTER%
  pause
  exit /b 1
)

:LAUNCH
echo.
echo Starte zentralen Einstieg / launching central starter:
echo   %STARTER%
echo (Der Starter uebernimmt Admin-Rechte, Setup und TUI.)
call "%STARTER%" %*
set "EC=%ERRORLEVEL%"
echo.
echo Exit code: %EC%
if not "%EC%"=="0" pause
exit /b %EC%

:GUI
REM GUI installieren + starten (gsi-root Slint-App, Prebuilt-Binary aus Release).
echo.
echo GUI-Modus / GUI mode: install + run gsi-root
if /i not "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
  if /i not "%PROCESSOR_ARCHITECTURE%"=="x86" (
    echo FEHLER: kein Prebuilt-Binary fuer diese CPU (%PROCESSOR_ARCHITECTURE%).
    echo No prebuilt binary for this CPU. Fallback: TUI via Run-FromGitHub.bat (ohne gui).
    pause
    exit /b 1
  )
)
set "GBIN=%LOCALAPPDATA%\gsi-root\bin\gsi-root.exe"
if exist "%GBIN%" (
  echo Vorhanden / already installed: %GBIN%
  goto :GUIRUN
)
set "GZIPNAME=gsi-root-windows-x86_64.zip"
set "GZIPURL=https://github.com/%REPO%/releases/download/%TAG%/%GZIPNAME%"
set "GZIP=%BASE%\%GZIPNAME%"
echo Lade GUI / downloading GUI: %GZIPURL%
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; try { Invoke-WebRequest -Uri '%GZIPURL%' -OutFile '%GZIP%' -UseBasicParsing; Write-Host 'Download OK.' } catch { Write-Host ('ERROR: ' + $_.Exception.Message); exit 1 }" 2>>"%BOOTLOG%"
if errorlevel 1 (
  echo FEHLER: GUI-Download fehlgeschlagen. Siehe "%BOOTLOG%".
  pause
  exit /b 1
)
echo Pruefe SHA256 / verifying ...
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; try { Invoke-WebRequest -Uri '%GZIPURL%.sha256' -OutFile '%GZIP%.sha256' -UseBasicParsing } catch { Write-Host 'WARN: no .sha256 asset, skipping verify.'; exit 0 }; $exp = ((Get-Content '%GZIP%.sha256' -TotalCount 1) -split '\s+')[0]; $act = (Get-FileHash '%GZIP%' -Algorithm SHA256).Hash; if ($act -eq $exp) { Write-Host 'SHA256 OK.' } else { Write-Host ('SHA256 MISMATCH!'); exit 2 }" 2>>"%BOOTLOG%"
if errorlevel 2 (
  echo FEHLER: SHA256 stimmt NICHT - Datei geloescht, Abbruch.
  del "%GZIP%" >nul 2>&1
  pause
  exit /b 1
)
if not exist "%LOCALAPPDATA%\gsi-root\bin" mkdir "%LOCALAPPDATA%\gsi-root\bin" >nul 2>&1
echo Entpacke nach / installing to %LOCALAPPDATA%\gsi-root\bin ...
powershell -NoProfile -ExecutionPolicy Bypass -Command "try { Expand-Archive -Path '%GZIP%' -DestinationPath '%LOCALAPPDATA%\gsi-root\bin' -Force; Write-Host 'Install OK.' } catch { Write-Host ('ERROR: ' + $_.Exception.Message); exit 1 }" 2>>"%BOOTLOG%"
if errorlevel 1 (
  echo FEHLER: Entpacken fehlgeschlagen.
  pause
  exit /b 1
)
if not exist "%GBIN%" (
  echo FEHLER: gsi-root.exe fehlt nach Install / missing after install.
  pause
  exit /b 1
)
echo Trage in User-PATH ein / adding to user PATH ...
powershell -NoProfile -ExecutionPolicy Bypass -Command "$d='%LOCALAPPDATA%\gsi-root\bin'; $p=[Environment]::GetEnvironmentVariable('Path','User'); if ($p -split ';' -notcontains $d) { [Environment]::SetEnvironmentVariable('Path',($p+';'+$d),'User'); Write-Host 'PATH updated (neue Terminals).' } else { Write-Host 'PATH already set.' }" 2>>"%BOOTLOG%"
:GUIRUN
echo.
echo Starte GUI / launching GUI: %GBIN%
start "" "%GBIN%"
exit /b 0
