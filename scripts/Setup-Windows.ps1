<#
.SYNOPSIS
  trebleManager first-run Setup (Windows): admin elevation, execution policy,
  ADB/fastboot + scrcpy install into PATH, saved config (data/config.json).
  Run: right-click Setup-TrebleToolkit.bat -> it self-elevates via UAC.
#>
param([switch]$NoElevate)

$ErrorActionPreference = "Continue"
$TTLang = "en"
try { if ([System.Globalization.CultureInfo]::CurrentUICulture.TwoLetterISOLanguageName -eq "de") { $TTLang = "de" } } catch {}
function L { param([string]$En, [string]$De) if ($TTLang -eq "de") { return $De }; return $En }

function Test-Admin {
  try {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    return (New-Object Security.Principal.WindowsPrincipal($id)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  } catch { return $false }
}

# --- 1. Self-elevation (fully automatic UAC request) ---
if (-not $NoElevate -and -not (Test-Admin)) {
  Write-Host (L "Requesting admin rights (UAC) ..." "Fordere Admin-Rechte an (UAC) ...") -ForegroundColor Cyan
  $me = $MyInvocation.MyCommand.Path
  if ([string]::IsNullOrEmpty($me)) { $me = $PSCommandPath }
  try {
    Start-Process -FilePath "powershell.exe" -ArgumentList ("-NoProfile -ExecutionPolicy Bypass -File `"$me`"") -Verb RunAs | Out-Null
    exit 0
  } catch {
    Write-Host (L "Elevation denied, continuing without admin (PATH install falls back to user scope)." "Elevation abgelehnt, weiter ohne Admin (PATH-Install nutzt User-Scope).") -ForegroundColor Yellow
  }
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrEmpty($ScriptDir)) { $ScriptDir = (Get-Location).Path }
$ToolRoot = $ScriptDir
if ((Split-Path -Leaf $ScriptDir) -eq "scripts") { $ToolRoot = Split-Path -Parent $ScriptDir }
$DataDir = Join-Path $ToolRoot "data"
$CfgFile = Join-Path $DataDir "config.json"
$InstallDir = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "trebleManager\tools"
New-Item -ItemType Directory -Path $DataDir, $InstallDir -Force | Out-Null

# --- 2. Execution policy: allow local scripts (process + current user) ---
try {
  Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process -Force -ErrorAction SilentlyContinue | Out-Null
  $cur = Get-ExecutionPolicy -Scope CurrentUser -ErrorAction SilentlyContinue
  if ($cur -eq "Restricted" -or $cur -eq "Undefined" -or $cur -eq "AllSigned") {
    Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force -ErrorAction Stop | Out-Null
    Write-Host (L "ExecutionPolicy CurrentUser -> RemoteSigned (scripts allowed)." "ExecutionPolicy CurrentUser -> RemoteSigned (Skripte erlaubt).") -ForegroundColor Green
  } else {
    Write-Host ("ExecutionPolicy CurrentUser: $cur (ok).") -ForegroundColor Green
  }
} catch {
  Write-Host ((L "Could not set ExecutionPolicy, use: powershell -ExecutionPolicy Bypass ... Reason: " "ExecutionPolicy nicht setzbar, nutze: powershell -ExecutionPolicy Bypass ... Grund: ") + $_.Exception.Message) -ForegroundColor Yellow
}

function Find-Bin {
  param([string[]]$Names, [string[]]$Paths)
  foreach ($n in $Names) {
    try { $c = Get-Command $n -ErrorAction SilentlyContinue; if ($c -and (Test-Path $c.Source)) { return $c.Source } } catch {}
  }
  foreach ($p in $Paths) { if (Test-Path $p) { return $p } }
  return ""
}

function Add-ToUserPath {
  param([string]$Dir)
  $cur = [Environment]::GetEnvironmentVariable("Path", "User")
  if ($cur -split ";" | Where-Object { $_ -eq $Dir }) { return }
  [Environment]::SetEnvironmentVariable("Path", ($cur + ";" + $Dir), "User") | Out-Null
  $env:Path = $env:Path + ";" + $Dir
  Write-Host ((L "Added to user PATH: " "Zu User-PATH hinzugefuegt: ") + $Dir) -ForegroundColor Green
}

function Install-ZipTool {
  param([string]$Url, [string]$DestDir, [string]$Label)
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("tt-setup-" + [System.IO.Path]::GetFileName($Url).Split("?")[0])
  Write-Host ((L "Downloading " "Lade ") + $Label + " ...") -ForegroundColor Cyan
  try {
    $wc = New-Object System.Net.WebClient
    $wc.DownloadFile($Url, $tmp)
  } catch {
    Write-Host ((L "Download failed: " "Download fehlgeschlagen: ") + $_.Exception.Message) -ForegroundColor Red
    return ""
  }
  try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue | Out-Null
    if (Test-Path $DestDir) { Remove-Item $DestDir -Recurse -Force -ErrorAction SilentlyContinue }
    [System.IO.Compression.ZipFile]::ExtractToDirectory($tmp, $DestDir)
    Remove-Item $tmp -Force -ErrorAction SilentlyContinue
    Write-Host ((L "Extracted to: " "Entpackt nach: ") + $DestDir) -ForegroundColor Green
    return $DestDir
  } catch {
    Write-Host ((L "Extract failed: " "Entpacken fehlgeschlagen: ") + $_.Exception.Message) -ForegroundColor Red
    return ""
  }
}

# --- 3. ADB/fastboot: existing path or auto-download Google platform-tools ---
$adb = Find-Bin @("adb.exe") @("C:\Program Files (x86)\Minimal ADB and Fastboot\adb.exe", "C:\Program Files\Minimal ADB and Fastboot\adb.exe", (Join-Path $InstallDir "platform-tools\adb.exe"))
$fb = ""
if ($adb -ne "") { $fb = Join-Path (Split-Path -Parent $adb) "fastboot.exe"; if (-not (Test-Path $fb)) { $fb = "" } }
if ($adb -ne "" -and $fb -ne "") {
  Write-Host ("ADB/fastboot: $adb") -ForegroundColor Green
} else {
  Write-Host (L "No ADB/fastboot found." "Kein ADB/fastboot gefunden.") -ForegroundColor Yellow
  Write-Host (L "[1] Auto-download Google platform-tools (official, portable)  [2] Enter own path (Minimal ADB)" "[1] Google platform-tools laden (offiziell, portable)  [2] Eigenen Pfad angeben (Minimal ADB)") -ForegroundColor Cyan
  $ch = Read-Host (L "Choice (Enter=1)" "Auswahl (Enter=1)")
  if ($ch -eq "2") {
    $p = Read-Host (L "Folder with adb.exe" "Ordner mit adb.exe")
    $adb = Join-Path $p "adb.exe"; $fb = Join-Path $p "fastboot.exe"
    if (-not ((Test-Path $adb) -and (Test-Path $fb))) {
      Write-Host (L "Not found there, aborting tool setup (config still saved)." "Dort nicht gefunden, Tool-Setup abgebrochen (Config trotzdem gespeichert).") -ForegroundColor Red
      $adb = ""; $fb = ""
    }
  } else {
    $d = Install-ZipTool "https://dl.google.com/android/repository/platform-tools-latest-windows.zip" (Join-Path $InstallDir "platform-tools-pkg") "platform-tools"
    if ($d -ne "") {
      # Zip contains platform-tools/ subfolder -> normalize
      $inner = Join-Path $d "platform-tools"
      if (Test-Path $inner) {
        $final = Join-Path $InstallDir "platform-tools"
        if (Test-Path $final) { Remove-Item $final -Recurse -Force -ErrorAction SilentlyContinue }
        Move-Item $inner $final -Force
        Remove-Item $d -Recurse -Force -ErrorAction SilentlyContinue
      } else {
        $final = $d
      }
      $adb = Join-Path $final "adb.exe"; $fb = Join-Path $final "fastboot.exe"
      if ((Test-Path $adb) -and (Test-Path $fb)) { Add-ToUserPath $final }
      else { $adb = ""; $fb = "" }
    }
  }
}

# --- 4. scrcpy: only with explicit consent ---
$scr = Find-Bin @("scrcpy.exe") @((Join-Path $InstallDir "scrcpy\scrcpy.exe"))
if ($scr -ne "") {
  Write-Host ("scrcpy: $scr") -ForegroundColor Green
} else {
  Write-Host (L "scrcpy not found (optional screen mirror). Install now? [y/N]: " "scrcpy fehlt (optional). Jetzt installieren? [j/N]: ") -NoNewline -ForegroundColor Yellow
  $yn = Read-Host
  if ($yn -eq "y" -or $yn -eq "Y" -or $yn -eq "j" -or $yn -eq "J") {
    try {
      [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
      $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/Genymobile/scrcpy/releases/latest" -UseBasicParsing
      $asset = $rel.assets | Where-Object { $_.name -like "*win64*.zip" } | Select-Object -First 1
      if ($asset -ne $null) {
        $d = Install-ZipTool $asset.browser_download_url (Join-Path $InstallDir "scrcpy-pkg") "scrcpy"
        if ($d -ne "") {
          $exe = Get-ChildItem -Path $d -Recurse -Filter "scrcpy.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
          if ($exe -ne $null) {
            $final = Join-Path $InstallDir "scrcpy"
            if (Test-Path $final) { Remove-Item $final -Recurse -Force -ErrorAction SilentlyContinue }
            New-Item -ItemType Directory -Path $final -Force | Out-Null
            Copy-Item (Join-Path $exe.DirectoryName "*") $final -Recurse -Force
            Remove-Item $d -Recurse -Force -ErrorAction SilentlyContinue
            $scr = Join-Path $final "scrcpy.exe"
            Add-ToUserPath $final
          }
        }
      } else { Write-Host (L "No win64 asset found in latest release." "Kein win64-Asset im latest Release.") -ForegroundColor Red }
    } catch {
      Write-Host ((L "scrcpy install failed: " "scrcpy-Install fehlgeschlagen: ") + $_.Exception.Message) -ForegroundColor Red
    }
  }
}

# --- 5. Save config ---
@{ adb = $adb; fastboot = $fb; scrcpy = $scr; installDir = $InstallDir; updated = (Get-Date -Format "yyyy-MM-dd HH:mm:ss") } |
  ConvertTo-Json -Depth 2 | Out-File -FilePath $CfgFile -Encoding utf8
Write-Host ((L "Config saved: " "Config gespeichert: ") + $CfgFile) -ForegroundColor Green
Write-Host (L "Done. Start with Start-TrebleToolkit.bat (new terminal picks up PATH)." "Fertig. Start mit Start-TrebleToolkit.bat (neues Terminal uebernimmt PATH).") -ForegroundColor Cyan
