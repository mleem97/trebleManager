<#
.SYNOPSIS
  Huawei P10 (VTR-L29 u.a.) TrebleDroid / Magisk Root-Manager - PowerShell TUI + CLI
  OS-unabhaengig: Stock-EMUI, TrebleDroid/Lineage-GSI, AOSP/PE-GSI, Custom-ROMs.

.DESCRIPTION
  Technische Grundlage:
   - https://github.com/phhusson/treble_experimentations/discussions/2542
   - https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus
  Design-Entscheidung (verifiziert, nicht geraten):
   - Zielpartition Huawei P10 = recovery_ramdisk (NICHT boot, NICHT recovery)
   - Quelldatei = RECOVERY_RAMDIS(K).img aus UPDATE.APP der passenden EMUI-9.1-Full-Firmware
   - Beispiel (NICHT exklusiv): VTR-L29 9.1.0.297(C432E5R1P9)
   - Magisk-Weg: Stock-Image -> Magisk "Select and Patch a File" -> fastboot flash recovery_ramdisk
   - Boot: Vol-Up + Power bis Huawei-Logo (Magisk boot cheat, nicht persistent)
   - Huawei-Fastboot antwortet oft FAILED (remote: Command not allowed) -> KEIN Lock-Beweis
   - GSI (system/vendor) bleibt erhalten, nur recovery_ramdisk wird angefasst
   - NIEMALS automatisch: erase/format userdata, flashing unlock, Bootloader-Unlock

  OS-Unterstuetzung (jedes OS, Custom oder nicht):
   - Stock EMUI 8.0 / 9.0 / 9.1, Harmony-Fake-9.1-Basis
   - TrebleDroid / Lineage-GSI (bgN/bvS/bgS...), Android 10-14
   - PixelExperience / SuperiorOS / AOSP-GSI, sonstige Custom-ROMs
   - Erkennung via getprop-Klassifizierung, kein Raten. GSI versteckt Huawei-Basis ->
     Firmware-Baseline wird assistiert ermittelt (User-Angabe + fastboot product + CUST).

.PARAMETER Command
  CLI-Modus: detect|analyze|firmware|extract|patch|backup|flash|verify|restore|diagnostic|wizard|help
  Ohne Command startet die TUI.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File Treble-Toolkit.ps1
  powershell -ExecutionPolicy Bypass -File Treble-Toolkit.ps1 detect --json
  powershell -ExecutionPolicy Bypass -File Treble-Toolkit.ps1 diagnostic
#>
param(
  [string]$Command = "",
  [string]$Image = "",
  [string]$FirmwareFile = "",
  [switch]$Json,
  [switch]$Yes,
  [switch]$NoReboot,
  [switch]$Anonymize,
  [switch]$NoElevateCheck,
  [switch]$Help
)

$ErrorActionPreference = "Continue"
$TTVersion = "2.0.0"

# Spec-Fehlerfaelle (explizit behandelt, SUCHBAR):
# ADB not found / No device detected / USB debugging authorization required (ADB unauthorized) /
# Fastboot not found / Command not allowed (Huawei getvar verweigert, NICHT auto locked) /
# Unsupported partition layout (recovery_ramdisk fehlt) / Unsupported model (falsches Geraet) /
# Firmware mismatch (falsche Firmware) / Cannot safely flash image (Image nicht verifizierbar) /
# Patch failed (Magisk-Patching fehlgeschlagen) / Flash failed (Fastboot Flash fehlgeschlagen) /
# Boot verification failed (Android bootet nicht -> Restore anbieten).

# ============================================================ Pfade / State
function Get-TTToolRoot {
  $d = Split-Path -Parent $MyInvocation.MyCommand.Path
  if ([string]::IsNullOrEmpty($d)) { $d = (Get-Location).Path }
  if ((Split-Path -Leaf $d) -eq "scripts") { return (Split-Path -Parent $d) }
  return $d
}
$TTRoot    = Get-TTToolRoot
$TTLogDir  = Join-Path $TTRoot "logs"
$TTData    = Join-Path $TTRoot "data"
$TTFirmDir = Join-Path $TTData "firmware"
$TTMagDir  = Join-Path $TTData "magisk"
$TTToolDir = Join-Path $TTData "tools"
$TTBackDir = Join-Path $TTRoot "backups"
foreach ($p in @($TTLogDir,$TTFirmDir,$TTMagDir,$TTToolDir,$TTBackDir)) {
  if (-not (Test-Path $p)) { New-Item -ItemType Directory -Path $p -Force | Out-Null }
}
$TTStamp = Get-Date -Format "yyyyMMdd-HHmmss"
$TTLog   = Join-Path $TTLogDir ("toolkit-" + $TTStamp + ".log")

$TT = @{
  Version   = $TTVersion
  Root      = $TTRoot
  Log       = $TTLog
  Adb       = $null
  Fastboot  = $null
  Mode      = "none"      # android | fastboot | none
  AdbSerial = ""
  FbSerial  = ""
  Props     = @{}
  ByName    = @()
  ByNameRaw = ""
  FbVars    = @{}
  FbRaw     = ""
  OS        = $null
  ProfileId = ""
  FirmwareBaseline = ""
  FirmwareCompat   = $null
  StockImage  = ""
  StockHash   = $null
  PatchedImage = ""
  PatchedHash  = $null
  MagiskApk    = ""
  MagiskInfo   = $null
  BackupDir    = ""
}

# ============================================================ Logging
function Write-TTLog {
  param([string]$Message = "", [string]$Level = "INFO")
  $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
  $line = "$ts $Level $Message"
  if ($Level -eq "ERROR") { Write-Host $line -ForegroundColor Red }
  elseif ($Level -eq "WARNING") { Write-Host $line -ForegroundColor Yellow }
  elseif ($Level -eq "SUCCESS") { Write-Host $line -ForegroundColor Green }
  elseif ($Level -eq "DEBUG") { Write-Host $line -ForegroundColor DarkGray }
  else { Write-Host $line -ForegroundColor Gray }
  try { $line | Out-File -FilePath $TT.Log -Encoding utf8 -Append } catch {}
}
"=== TrebleToolkit v$TTVersion gestartet (PS $($PSVersionTable.PSVersion)) ===" | Out-File -FilePath $TT.Log -Encoding utf8 -Append

function Test-TTAdmin {
  try {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $p = New-Object Security.Principal.WindowsPrincipal($id)
    return $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  } catch { return $false }
}

# ============================================================ Geraete-Profile (modular)
# Neues Huawei-Geraet = hier einen Block ergaenzen, Rest bleibt gleich.
$DeviceProfiles = @{
  "VTR-L29" = @{
    Id = "VTR-L29"; Marketing = "Huawei P10"; Arch = "arm64"; SoC = "Kirin 960"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    KnownGoodAdvisory = @(
      "VTR-L29 9.1.0.297(C432E5R1P9)",
      "VTR-L29 9.1.0.275(C432E2R1P9T8)"
    )
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power bis Huawei-Logo, dann loslassen (Magisk boot cheat, nicht persistent)"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV (USER LOCK + BL LOCK, fastboot oem unlock <code>)"
    FirmwareFinder = "https://professorjtj.github.io/v2/ (HUAWEI FIRM FINDER V2)"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VTR-L09" = @{
    Id = "VTR-L09"; Marketing = "Huawei P10"; Arch = "arm64"; SoC = "Kirin 960"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    KnownGoodAdvisory = @("VTR-L09 EMUI 9.1 mit passender CUST (C432/C185/...) - Region muss zum Geraet passen")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power bis Huawei-Logo, dann loslassen"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VKY-L29" = @{
    Id = "VKY-L29"; Marketing = "Huawei P10 Plus"; Arch = "arm64"; SoC = "Kirin 960"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    KnownGoodAdvisory = @("VKY-L29 EMUI 9.1 mit passender CUST")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power bis Huawei-Logo, dann loslassen"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
}

$MagiskCompatTable = @(
  @{ Android = "8.x (EMUI 8)"; Tested = "Magisk v25+"; Note = "Wiki: v25+ auf EMUI8/9-Basis ok. Changelog gegenpruefen." }
  @{ Android = "9.x (EMUI 9/9.1)"; Tested = "Magisk v25+"; Note = "Wiki explizit. Kein blindes Latest." }
  @{ Android = "10-12 (GSI/Custom)"; Tested = "Magisk v26+"; Note = "Kompatibilitaet pro Release pruefen (topjohnwu Releases)." }
  @{ Android = "13-14 (TrebleDroid/Lineage-GSI)"; Tested = "Magisk v27/v28+ nach Changelog"; Note = "Aktuell: lineage_arm64_bgN A13 laeuft; Magisk-Version + SHA-256 dokumentieren." }
)

# ============================================================ Firmware-Quellen (kuratiert, ehrlich)
# Kein direkter Stock-ZIP ist 2026 garantiert offiziell verfuegbar. Deshalb: Quellen anzeigen,
# User bestaetigt Download explizit, danach Hash/Groesse pruefen. Kein Dubios-Auto-Download.
$FirmwareSources = @(
  @{ Id = "hisuite"; Name = "Huawei HiSuite (offiziell, empfohlen f. Re-Install)"; Url = "https://consumer.huawei.com/de/support/hisuite/"; Kind = "official"; Note = "Kein Direkt-ZIP. Stock per USB zurueckspielen oder UPDATE.APP aus HiSuite-Cache sichern." }
  @{ Id = "consumer"; Name = "Huawei Consumer Support (offizielle Suche)"; Url = "https://consumer.huawei.com/de/support/"; Kind = "official"; Note = "Modell VTR-L29 manuell suchen. Alte EMUI-9.1-Pakete teils entfernt." }
  @{ Id = "firmfinder"; Name = "HUAWEI FIRM FINDER V2 (historischer Proxy)"; Url = "https://professorjtj.github.io/v2/"; Kind = "proxy-historic"; Note = "Frueher Huawei-Cloud-Proxy. Status pruefen, evtl. offline. Nur als Recherche." }
  @{ Id = "androidhost"; Name = "androidhost.ru Huawei-Archiv (Community, XDA-referenziert)"; Url = "https://androidhost.ru/search.html?search=VTR-L29"; Kind = "community-archive"; Note = "Etabliertes Community-Archiv f. Full-OTA (UPDATE.APP). Direktlink dort waehlen, hier einfuegen, Hash pruefen." }
  @{ Id = "wiki"; Name = "phhusson Wiki/Discussion (Doku, kein Download)"; Url = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"; Kind = "docs"; Note = "Referenz f. Methode/Partition. Beispiel-Firmware VTR-L29 9.1.0.297(C432E5R1P9) ist NICHT exklusiv." }
  @{ Id = "custom"; Name = "Eigene URL einfuegen (Full-Firmware ZIP)"; Url = ""; Kind = "user"; Note = "Nur http(s), nur eigene gepruefte Quelle. Wird validiert + gehasht." }
)

function Test-FirmwareUrl {
  # Rein, unit-testfaehig. Erlaubt: http/https, Dateityp ZIP/7Z/TAR/GZ/APP, max 2048 Zeichen.
  param([string]$Url)
  if ([string]::IsNullOrWhiteSpace($Url)) { return @{ Ok = $false; Reason = "URL leer." } }
  $u = $Url.Trim()
  if ($u.Length -gt 2048) { return @{ Ok = $false; Reason = "URL zu lang." } }
  if (-not ($u -match "^https?://")) { return @{ Ok = $false; Reason = "Nur http(s) erlaubt (kein ftp/file/javascript)." } }
  try { $uri = New-Object System.Uri($u) } catch { return @{ Ok = $false; Reason = "URL ungueltig." } }
  if ([string]::IsNullOrEmpty($uri.Host)) { return @{ Ok = $false; Reason = "Host fehlt." } }
  if ($u -match "@" -and $u -match "://[^/]*:.*@") { return @{ Ok = $false; Reason = "Keine Credentials in URL." } }
  $low = $u.ToLower().Split("?")[0]
  $allowed = @(".zip",".7z",".tar",".gz",".tgz",".app",".rar")
  $hit = $false
  foreach ($e in $allowed) { if ($low.EndsWith($e)) { $hit = $true } }
  if (-not $hit) { return @{ Ok = $false; Reason = "Dateityp muss Firmware-Archiv sein (zip/7z/tar/gz/app/rar)." } }
  return @{ Ok = $true; Reason = "OK"; Host = $uri.Host }
}

function Invoke-FirmwareDownload {
  # BITS (mit Fortschritt) bevorzugt, WebClient+Write-Progress als Fallback. Read-only bis auf Zieldatei.
  param([string]$Url, [string]$OutFile)
  $chk = Test-FirmwareUrl $Url
  if (-not $chk.Ok) { Write-TTLog ("URL abgelehnt: " + $chk.Reason) "ERROR"; return @{ Ok = $false; Path = "" } }
  $dir = Split-Path -Parent $OutFile
  if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
  if (Test-Path $OutFile) {
    Write-TTLog ("Zieldatei existiert bereits (Offline-Cache, kein Re-Download): " + $OutFile) "WARNING"
    return @{ Ok = $true; Path = $OutFile; Cached = $true }
  }
  Write-TTLog ("Download startet: " + $Url) "INFO"
  Write-TTLog ("Ziel: " + $OutFile) "INFO"
  # --- Versuch 1: BITS (Windows, Resume + Fortschritt) ---
  try {
    $bits = Get-Command Start-BitsTransfer -ErrorAction SilentlyContinue
    if ($bits) {
      Write-Host "Download via BITS (Fortschritt im BITS-Manager + unten) ..." -ForegroundColor Cyan
      Start-BitsTransfer -Source $Url -Destination $OutFile -DisplayName "TrebleToolkit Firmware" -Description $Url -ErrorAction Stop | Out-Null
      if (Test-Path $OutFile) {
        $mb = [math]::Round((Get-Item $OutFile).Length/1MB,1)
        Write-Progress -Activity "Firmware-Download" -Completed
        Write-TTLog ("BITS-Download fertig (" + $mb + " MB).") "SUCCESS"
        return @{ Ok = $true; Path = $OutFile }
      }
    }
  } catch { Write-TTLog ("BITS fehlgeschlagen, Fallback WebClient: " + $_.Exception.Message) "WARNING" }
  # --- Versuch 2: WebClient mit visuellem Fortschritt ---
  try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $wc = New-Object System.Net.WebClient
    $state = @{ Done = $false; Total = 0 }
    $progId = Get-Random -Minimum 1000 -Maximum 9999
    Register-ObjectEvent -InputObject $wc -EventName DownloadProgressChanged -SourceIdentifier ("TTDL" + $progId) -Action {
      $p = $Event.SourceEventArgs.ProgressPercentage
      $rec = $Event.SourceEventArgs.BytesReceived
      $tot = $Event.SourceEventArgs.TotalBytesToReceive
      $mbR = [math]::Round($rec/1MB,1); $mbT = [math]::Round($tot/1MB,1)
      Write-Progress -Activity "Firmware-Download (Stock, original)" -Status "$mbR / $mbT MB ($p %)" -PercentComplete $p
    } | Out-Null
    Register-ObjectEvent -InputObject $wc -EventName DownloadFileCompleted -SourceIdentifier ("TTDONE" + $progId) -Action {
      Write-Progress -Activity "Firmware-Download" -Completed
    } | Out-Null
    Write-Host "Lade ... (Balken + MB-Anzeige, Abbruch: Strg+C)" -ForegroundColor Cyan
    $task = $wc.DownloadFileTaskAsync($Url, $OutFile)
    while (-not $task.IsCompleted) { Start-Sleep -Milliseconds 300 }
    Unregister-Event -SourceIdentifier ("TTDL" + $progId) -ErrorAction SilentlyContinue
    Unregister-Event -SourceIdentifier ("TTDONE" + $progId) -ErrorAction SilentlyContinue
    $wc.Dispose()
    if ($task.IsFaulted) { throw $task.Exception }
    if (Test-Path $OutFile) {
      $mb = [math]::Round((Get-Item $OutFile).Length/1MB,1)
      Write-TTLog ("Download fertig (" + $mb + " MB).") "SUCCESS"
      return @{ Ok = $true; Path = $OutFile }
    }
  } catch {
    Write-Progress -Activity "Firmware-Download" -Completed
    Write-TTLog ("Download fehlgeschlagen: " + $_.Exception.Message) "ERROR"
    if (Test-Path $OutFile) { Remove-Item $OutFile -Force -ErrorAction SilentlyContinue }
    return @{ Ok = $false; Path = "" }
  }
  return @{ Ok = $false; Path = "" }
}

function Test-DownloadedFirmware {
  param([string]$Path)
  if (-not (Test-Path $Path)) { return @{ Ok = $false; Note = "Datei fehlt." } }
  $info = Get-FileHashInfo $Path
  $mb = [math]::Round($info.Size/1MB,1)
  $notes = @("Groesse: $mb MB")
  $ok = $true
  if ($info.Size -lt 100MB) { $ok = $false; $notes += "WARN: < 100 MB - unplausibel klein fuer Full-Firmware (evtl. nur OTA-Delta/verkuerzt)." }
  $shaFile = $Path + ".sha256"
  $info.SHA256 | Out-File -FilePath $shaFile -Encoding ascii
  $notes += "SHA-256 in $shaFile gesichert."
  # ZIP-Inhalt auf UPDATE.APP pruefen (wenn ZIP)
  if ($Path.ToLower().EndsWith(".zip")) {
    try {
      Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue | Out-Null
      $zip = [System.IO.Compression.ZipFile]::OpenRead($Path)
      $names = @()
      foreach ($e in $zip.Entries) { $names += $e.FullName }
      $zip.Dispose()
      $hasApp = ($names | Where-Object { $_ -like "*UPDATE.APP" }).Count -gt 0
      if ($hasApp) { $notes += "UPDATE.APP im ZIP enthalten." }
      else { $notes += "WARN: keine UPDATE.APP im ZIP gefunden - evtl. falsches Paket/Region."; $ok = $false }
    } catch { $notes += "ZIP-Check nicht moeglich: $($_.Exception.Message)" }
  } else { $notes += "Kein ZIP - Entpacken/UPDATE.APP-Suche manuell, dann TUI Step 4." }
  return @{ Ok = $ok; SHA256 = $info.SHA256; Size = $info.Size; Notes = $notes }
}

# ============================================================ Reine Parser (unit-testfaehig)
function ConvertFrom-AdbDevices {
  param([string[]]$Lines)
  $out = @()
  foreach ($l in $Lines) {
    $t = $l.Trim()
    if ($t -eq "" -or $t.StartsWith("List of")) { continue }
    $parts = ($t -split "\s+")
    if ($parts.Count -ge 2) {
      $o = New-Object PSObject -Property @{ Serial = $parts[0]; State = $parts[1]; Raw = $t }
      $out += $o
    }
  }
  return $out
}

function ConvertFrom-FastbootDevices {
  param([string[]]$Lines)
  $out = @()
  foreach ($l in $Lines) {
    $t = $l.Trim()
    if ($t -eq "") { continue }
    $parts = ($t -split "\s+")
    if ($parts.Count -ge 2 -and $parts[1] -eq "fastboot") {
      $o = New-Object PSObject -Property @{ Serial = $parts[0]; State = "fastboot"; Raw = $t }
      $out += $o
    }
  }
  return $out
}

function ConvertFrom-GetpropDump {
  param([string[]]$Lines)
  $h = @{}
  foreach ($l in $Lines) {
    $t = $l.Trim()
    if ($t -eq "") { continue }
    if ($t -match "^\[([^\]]+)\]:\s*\[([^\]]*)\]") { $h[$Matches[1]] = $Matches[2] }
    elseif ($t -match "^([^=:]+)\s*=\s*(.*)$") { $h[$Matches[1].Trim()] = $Matches[2].Trim() }
  }
  return $h
}

function ConvertFrom-ByNameListing {
  param([string]$Raw)
  $names = @()
  if ([string]::IsNullOrEmpty($Raw)) { return $names }
  $lines = $Raw -split "`n"
  foreach ($l in $lines) {
    $t = $l.Trim()
    if ($t -eq "") { continue }
    # Form: recovery_ramdisk -> /dev/block/mmcblk0pXX  oder  lrwxrwxrwx ... recovery_ramdisk -> ...
    $m = [regex]::Match($t, "([A-Za-z0-9_\-]+)\s*->\s*(\S+)")
    if ($m.Success) {
      $o = New-Object PSObject -Property @{ Name = $m.Groups[1].Value; Target = $m.Groups[2].Value; Raw = $t }
      $names += $o
    } elseif ($t -match "by-name") { continue }
  }
  return $names
}

function ConvertFrom-FastbootGetvar {
  param([string[]]$Lines)
  # Huawei gibt oft: getvar:unlocked FAILED (remote: Command not allowed) / finished.
  $h = @{}
  $raw = ($Lines -join "`n")
  foreach ($l in $Lines) {
    $t = $l.Trim()
    if ($t -match "^(.*?):\s*(.*)$" -and $t -notmatch "^finished" -and $t -notmatch "^getvar") {
      $h[$Matches[1].Trim()] = $Matches[2].Trim()
    }
    if ($t -match "FAILED\s*\(remote:\s*(.*?)\)") {
      $h["_last_failed_reason"] = $Matches[1]
    }
  }
  $denied = $false
  if ($raw -match "Command not allowed") { $denied = $true }
  return @{ Vars = $h; Raw = $raw; CommandDenied = $denied }
}

function Get-OSClassification {
  param([hashtable]$Props)
  $model = ""; $pname = ""; $display = ""; $release = ""
  if ($Props.ContainsKey("ro.product.model")) { $model = [string]$Props["ro.product.model"] }
  if ($Props.ContainsKey("ro.product.name")) { $pname = [string]$Props["ro.product.name"] }
  if ($Props.ContainsKey("ro.build.display.id")) { $display = [string]$Props["ro.build.display.id"] }
  if ($Props.ContainsKey("ro.build.version.release")) { $release = [string]$Props["ro.build.version.release"] }
  $emui = ""
  foreach ($k in @("ro.build.version.emui","ro.emui.version","ro.huawei.build.display.id","ro.build.huawei.version")) {
    if ($Props.ContainsKey($k) -and -not [string]::IsNullOrWhiteSpace([string]$Props[$k])) { $emui = [string]$Props[$k]; break }
  }
  $blob = ("$model $pname $display").ToLower()
  $isGsi = $false
  foreach ($kw in @("lineage_","trebledroid","treble","phh","aosp","pixel","superior","havoc","crdroid","gsi","arm64_b")) {
    if ($blob.Contains($kw)) { $isGsi = $true; break }
  }
  $isStock = $false
  if ($emui -ne "" -or $display -match "EMUI" -or $display -match "VTR-.*C\d+") { $isStock = $true }
  if ($isGsi) { $isStock = $false }  # GSI ueberschreibt (versteckt Huawei-Basis)

  $kind = "Unknown"
  if ($blob.Contains("trebledroid")) { $kind = "TrebleDroid-GSI" }
  elseif ($blob.Contains("lineage_")) { $kind = "Lineage-GSI" }
  elseif ($blob.Contains("pixel")) { $kind = "PixelExperience-GSI" }
  elseif ($blob.Contains("superior")) { $kind = "SuperiorOS-GSI" }
  elseif ($isGsi) { $kind = "AOSP/GSI-Custom" }
  elseif ($isStock -and $emui -match "9\.1") { $kind = "Stock-EMUI-9.1" }
  elseif ($isStock -and $emui -match "9\.0") { $kind = "Stock-EMUI-9.0" }
  elseif ($isStock -and $emui -match "8\.") { $kind = "Stock-EMUI-8" }
  elseif ($isStock) { $kind = "Stock-EMUI/Harmony-Basis" }
  elseif ($blob.Contains("havoc") -or $blob.Contains("crdroid") -or $blob.Contains("arrow")) { $kind = "Custom-ROM (non-GSI)" }

  $variant = ""
  $m = [regex]::Match($blob, "arm64_([a-z0-9]+)")
  if ($m.Success) { $variant = $m.Groups[1].Value }

  $supported = $false
  if ($release -match "^(8|9|10|11|12|13|14)") { $supported = $true }

  $notes = @()
  if ($kind -like "*GSI*") { $notes += "GSI versteckt Huawei-Firmwarebasis -> Baseline assistiert ermitteln." }
  if ($kind -like "Stock*") { $notes += "Stock: EMUI-Version direkt auslesbar, Baseline verifizierbar." }
  if (-not $supported) { $notes += "Android-Release '$release' ungeprueft (8-14 supportet)." }

  return @{
    Kind = $kind; Detail = "$model / $pname / Android $release ($display)"
    AndroidRelease = $release; GsiVariant = $variant
    IsStock = $isStock; IsGsi = $isGsi; SupportedAndroid = $supported
    Emui = $emui; Notes = $notes
  }
}

function Test-FirmwareCompatibility {
  param([string]$Model, [string]$FirmwareString, [string]$Region)
  $reasons = @(); $status = "PASS"
  if ([string]::IsNullOrWhiteSpace($FirmwareString)) {
    return @{ Status = "FAIL"; Reasons = @("Keine Firmware-Angabe (Baseline fehlt).") }
  }
  $fw = $FirmwareString.ToUpper()
  $mo = $Model.ToUpper()
  # Modell-Familie: VTR vs VKY strikt
  if ($mo.StartsWith("VTR") -and -not ($fw.Contains("VTR"))) {
    $status = "FAIL"; $reasons += "Firmware enthaelt kein VTR (P10), Modell ist $Model."
  }
  if ($mo.StartsWith("VKY") -and -not ($fw.Contains("VKY"))) {
    $status = "FAIL"; $reasons += "Firmware enthaelt kein VKY (P10 Plus), Modell ist $Model."
  }
  # Submodell exakt (L29 vs L09): WARN statt FAIL, da oft kreuzkompatibel aber CUST beachten
  if ($mo -eq "VTR-L29" -and $fw.Contains("VTR-L09") -and -not $fw.Contains("VTR-L29")) {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "Submodell-Abweichung: Geraet L29, Firmware L09 -> CUST/Region doppelt pruefen."
  }
  if ($mo -eq "VTR-L09" -and $fw.Contains("VTR-L29") -and -not $fw.Contains("VTR-L09")) {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "Submodell-Abweichung: Geraet L09, Firmware L29 -> CUST/Region doppelt pruefen."
  }
  # Region Cxxx
  $m1 = [regex]::Match($fw, "\(C(\d+)")
  $fwCust = ""; if ($m1.Success) { $fwCust = "C" + $m1.Groups[1].Value }
  if ($Region -ne "" -and $fwCust -ne "" -and $Region.ToUpper() -ne $fwCust.ToUpper()) {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "Region: Geraet $Region vs Firmware $fwCust -> nur mit passender CUST flashen."
  }
  # EMUI 9.1 Pflicht fuer recovery_ramdisk-Methode
  if ($fw -match "9\.1\.0") { $reasons += "EMUI 9.1-Basis ok (recovery_ramdisk-Methode)." }
  elseif ($fw -match "9\.0") {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "EMUI 9.0 statt 9.1 -> Methode moeglich, aber Full-9.1-Firmware bevorzugen."
  }
  elseif ($fw -match "8\.") {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "EMUI 8-Basis -> andere Bootchain moeglich, Wiki/Kernel-Hinweise beachten."
  } else {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "EMUI-Version aus Firmware-String nicht erkennbar -> manuell verifizieren."
  }
  if ($reasons.Count -eq 0) { $reasons += "Basispruefung bestanden." }
  return @{ Status = $status; Reasons = $reasons; FwCust = $fwCust }
}

function Get-FileHashInfo {
  param([string]$Path)
  if (-not (Test-Path $Path)) { return $null }
  try {
    $h256 = (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
    $h512 = (Get-FileHash -Path $Path -Algorithm SHA512).Hash.ToLower()
    $len = (Get-Item $Path).Length
    return @{ SHA256 = $h256; SHA512 = $h512; Size = $len; Path = $Path }
  } catch { return $null }
}

function Test-RecoveryImageFile {
  param([string]$Path)
  $res = @{ Exists = $false; SizeOk = $false; Hash = $null; HeaderHex = ""; HeaderText = ""; Verdict = "FAIL"; Notes = @() }
  if (-not (Test-Path $Path)) { $res.Notes += "Datei fehlt: $Path"; return $res }
  $res.Exists = $true
  $info = Get-FileHashInfo $Path
  $res.Hash = $info
  if ($info.Size -ge 1MB -and $info.Size -le 100MB) { $res.SizeOk = $true }
  else { $res.Notes += "Groesse unplausibel: $($info.Size) Bytes (erwartet 1-100 MB)." }
  try {
    $fs = [System.IO.File]::OpenRead($Path)
    $buf = New-Object byte[] 16
    [void]$fs.Read($buf, 0, 16)
    $fs.Close()
    $hex = ($buf | ForEach-Object { $_.ToString("X2") }) -join " "
    $res.HeaderHex = $hex
    $ascii = ""
    foreach ($b in $buf) { if ($b -ge 32 -and $b -le 126) { $ascii += [char]$b } else { $ascii += "." } }
    $res.HeaderText = $ascii
    if ($ascii.StartsWith("ANDROID!")) { $res.Notes += "Header ANDROID! (Android bootimg) erkannt." }
    elseif ($hex.StartsWith("1F 8B")) { $res.Notes += "Header gzip (1F 8B) erkannt." }
    else { $res.Notes += "Header unspezifisch ($hex) -> kein Ausschluss, aber manuell gegen Stock vergleichen." }
  } catch { $res.Notes += "Header nicht lesbar: $($_.Exception.Message)" }
  if ($res.Exists -and $res.SizeOk) { $res.Verdict = "PASS" } else { $res.Verdict = "FAIL" }
  return $res
}

# ============================================================ Tool-Suche / Wrapper
function Find-TTTools {
  $adbCandidates = @()
  try { $c = Get-Command "adb.exe" -ErrorAction SilentlyContinue; if ($c) { $adbCandidates += $c.Source } } catch {}
  try { $c = Get-Command "adb" -ErrorAction SilentlyContinue; if ($c -and $c.Source -notin $adbCandidates) { $adbCandidates += $c.Source } } catch {}
  foreach ($p in @(
    "C:\Program Files (x86)\Minimal ADB and Fastboot\adb.exe",
    "C:\Program Files\Minimal ADB and Fastboot\adb.exe",
    (Join-Path $TTRoot "platform-tools\adb.exe"),
    (Join-Path $TTRoot "tools\adb.exe")
  )) { if (Test-Path $p) { $adbCandidates += $p } }
  foreach ($p in $adbCandidates) { if (Test-Path $p) { $TT.Adb = $p; break } }
  if (-not $TT.Adb -and $adbCandidates.Count -gt 0) { $TT.Adb = $adbCandidates[0] }

  $fbCandidates = @()
  try { $c = Get-Command "fastboot.exe" -ErrorAction SilentlyContinue; if ($c) { $fbCandidates += $c.Source } } catch {}
  try { $c = Get-Command "fastboot" -ErrorAction SilentlyContinue; if ($c -and $c.Source -notin $fbCandidates) { $fbCandidates += $c.Source } } catch {}
  foreach ($p in @(
    "C:\Program Files (x86)\Minimal ADB and Fastboot\fastboot.exe",
    "C:\Program Files\Minimal ADB and Fastboot\fastboot.exe",
    (Join-Path $TTRoot "platform-tools\fastboot.exe"),
    (Join-Path $TTRoot "tools\fastboot.exe")
  )) { if (Test-Path $p) { $fbCandidates += $p } }
  foreach ($p in $fbCandidates) { if (Test-Path $p) { $TT.Fastboot = $p; break } }
  if (-not $TT.Fastboot -and $fbCandidates.Count -gt 0) { $TT.Fastboot = $fbCandidates[0] }

  if ($TT.Adb) { Write-TTLog "ADB: $($TT.Adb)" "SUCCESS" } else { Write-TTLog "ADB nicht gefunden / ADB not found (Minimal ADB / platform-tools in PATH legen)." "ERROR" }
  if ($TT.Fastboot) { Write-TTLog "Fastboot: $($TT.Fastboot)" "SUCCESS" } else { Write-TTLog "Fastboot nicht gefunden / Fastboot not found." "WARNING" }
}

function Invoke-TTAdb {
  param([string[]]$Args)
  if (-not $TT.Adb) { Write-TTLog "ADB fehlt, Befehl uebersprungen: adb $($Args -join ' ')" "ERROR"; return @() }
  Write-TTLog ">> adb $($Args -join ' ')" "DEBUG"
  try {
    $o = & $TT.Adb @Args 2>&1
    $lines = @()
    foreach ($x in $o) { $lines += [string]$x }
    ($lines -join "`n") | Out-File -FilePath $TT.Log -Encoding utf8 -Append
    return $lines
  } catch { Write-TTLog "ADB-Fehler: $($_.Exception.Message)" "ERROR"; return @() }
}

function Invoke-TTFastboot {
  param([string[]]$Args)
  if (-not $TT.Fastboot) { Write-TTLog "Fastboot fehlt, Befehl uebersprungen." "ERROR"; return @() }
  Write-TTLog ">> fastboot $($Args -join ' ')" "DEBUG"
  try {
    $o = & $TT.Fastboot @Args 2>&1
    $lines = @()
    foreach ($x in $o) { $lines += [string]$x }
    ($lines -join "`n") | Out-File -FilePath $TT.Log -Encoding utf8 -Append
    return $lines
  } catch { Write-TTLog "Fastboot-Fehler: $($_.Exception.Message)" "ERROR"; return @() }
}

function Get-TTProp {
  param([string]$Name)
  $o = Invoke-TTAdb @("shell","getprop",$Name)
  return ((($o -join "") -replace "`r","") -replace "`n","").Trim()
}

# ============================================================ Modus / Analyse (OS-unabhaengig)
function Update-TTMode {
  $TT.Mode = "none"; $TT.AdbSerial = ""; $TT.FbSerial = ""
  if ($TT.Adb) {
    $o = Invoke-TTAdb @("devices")
    $devs = ConvertFrom-AdbDevices $o
    foreach ($d in $devs) { if ($d.State -eq "device") { $TT.Mode = "android"; $TT.AdbSerial = $d.Serial; break } }
    if ($TT.Mode -eq "none") {
      foreach ($d in $devs) {
        if ($d.State -eq "unauthorized") { Write-TTLog "ADB autorisierung ausstehend (Dialog am Geraet bestaetigen)." "WARNING" }
      }
    }
  }
  if ($TT.Mode -eq "none" -and $TT.Fastboot) {
    $o = Invoke-TTFastboot @("devices")
    $devs = ConvertFrom-FastbootDevices $o
    foreach ($d in $devs) { $TT.Mode = "fastboot"; $TT.FbSerial = $d.Serial; break }
  }
  return $TT.Mode
}

$PropListAndroid = @(
  "ro.product.model","ro.product.name","ro.product.device",
  "ro.build.version.release","ro.build.display.id","ro.build.type","ro.build.tags",
  "ro.build.version.sdk","ro.build.version.emui","ro.emui.version",
  "ro.product.cpu.abi","ro.hardware",
  "ro.treble.enabled","ro.vndk.version",
  "ro.boot.slot_suffix","ro.boot.verifiedbootstate","ro.boot.flash.locked",
  "ro.boot.vbmeta.device_state","ro.secure","ro.debuggable"
)

function Invoke-TTAndroidAnalysis {
  Write-TTLog "Android-Analyse (read-only, OS-unabhaengig) ..." "INFO"
  $h = @{}
  foreach ($p in $PropListAndroid) { $h[$p] = Get-TTProp $p }
  $TT.Props = $h
  $TT.OS = Get-OSClassification $h
  $byRaw = (Invoke-TTAdb @("shell","ls","-l","/dev/block/by-name/") -join "`n")
  $TT.ByNameRaw = $byRaw
  $TT.ByName = ConvertFrom-ByNameListing $byRaw
  $cmdline = (Invoke-TTAdb @("shell","cat","/proc/cmdline") -join "`n")
  $TT.Cmdline = $cmdline
  # Root-Spuren (OS-unabhaengig)
  $TT.WhichSu = (Invoke-TTAdb @("shell","which su; ls -l /system/xbin/su 2>&1; ls -l /system/bin/su 2>&1") -join "`n")
  $TT.MagiskVer = (Invoke-TTAdb @("shell","magisk -v 2>&1; su -v 2>&1; su -c id 2>&1") -join "`n")
  $TT.MagiskPkg = (Invoke-TTAdb @("shell","pm list packages 2>&1 | grep -i -E 'magisk|topjohnwu|phh|kernelsu'") -join "`n")
  # Profil aus Modell ableiten (GSI meldet TrebleDroid statt VTR -> Hinweis, Profil bleibt VTR-L29 default)
  $model = [string]$h["ro.product.model"]
  $prof = ""
  if ($model -match "VTR-L29") { $prof = "VTR-L29" }
  elseif ($model -match "VTR-L09") { $prof = "VTR-L09" }
  elseif ($model -match "VKY-L29") { $prof = "VKY-L29" }
  else {
    # GSI/Stock ohne Huawei-Modellstring -> Default VTR-L29 (Zielgeraet), aber als WARN markieren
    $prof = "VTR-L29"
    Write-TTLog "Modellstring ist GSI ('$model'), kein VTR/VKY. Profil-Default VTR-L29 (Zielgeraet), Verifikation vor Flash Pflicht." "WARNING"
  }
  $TT.ProfileId = $prof
  Write-TTLog "OS: $($TT.OS.Kind) | $($TT.OS.Detail)" "SUCCESS"
  Write-TTLog "Profil: $prof | Zielpartition: $($DeviceProfiles[$prof].TargetPartition)" "INFO"
  return $TT.OS
}

$FbVarList = @(
  "product","secure","unlocked","current-slot",
  "partition-type:recovery_ramdisk","partition-size:recovery_ramdisk",
  "partition-type:boot","partition-size:boot",
  "partition-type:recovery","partition-size:recovery",
  "partition-type:system","partition-size:system",
  "partition-type:vendor","partition-size:vendor"
)

function Invoke-TTFastbootAnalysis {
  Write-TTLog "Fastboot-Analyse (read-only) ..." "INFO"
  $all = @{}
  $rawAll = ""
  foreach ($v in $FbVarList) {
    $o = Invoke-TTFastboot @("getvar",$v)
    $p = ConvertFrom-FastbootGetvar $o
    $all[$v] = ($o -join " | ").Trim()
    $rawAll += "### getvar $v`n" + ($o -join "`n") + "`n"
  }
  $oAll = Invoke-TTFastboot @("getvar","all")
  $pAll = ConvertFrom-FastbootGetvar $oAll
  $rawAll += "### getvar all`n" + ($oAll -join "`n")
  $TT.FbVars = $all
  $TT.FbRaw = $rawAll
  if ($pAll.CommandDenied -or ($rawAll -match "Command not allowed")) {
    Write-TTLog "Huawei verweigert getvar (Command not allowed). Das ist KEIN Lock-Beweis, nur Huawei-Eigenheit." "WARNING"
  }
  return $all
}

# ============================================================ Firmware-Baseline (OS-abhaengig assistiert)
function Get-TTFirmwareBaseline {
  param([switch]$Interactive)
  $os = $TT.OS
  $base = ""
  if ($os -ne $null -and $os.IsStock) {
    $disp = [string]$TT.Props["ro.build.display.id"]
    $incr = Get-TTProp "ro.build.version.incremental"
    $base = ($disp + " " + $incr).Trim()
    Write-TTLog "Stock erkannt -> Baseline aus Build: $base" "SUCCESS"
  } else {
    Write-TTLog "GSI/Custom erkannt -> Huawei-Basis ist versteckt. Assistierte Baseline noetig." "WARNING"
    Write-TTLog "Fastboot 'product' (falls verfuegbar) + deine Original-Firmware (z.B. VTR-L29 9.1.0.xxx(Cxxx...))." "INFO"
    if ($TT.FbVars.ContainsKey("product")) { Write-TTLog "fastboot product: $($TT.FbVars['product'])" "INFO" }
  }
  if ($TT.FirmwareBaseline -ne "") { $base = $TT.FirmwareBaseline }
  if ($Interactive -and [string]::IsNullOrWhiteSpace($base)) {
    Write-Host ""
    Write-Host "Original Huawei-Firmware (Bsp: VTR-L29 9.1.0.297(C432E5R1P9)) eingeben." -ForegroundColor Yellow
    Write-Host "Bei GSI: die Firmware, auf deren Basis das GSI geflasht wurde bzw. die zum Geraet/Region passt." -ForegroundColor Gray
    Write-Host "Firmware-String (Enter = spaeter): " -NoNewline -ForegroundColor Yellow
    $in = Read-Host
    if (-not [string]::IsNullOrWhiteSpace($in)) { $base = $in.Trim() }
  }
  if ($base -ne "") { $TT.FirmwareBaseline = $base }
  return $base
}

function Find-TTRecoveryImage {
  $prof = $DeviceProfiles[$TT.ProfileId]
  if ($prof -eq $null) { $prof = $DeviceProfiles["VTR-L29"] }
  $found = @()
  foreach ($dir in @($TTFirmDir, (Join-Path $TTRoot "data"))) {
    if (-not (Test-Path $dir)) { continue }
    foreach ($n in $prof.StockFileNames) {
      $hits = Get-ChildItem -Path $dir -Recurse -File -Filter $n -ErrorAction SilentlyContinue
      foreach ($h in $hits) { $found += $h.FullName }
    }
    # Fallback: alles mit RECOVERY_RAMDIS im Namen (exakte Bezeichnung nicht raten/umbenennen)
    $wild = Get-ChildItem -Path $dir -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*RECOVERY*RAMDIS*" }
    foreach ($h in $wild) { if ($found -notcontains $h.FullName) { $found += $h.FullName } }
  }
  return $found
}

function Invoke-TTUpdateAppAnalysis {
  param([string]$UpdateAppPath)
  $res = @{ Exists = $false; Size = 0; HeaderHex = ""; Note = "" }
  if ([string]::IsNullOrWhiteSpace($UpdateAppPath) -or -not (Test-Path $UpdateAppPath)) {
    $res.Note = "UPDATE.APP nicht angegeben/gefunden. Lege Full-Firmware (UPDATE.APP) nach data/firmware/ und waehle sie im Menue."
    return $res
  }
  $res.Exists = $true
  $res.Size = (Get-Item $UpdateAppPath).Length
  try {
    $fs = [System.IO.File]::OpenRead($UpdateAppPath)
    $buf = New-Object byte[] 32
    [void]$fs.Read($buf, 0, 32)
    $fs.Close()
    $res.HeaderHex = (($buf | ForEach-Object { $_.ToString("X2") }) -join " ")
  } catch { $res.Note = "Header nicht lesbar: $($_.Exception.Message)" }
  $res.Note = "UPDATE.APP erkannt ($([math]::Round($res.Size/1MB,1)) MB). Extraktion: Huawei-Format beachten (z.B. huawei-update-extractor / splitupdate in data/tools/ ablegen oder manuell extrahieren). Danach RECOVERY_RAMDIS(K).img NICHT umbenennen, Originalnamen dokumentieren."
  return $res
}

# ============================================================ Magisk (kein Fake-Patching)
function Find-TTMagiskApk {
  $hits = Get-ChildItem -Path $TTMagDir -Filter "*.apk" -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
  if ($hits -and $hits.Count -gt 0) { return $hits[0].FullName }
  return ""
}

function Get-TTMagiskInfo {
  param([string]$ApkPath)
  if ([string]::IsNullOrWhiteSpace($ApkPath) -or -not (Test-Path $ApkPath)) { return $null }
  $info = Get-FileHashInfo $ApkPath
  $ver = "unbekannt (Dateiname pruefen)"
  $m = [regex]::Match([System.IO.Path]::GetFileName($ApkPath), "[Vv]?(\d+\.\d+[\.\d]*)")
  if ($m.Success) { $ver = $m.Groups[1].Value }
  return @{ Path = $ApkPath; Version = $ver; SHA256 = $info.SHA256; Size = $info.Size; Source = "offiziell: https://github.com/topjohnwu/Magisk/releases (manuell verifizieren)" }
}

function Prepare-TTMagiskPatch {
  param([string]$StockImage)
  $stage = Join-Path $TTMagDir "to-patch"
  if (-not (Test-Path $stage)) { New-Item -ItemType Directory -Path $stage -Force | Out-Null }
  $dest = Join-Path $stage ([System.IO.Path]::GetFileName($StockImage))
  Copy-Item -Path $StockImage -Destination $dest -Force
  $guide = Join-Path $stage "PATCH-ANLEITUNG.txt"
  @(
    "Magisk-Patch (echt, kein Kopieren = gepatcht behaupten)",
    "=======================================================",
    "",
    "1. Magisk-APK aus offizieller Quelle installieren:",
    "   https://github.com/topjohnwu/Magisk/releases",
    "   APK liegt idealerweise in: data/magisk/",
    "",
    "2. Diese Datei aufs Geraet kopieren:",
    "   $dest",
    "   z.B.: adb push `"$dest`" /sdcard/Download/",
    "",
    "3. Am Geraet (jedes OS, Stock wie GSI/Custom):",
    "   Magisk oeffnen -> Installieren -> 'Select and Patch a File'",
    "   -> RECOVERY_RAMDIS(K).img auswaehlen (exakt diese Datei, NICHT boot.img/recovery.img)",
    "",
    "4. Ergebnis am Geraet: /sdcard/Download/magisk_patched-*.img",
    "   Zurueck auf PC holen:",
    "   adb pull /sdcard/Download/magisk_patched-XXXX.img data/magisk/",
    "",
    "5. Hier im Tool 'Gepatchte Datei registrieren' waehlen und validieren lassen.",
    "   Validierung: Hash != Stock, Groesse plausibel, Header dokumentiert."
  ) | Out-File -FilePath $guide -Encoding utf8
  Write-TTLog "Patch-Vorbereitung: $dest" "SUCCESS"
  Write-TTLog "Anleitung: $guide" "INFO"
  return $dest
}

# ============================================================ Backup / Safety / Flash / Verify / Restore
function New-TTBackup {
  param([string]$StockImage)
  $model = $TT.ProfileId
  if ([string]::IsNullOrEmpty($model)) { $model = "VTR-L29" }
  $prof = $DeviceProfiles[$model]
  $part = $prof.TargetPartition
  $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $dir = Join-Path (Join-Path (Join-Path $TTBackDir $model) $part) $stamp
  New-Item -ItemType Directory -Path $dir -Force | Out-Null
  $ok = $false; $note = ""
  if ((Test-Path $StockImage)) {
    Copy-Item -Path $StockImage -Destination (Join-Path $dir "original.img") -Force
    $ok = $true; $note = "firmware-extracted"
  } else { $note = "kein Stock-Image vorhanden - Backup NICHT vorgetaeuscht" }
  # Versuch: direkte Partition lesen (ehrlich, scheitert ohne Root meist) - nur dokumentieren
  $ddOut = ""
  if ($TT.Mode -eq "android") {
    $link = $TT.ByName | Where-Object { $_.Name -eq $part } | Select-Object -First 1
    if ($link -ne $null) {
      $ddOut = (Invoke-TTAdb @("shell","dd if=$($link.Target) of=/sdcard/recovery_ramdisk_dump.img bs=4096 count=8192 2>&1; echo EXIT:$?") -join "`n")
      Write-TTLog "dd-Versuch ($part): $ddOut" "DEBUG"
    }
  }
  $hash = Get-FileHashInfo (Join-Path $dir "original.img")
  if ($hash -eq $null -and $ok) { $ok = $false }
  $meta = @{
    device = $prof.Marketing; model = $model; partition = $part
    firmware = $TT.FirmwareBaseline; build = [string]$TT.Props["ro.build.display.id"]
    os_kind = ""; size = 0; sha256 = ""; sha512 = ""
    timestamp = (Get-Date -Format "yyyy-MM-dd HH:mm:ss"); source = $note
    tool = "TrebleToolkit v$TTVersion"; target_partition = $part
  }
  if ($TT.OS -ne $null) { $meta.os_kind = $TT.OS.Kind }
  if ($hash -ne $null) { $meta.size = $hash.Size; $meta.sha256 = $hash.SHA256; $meta.sha512 = $hash.SHA512 }
  ($meta | ConvertTo-Json -Depth 4) | Out-File -FilePath (Join-Path $dir "metadata.json") -Encoding utf8
  if ($hash -ne $null) {
    $hash.SHA256 | Out-File -FilePath (Join-Path $dir "sha256.txt") -Encoding ascii
    $hash.SHA512 | Out-File -FilePath (Join-Path $dir "sha512.txt") -Encoding ascii
  }
  ("dd_probe:`n" + $ddOut) | Out-File -FilePath (Join-Path $dir "partition-probe.txt") -Encoding utf8
  if ($ok) { $TT.BackupDir = $dir; Write-TTLog "Backup: $dir" "SUCCESS" }
  else { Write-TTLog "Backup NICHT moeglich (kein Original). Es wird nichts vorgetaeuscht." "ERROR" }
  return @{ Ok = $ok; Dir = $dir }
}

function Test-TTFlashReadiness {
  $checks = @()
  $prof = $DeviceProfiles[$TT.ProfileId]
  if ($prof -eq $null) { $prof = $DeviceProfiles["VTR-L29"] }

  $c1 = ($TT.ProfileId -match "VTR|VKY")
  $checks += New-Object PSObject -Property @{ Name = "Model matches (VTR/VKY)"; Pass = [bool]$c1; Detail = "Profil: $($TT.ProfileId)" }

  $hasPart = $false; $partDetail = "nicht gefunden"
  foreach ($n in $TT.ByName) { if ($n.Name -eq $prof.TargetPartition) { $hasPart = $true; $partDetail = "$($n.Name) -> $($n.Target)"; break } }
  if (-not $hasPart -and $TT.FbVars.ContainsKey("partition-size:recovery_ramdisk")) {
    $v = [string]$TT.FbVars["partition-size:recovery_ramdisk"]
    if ($v -ne "" -and $v -notmatch "FAILED" -and $v -notmatch "not allowed") { $hasPart = $true; $partDetail = "fastboot: $v" }
  }
  if (-not $hasPart -and $TT.FbVars.ContainsKey("partition-type:recovery_ramdisk")) {
    $v = [string]$TT.FbVars["partition-type:recovery_ramdisk"]
    if ($v -ne "" -and $v -notmatch "FAILED" -and $v -notmatch "not allowed") { $hasPart = $true; $partDetail = "fastboot: $v" }
  }
  $checks += New-Object PSObject -Property @{ Name = "Partition exists ($($prof.TargetPartition))"; Pass = [bool]$hasPart; Detail = $partDetail }

  $imgOk = (Test-Path $TT.PatchedImage)
  $checks += New-Object PSObject -Property @{ Name = "Image exists (patched)"; Pass = [bool]$imgOk; Detail = [string]$TT.PatchedImage }

  $hashOk = ($TT.PatchedHash -ne $null -and -not [string]::IsNullOrEmpty($TT.PatchedHash.SHA256))
  $checks += New-Object PSObject -Property @{ Name = "Image hash known (SHA-256)"; Pass = [bool]$hashOk; Detail = $(if ($TT.PatchedHash) { $TT.PatchedHash.SHA256.Substring(0,16)+"..." } else { "(kein Hash)" }) }

  $sizeOk = ($TT.PatchedHash -ne $null -and $TT.PatchedHash.Size -ge 1MB -and $TT.PatchedHash.Size -le 100MB)
  $checks += New-Object PSObject -Property @{ Name = "Image size plausible (1-100 MB)"; Pass = [bool]$sizeOk; Detail = $(if ($TT.PatchedHash) { [string]$TT.PatchedHash.Size } else { "?" }) }

  $fwOk = ($TT.FirmwareCompat -ne $null -and $TT.FirmwareCompat.Status -ne "FAIL" -and -not [string]::IsNullOrWhiteSpace($TT.FirmwareBaseline))
  $checks += New-Object PSObject -Property @{ Name = "Firmware compatibility established"; Pass = [bool]$fwOk; Detail = "$($TT.FirmwareBaseline) [$($TT.FirmwareCompat.Status)]" }

  $bakOk = ($TT.BackupDir -ne "" -and (Test-Path (Join-Path $TT.BackupDir "original.img")))
  $checks += New-Object PSObject -Property @{ Name = "Backup available (original.img)"; Pass = [bool]$bakOk; Detail = [string]$TT.BackupDir }

  $fbOk = ($TT.Mode -eq "fastboot" -and $TT.Fastboot -ne $null)
  # Re-evaluate fastboot live (nicht blind auf Cache verlassen)
  if ($TT.Fastboot) {
    $o = Invoke-TTFastboot @("devices")
    $devs = ConvertFrom-FastbootDevices $o
    $fbOk = ($devs.Count -gt 0)
    if ($fbOk) { $TT.Mode = "fastboot"; $TT.FbSerial = $devs[0].Serial }
  }
  $checks += New-Object PSObject -Property @{ Name = "Fastboot device connected"; Pass = [bool]$fbOk; Detail = "Modus: $($TT.Mode)" }

  $diffOk = $true
  if ($TT.StockHash -ne $null -and $TT.PatchedHash -ne $null) {
    if ($TT.StockHash.SHA256 -eq $TT.PatchedHash.SHA256) { $diffOk = $false }
  }
  $checks += New-Object PSObject -Property @{ Name = "Patched != stock (kein Fake-Patch)"; Pass = [bool]$diffOk; Detail = "Hashes muessen sich unterscheiden" }

  $overall = $true
  foreach ($c in $checks) { if (-not $c.Pass) { $overall = $false } }
  return @{ Checks = $checks; Go = $overall }
}

function Invoke-TTSafeFlash {
  param([switch]$ForceYes)
  $prof = $DeviceProfiles[$TT.ProfileId]
  if ($prof -eq $null) { $prof = $DeviceProfiles["VTR-L29"] }
  $part = $prof.TargetPartition
  $readiness = Test-TTFlashReadiness
  Write-Host ""
  Write-Host "=== Sicherheitspruefung vor Flash ===" -ForegroundColor Cyan
  foreach ($c in $readiness.Checks) {
    if ($c.Pass) { Write-Host (" [OK]   " + $c.Name + " -- " + $c.Detail) -ForegroundColor Green }
    else { Write-Host (" [FAIL] " + $c.Name + " -- " + $c.Detail) -ForegroundColor Red }
  }
  if (-not $readiness.Go) {
    Write-Host ""
    Write-Host "DO NOT FLASH - mindestens eine Pruefung ist fehlgeschlagen." -ForegroundColor Red -BackgroundColor Black
    Write-TTLog "Flash blockiert (Safety-Gate)." "ERROR"
    return $false
  }
  Write-Host ""
  Write-Host "WARNING" -ForegroundColor Red
  Write-Host "You are about to modify:" -ForegroundColor Yellow
  Write-Host "  $part" -ForegroundColor White
  Write-Host ("Device: " + $prof.Marketing + " " + $TT.ProfileId) -ForegroundColor White
  Write-Host ("Image: " + $TT.PatchedImage) -ForegroundColor White
  Write-Host ("SHA-256: " + $TT.PatchedHash.SHA256) -ForegroundColor White
  Write-Host ("Original backup: " + $TT.BackupDir) -ForegroundColor White
  Write-Host "This operation modifies the boot chain." -ForegroundColor Yellow
  Write-Host "GSI (system/vendor) bleibt unangetastet. NIEMALS wird userdata geloescht." -ForegroundColor Gray
  Write-Host "Continue?" -ForegroundColor Yellow
  if (-not $ForceYes) {
    Write-Host "Zum Fortfahren 'FLASHEN' tippen (1/2): " -NoNewline -ForegroundColor Yellow
    $a = Read-Host
    if ($a -ne "FLASHEN") { Write-TTLog "Flash abgebrochen (1. Bestaetigung fehlt)." "WARNING"; return $false }
    Write-Host "Wirklich sicher? Nochmal 'JA' (2/2): " -NoNewline -ForegroundColor Yellow
    $b = Read-Host
    if ($b -ne "JA") { Write-TTLog "Flash abgebrochen (2. Bestaetigung fehlt)." "WARNING"; return $false }
  } else {
    Write-TTLog "CLI --yes: explizite Bestaetigung via Flag dokumentiert." "WARNING"
  }
  # Abgeleiteter Befehl aus Profil (NICHT hart auf andere Partitionen)
  Write-TTLog "Starte: fastboot flash $part <patched>" "WARNING"
  $o = Invoke-TTFastboot @("flash",$part,$TT.PatchedImage)
  Write-Host ($o -join "`n") -ForegroundColor White
  $ok = ($o -join "`n") -match "OKAY|finished|Writing"
  if ($ok) { Write-TTLog "Flash gemeldet: OK (Ausgabe pruefen)." "SUCCESS" } else { Write-TTLog "Flash-Ausgabe unklar/fehlerhaft - Ausgabe oben pruefen." "ERROR" }
  return [bool]$ok
}

function Invoke-TTRootVerification {
  param([switch]$NoReboot)
  if (-not $NoReboot -and $TT.Mode -eq "fastboot" -and $TT.Fastboot) {
    Write-TTLog "fastboot reboot ..." "INFO"
    Invoke-TTFastboot @("reboot") | Out-Null
  }
  Write-Host ""
  Write-Host "Huawei Boot-Prozedur (Pflicht, sonst kein Root):" -ForegroundColor Cyan
  Write-Host "  Vol-Up + Power bis Huawei-Logo, dann loslassen (Magisk boot cheat)." -ForegroundColor White
  Write-Host "  Ohne Trick bootet Stock (ohne Root). Ggf. Magisk-App oeffnen und Root freigeben." -ForegroundColor Gray
  Write-Host ""
  if ($TT.Adb) {
    Write-TTLog "Warte auf adb (wait-for-device, bis 120s, erster Boot dauert) ..." "INFO"
    $o = Invoke-TTAdb @("wait-for-device")
    Start-Sleep -Seconds 5
    Update-TTMode | Out-Null
  }
  $res = @{ Root = "UNKNOWN"; Details = @() }
  $w = (Invoke-TTAdb @("shell","which su") -join "`n").Trim()
  $res.Details += "which su: $w"
  $id = (Invoke-TTAdb @("shell","su -c id 2>&1") -join "`n").Trim()
  $res.Details += "su -c id: $id"
  $mv = (Invoke-TTAdb @("shell","magisk -v 2>&1") -join "`n").Trim()
  $res.Details += "magisk -v: $mv"
  $pkg = (Invoke-TTAdb @("shell","pm list packages 2>&1 | grep -i -E 'magisk|topjohnwu'") -join "`n").Trim()
  $res.Details += "magisk pkg: $pkg"
  foreach ($d in $res.Details) { Write-Host $d -ForegroundColor White; Write-TTLog $d "INFO" }
  if ($id -match "uid=0") { $res.Root = "ROOTED"; Write-TTLog "ROOT DETECTED (uid=0)." "SUCCESS" }
  elseif ($w -ne "" -and $w -notmatch "not found" -and $w -notmatch "No such") {
    $res.Root = "INCONCLUSIVE"; Write-TTLog "su vorhanden, aber kein uid=0 (Freigabe/Magisk-App/first-boot?). INCONCLUSIVE." "WARNING"
  } else {
    $res.Root = "NOT_ROOTED"; Write-TTLog "Kein Root nachweisbar. Boot-Trick wiederholen + Magisk-App pruefen." "WARNING"
  }
  Write-TTLog "Hinweis: Boot allein != Root. Nur uid=0 zaehlt." "INFO"
  return $res
}

function Invoke-TTRestoreFlow {
  param([string]$BackupPick = "", [switch]$ForceYes)
  $cands = Get-ChildItem -Path (Join-Path (Join-Path $TTBackDir $TT.ProfileId) $DeviceProfiles[$TT.ProfileId].TargetPartition) -Directory -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
  if (-not $cands -or $cands.Count -eq 0) {
    # Fallback: alle Backups
    $cands = Get-ChildItem -Path $TTBackDir -Recurse -Directory -ErrorAction SilentlyContinue | Where-Object { Test-Path (Join-Path $_.FullName "original.img") } | Sort-Object LastWriteTime -Descending
  }
  if (-not $cands -or $cands.Count -eq 0) { Write-TTLog "Kein Backup mit original.img gefunden." "ERROR"; return $false }
  $pick = $cands[0].FullName
  if ($BackupPick -ne "" -and (Test-Path $BackupPick)) { $pick = $BackupPick }
  Write-TTLog "Restore-Kandidat: $pick" "INFO"
  $orig = Join-Path $pick "original.img"
  $metaF = Join-Path $pick "metadata.json"
  try {
    $meta = Get-Content $metaF -Raw | ConvertFrom-Json
    Write-Host ("Backup: " + $meta.model + " / " + $meta.partition + " / " + $meta.firmware) -ForegroundColor White
  } catch { Write-TTLog "metadata.json nicht lesbar, fahre mit Hash-Pruefung fort." "WARNING" }
  $h = Get-FileHashInfo $orig
  if ($h -eq $null) { Write-TTLog "original.img unlesbar." "ERROR"; return $false }
  Write-TTLog ("Restore-Hash SHA-256: " + $h.SHA256) "INFO"
  $part = $DeviceProfiles[$TT.ProfileId].TargetPartition
  Write-Host ""
  Write-Host "Restore: fastboot flash $part original.img" -ForegroundColor Yellow
  if (-not $ForceYes) {
    Write-Host "Zum Fortfahren 'RESTORE' + 'JA' tippen." -ForegroundColor Yellow
    $a = Read-Host "1/2 (RESTORE)"
    if ($a -ne "RESTORE") { return $false }
    $b = Read-Host "2/2 (JA)"
    if ($b -ne "JA") { return $false }
  }
  $o = Invoke-TTFastboot @("flash",$part,$orig)
  Write-Host ($o -join "`n") -ForegroundColor White
  Write-TTLog "Restore ausgefuehrt, Ausgabe oben." "WARNING"
  return $true
}

# ============================================================ Diagnose-ZIP
function New-TTDiagnostic {
  param([switch]$Anonymize)
  $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("P10Diag-" + $stamp)
  New-Item -ItemType Directory -Path $tmp -Force | Out-Null
  $ser = $TT.AdbSerial
  if ($ser -eq "") { $ser = $TT.FbSerial }
  if ($Anonymize -and $ser -ne "") { $ser = "***ANONYMIZED***" }

  $device = @{
    tool = "TrebleToolkit v$TTVersion"; timestamp = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
    mode = $TT.Mode; serial = $ser
    profile = $TT.ProfileId; target_partition = $DeviceProfiles[$TT.ProfileId].TargetPartition
    os = $TT.OS; props_subset = $TT.Props; firmware_baseline = $TT.FirmwareBaseline
    firmware_compat = $TT.FirmwareCompat
  }
  ($device | ConvertTo-Json -Depth 6) | Out-File (Join-Path $tmp "device.json") -Encoding utf8
  (Invoke-TTAdb @("devices") -join "`n") | Out-File (Join-Path $tmp "adb.txt") -Encoding utf8
  ($TT.FbRaw) | Out-File (Join-Path $tmp "fastboot.txt") -Encoding utf8
  ($TT.ByNameRaw) | Out-File (Join-Path $tmp "partitions.txt") -Encoding utf8
  (($TT.Props.GetEnumerator() | ForEach-Object { "$($_.Key) = $($_.Value)" }) -join "`n") | Out-File (Join-Path $tmp "properties.txt") -Encoding utf8
  @("cmdline:", $TT.Cmdline, "", "verifiedbootstate: " + [string]$TT.Props["ro.boot.verifiedbootstate"],
    "flash.locked: " + [string]$TT.Props["ro.boot.flash.locked"],
    "bootkeys: Vol-Up + Power bis Huawei-Logo (Magisk boot cheat)") -join "`n" | Out-File (Join-Path $tmp "boot.txt") -Encoding utf8
  @("baseline: " + $TT.FirmwareBaseline,
    "compat: " + (($TT.FirmwareCompat | ConvertTo-Json -Depth 3)),
    "advisory (NICHT exklusiv): VTR-L29 9.1.0.297(C432E5R1P9); VTR-L29 9.1.0.275(C432E2R1P9T8)") -join "`n" | Out-File (Join-Path $tmp "firmware.txt") -Encoding utf8
  $hashLines = @()
  foreach ($f in @($TT.StockImage, $TT.PatchedImage)) {
    if ($f -ne "" -and (Test-Path $f)) { $hi = Get-FileHashInfo $f; $hashLines += "$f`n  SHA256: $($hi.SHA256)`n  SHA512: $($hi.SHA512)`n  Size: $($hi.Size)" }
  }
  ($hashLines -join "`n") | Out-File (Join-Path $tmp "hashes.txt") -Encoding utf8
  ("TrebleToolkit v$TTVersion, PS $($PSVersionTable.PSVersion)") | Out-File (Join-Path $tmp "tool-version.txt") -Encoding utf8
  Copy-Item $TT.Log (Join-Path $tmp "log.txt") -Force -ErrorAction SilentlyContinue
  $zip = Join-Path $TTLogDir ("Huawei-P10-Diagnostic-" + $stamp + ".zip")
  try {
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Compress-Archive -Path (Join-Path $tmp "*") -DestinationPath $zip -Force
    Write-TTLog "Diagnose-ZIP: $zip" "SUCCESS"
  } catch { Write-TTLog "ZIP fehlgeschlagen: $($_.Exception.Message)" "ERROR"; return "" }
  return $zip
}

# ============================================================ TUI-Basis
function Show-TTHeader {
  param([string]$Title)
  Clear-Host
  $admin = "NEIN"; if (Test-TTAdmin) { $admin = "JA" }
  Write-Host "================================================================" -ForegroundColor DarkCyan
  Write-Host " Huawei P10 Root Manager  v$TTVersion  |  TUI (PS $($PSVersionTable.PSVersion))" -ForegroundColor Cyan
  Write-Host (" Modus: $($TT.Mode.ToUpper())  Admin: $admin  Profil: $($TT.ProfileId)  OS: $(if ($TT.OS) { $TT.OS.Kind } else { '?' })") -ForegroundColor DarkGray
  $chip = "READY"
  if ($TT.Mode -eq "none") { $chip = "WARNING: kein Geraet" }
  Write-Host (" State: $chip  |  Log: logs\") -ForegroundColor DarkGray
  Write-Host "================================================================`n" -ForegroundColor DarkCyan
  Write-Host " $Title" -ForegroundColor White
}

function Show-TTMenu {
  param([string]$Title, [string[]]$Options, [string]$Hint = "Pfeiltasten + Enter | 1-9 | Esc = zurueck")
  $sel = 0
  while ($true) {
    Show-TTHeader $Title
    Write-Host ""
    for ($i = 0; $i -lt $Options.Count; $i++) {
      $n = ($i + 1).ToString().PadLeft(2)
      if ($i -eq $sel) { Write-Host (" > [$n] " + $Options[$i]) -ForegroundColor Black -BackgroundColor Cyan }
      else { Write-Host ("   [$n] " + $Options[$i]) -ForegroundColor Gray }
    }
    Write-Host ""
    Write-Host " $Hint" -ForegroundColor DarkGray
    $key = [Console]::ReadKey($true)
    if ($key.Key -eq "UpArrow") { $sel = ($sel - 1 + $Options.Count) % $Options.Count }
    elseif ($key.Key -eq "DownArrow") { $sel = ($sel + 1) % $Options.Count }
    elseif ($key.Key -eq "Enter") { return $sel }
    elseif ($key.Key -eq "Escape") { return -1 }
    elseif ($key.KeyChar -ge "1" -and $key.KeyChar -le "9") {
      $idx = [int]$key.KeyChar.ToString() - 1
      if ($idx -ge 0 -and $idx -lt $Options.Count) { return $idx }
    }
  }
}

function Pause-TT {
  Write-Host ""
  Write-Host "[Enter] zurueck ..." -ForegroundColor DarkGray -NoNewline
  [void][Console]::ReadKey($true)
}

function Show-TTStatus {
  Show-TTHeader "Status (jedes OS wird erkannt, nichts vorausgesetzt)"
  Write-Host ""
  Write-Host ("Device-Profil : " + $TT.ProfileId + " (" + $DeviceProfiles[$TT.ProfileId].Marketing + ")") -ForegroundColor White
  if ($TT.OS -ne $null) {
    Write-Host ("OS-Klasse     : " + $TT.OS.Kind) -ForegroundColor White
    Write-Host ("Detail        : " + $TT.OS.Detail) -ForegroundColor Gray
    Write-Host ("GSI-Variante  : " + $TT.OS.GsiVariant) -ForegroundColor Gray
  } else { Write-Host "OS-Klasse     : (noch keine Analyse)" -ForegroundColor Yellow }
  Write-Host ("Android       : " + [string]$TT.Props["ro.build.version.release"]) -ForegroundColor White
  Write-Host ("Modus         : " + $TT.Mode) -ForegroundColor White
  $rr = ($TT.ByName | Where-Object { $_.Name -eq "recovery_ramdisk" } | Select-Object -First 1)
  if ($rr) { Write-Host "RecoveryRamdisk: DETECTED ($($rr.Target))" -ForegroundColor Green }
  else { Write-Host "RecoveryRamdisk: UNKNOWN/NOT DETECTED (Analyse laufen lassen)" -ForegroundColor Yellow }
  Write-Host ("Firmware      : " + $(if ($TT.FirmwareBaseline -ne "") { $TT.FirmwareBaseline } else { "UNKNOWN" })) -ForegroundColor White
  Write-Host ("Stock-Image   : " + $(if ($TT.StockImage -ne "") { $TT.StockImage } else { "fehlt" })) -ForegroundColor White
  Write-Host ("Patched-Image : " + $(if ($TT.PatchedImage -ne "") { $TT.PatchedImage } else { "fehlt" })) -ForegroundColor White
  Write-Host ("Backup        : " + $(if ($TT.BackupDir -ne "") { $TT.BackupDir } else { "fehlt" })) -ForegroundColor White
  Write-Host ""
  Write-Host "Bootloader: Unknown/Unlocked/Locked wird NICHT aus 'Command not allowed' geraten." -ForegroundColor DarkGray
  Write-Host "Magisk: nur verifiziert melden (uid=0), niemals aus Boot allein." -ForegroundColor DarkGray
  Pause-TT
}

# ============================================================ TUI-Screens (Wizard Steps 1-9)
function Screen-Detect {
  Show-TTHeader "Step 1 - Device (Detect, auto ADB/Fastboot)"
  Update-TTMode | Out-Null
  Write-Host ""
  Write-Host ("Modus: " + $TT.Mode) -ForegroundColor White
  if ($TT.AdbSerial -ne "") { Write-Host ("ADB: " + $TT.AdbSerial) -ForegroundColor White }
  if ($TT.FbSerial -ne "") { Write-Host ("Fastboot: " + $TT.FbSerial + " (z.B. 6PQ0217B08003446)") -ForegroundColor White }
  if ($TT.Mode -eq "none") {
    Write-Host ""
    Write-Host "Kein Geraet. USB-Debugging an / Fastboot-Treiber pruefen / Kabel wechseln." -ForegroundColor Red
  } else { Write-TTLog "Device-Modus: $($TT.Mode)" "SUCCESS" }
  Pause-TT
}

function Screen-Analyze {
  Show-TTHeader "Step 2 - Analyse (Android + Partitionen + Bootchain, read-only)"
  if ($TT.Mode -eq "none") { Update-TTMode | Out-Null }
  if ($TT.Mode -eq "android") {
    Invoke-TTAndroidAnalysis | Out-Null
    Write-Host ""
    Write-Host ("OS-Klasse: " + $TT.OS.Kind) -ForegroundColor Cyan
    Write-Host ("Detail: " + $TT.OS.Detail) -ForegroundColor White
    Write-Host ""
    Write-Host "--- Partitionen (/dev/block/by-name, gefiltert) ---" -ForegroundColor Cyan
    $hit = $false
    foreach ($n in $TT.ByName) {
      if ($n.Name -match "boot|recovery|ramdisk|system|vendor|vbmeta") {
        Write-Host (" " + $n.Name.PadRight(18) + " -> " + $n.Target) -ForegroundColor White
        $hit = $true
      }
    }
    if (-not $hit) { Write-Host $TT.ByNameRaw -ForegroundColor White }
    Write-Host ""
    Write-Host "--- Boot-State ---" -ForegroundColor Cyan
    Write-Host (" verifiedbootstate=" + [string]$TT.Props["ro.boot.verifiedbootstate"]) -ForegroundColor White
    Write-Host (" flash.locked=" + [string]$TT.Props["ro.boot.flash.locked"]) -ForegroundColor White
    Write-Host (" vbmeta=" + [string]$TT.Props["ro.boot.vbmeta.device_state"]) -ForegroundColor White
    Write-Host (" slot=" + [string]$TT.Props["ro.boot.slot_suffix"] + " (P10: kein klassisches A/B voraussetzen)") -ForegroundColor White
    $rr = $TT.ByName | Where-Object { $_.Name -eq "recovery_ramdisk" }
    if ($rr) { Write-Host "recovery_ramdisk: DETECTED" -ForegroundColor Green }
    else { Write-Host "recovery_ramdisk: NICHT in by-name gesehen -> Fastboot-Analyse + Firmware-Weg noetig." -ForegroundColor Yellow }
  } elseif ($TT.Mode -eq "fastboot") {
    Write-Host "Geraet in Fastboot -> erst Android-Analyse nach Reboot, jetzt Fastboot-Analyse." -ForegroundColor Yellow
    Invoke-TTFastbootAnalysis | Out-Null
    Write-Host ($TT.FbRaw) -ForegroundColor White
  } else {
    Write-Host "Kein Geraet verbunden." -ForegroundColor Red
  }
  # Report
  $rep = Join-Path $TTLogDir ("analyze-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".txt")
  @("OS: $(if ($TT.OS) { $TT.OS.Kind } else { '?' })",
    "Props:", (($TT.Props.GetEnumerator() | ForEach-Object { "$($_.Key) = $($_.Value)" }) -join "`n"),
    "", "by-name:", $TT.ByNameRaw) -join "`n" | Out-File $rep -Encoding utf8
  Write-TTLog "Analyse-Report: $rep" "SUCCESS"
  Pause-TT
}

function Screen-Firmware {
  Show-TTHeader "Step 3 - Firmware (kompatibel bestimmen, keine dubiosen Downloads)"
  $base = Get-TTFirmwareBaseline -Interactive
  Write-Host ""
  Write-Host ("Baseline: " + $(if ($base -ne "") { $base } else { "(leer)" })) -ForegroundColor White
  Write-Host ""
  Write-Host "Quellen (im UI, kein Auto-Download aus dubiosen Quellen):" -ForegroundColor Cyan
  Write-Host " - HUAWEI FIRM FINDER V2: https://professorjtj.github.io/v2/" -ForegroundColor White
  Write-Host " - Diskussion #2542: VTR-L29 9.1.0.297(C432E5R1P9) als BEISPIEL (nicht exklusiv)" -ForegroundColor White
  Write-Host " - Wiki P10: EMUI 9.1-Basis, PotatoNV-Unlock, RECOVERY_RAMDIS.img aus UPDATE.APP" -ForegroundColor White
  $region = ""
  $m = [regex]::Match([string]$base, "\(C(\d+)")
  if ($m.Success) { $region = "C" + $m.Groups[1].Value }
  else {
    Write-Host "Region/CUST (z.B. C432, Enter=unbekannt): " -NoNewline -ForegroundColor Yellow
    $region = Read-Host
  }
  $model = $TT.ProfileId
  if ([string]::IsNullOrEmpty($model)) { $model = "VTR-L29" }
  $TT.FirmwareCompat = Test-FirmwareCompatibility $model $base $region
  Write-Host ""
  Write-Host ("Kompatibilitaet: " + $TT.FirmwareCompat.Status) -ForegroundColor $(if ($TT.FirmwareCompat.Status -eq "PASS") { "Green" } elseif ($TT.FirmwareCompat.Status -eq "WARN") { "Yellow" } else { "Red" })
  foreach ($r in $TT.FirmwareCompat.Reasons) { Write-Host (" - " + $r) -ForegroundColor Gray }
  if ($TT.FirmwareCompat.Status -eq "FAIL") { Write-Host "FAIL -> NICHT fortfahren, andere Firmware waehlen." -ForegroundColor Red }
  Write-Host ""
  Write-Host "[D] Stock-Firmware herunterladen (mit Fortschritt)  [Enter] zurueck" -ForegroundColor DarkGray -NoNewline
  $k = [Console]::ReadKey($true)
  Write-Host ""
  if ($k.KeyChar -eq "d" -or $k.KeyChar -eq "D") { Screen-DownloadFirmware }
  else { return }
}

function Screen-DownloadFirmware {
  Show-TTHeader "Stock-Firmware Download (original, mit Fortschritt + Bestaetigung)"
  Write-Host ""
  Write-Host "Ziel: Original-Firmware neu installieren ODER nur besitzen (UPDATE.APP -> Step 4)." -ForegroundColor Gray
  Write-Host "Ablage: data/firmware/ (Offline-Cache, kein Re-Download). Nur Full-Pakete (keine Deltas)." -ForegroundColor Gray
  Write-Host ""
  Write-Host "Zuverlaessige Quellen (ehrlich, kein garantierter Direktlink 2026):" -ForegroundColor Cyan
  $i = 1
  foreach ($s in $FirmwareSources) {
    Write-Host (" [$i] " + $s.Name + " [" + $s.Kind + "]") -ForegroundColor White
    Write-Host ("     " + $s.Url) -ForegroundColor DarkGray
    Write-Host ("     " + $s.Note) -ForegroundColor Gray
    $i++
  }
  Write-Host ""
  Write-Host "Freier Platz: " -NoNewline -ForegroundColor Gray
  try {
    $drv = (Get-Item $TTFirmDir).PSDrive.Name
    $vol = Get-WmiObject Win32_LogicalDisk -Filter ("DeviceID='" + $drv + ":'") -ErrorAction SilentlyContinue
    if ($vol) { Write-Host ([math]::Round($vol.FreeSpace/1GB,1).ToString() + " GB frei auf " + $drv + ": (Full-Firmware ca. 2-4 GB)") -ForegroundColor White }
    else { Write-Host "(unbekannt, ca. 2-4 GB einplanen)" -ForegroundColor White }
  } catch { Write-Host "(unbekannt, ca. 2-4 GB einplanen)" -ForegroundColor White }
  Write-Host ""
  Write-Host "Direktlink zum Full-Firmware-ZIP einfuegen (Enter = nur Quelle oeffnen/abbruch): " -NoNewline -ForegroundColor Yellow
  $url = Read-Host
  if ([string]::IsNullOrWhiteSpace($url)) {
    Write-Host "Kein Download gestartet. Quelle im Browser oeffnen? [J/N]: " -NoNewline -ForegroundColor Yellow
    $o = Read-Host
    if ($o -eq "J" -or $o -eq "j") {
      try { Start-Process $FirmwareSources[3].Url | Out-Null } catch { Write-TTLog $_.Exception.Message "ERROR" }
    }
    Pause-TT; return
  }
  $chk = Test-FirmwareUrl $url
  if (-not $chk.Ok) { Write-Host ("URL abgelehnt: " + $chk.Reason) -ForegroundColor Red; Write-TTLog ("URL abgelehnt: " + $chk.Reason) "ERROR"; Pause-TT; return }
  $fname = "stock-firmware-" + $TT.ProfileId + "-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".zip"
  $low = $url.Split("?")[0].ToLower()
  foreach ($e in @(".zip",".7z",".tar",".gz",".tgz",".app",".rar")) {
    if ($low.EndsWith($e)) {
      try { $fname = "stock-firmware-" + $TT.ProfileId + "-" + [System.IO.Path]::GetFileName($low.Split("/")[-1]) } catch {}
    }
  }
  $out = Join-Path $TTFirmDir $fname
  Write-Host ""
  Write-Host "=== Bestaetigung (Pflicht vor Download) ===" -ForegroundColor Cyan
  Write-Host ("URL   : " + $url) -ForegroundColor White
  Write-Host ("Host  : " + $chk.Host) -ForegroundColor White
  Write-Host ("Ziel  : " + $out) -ForegroundColor White
  Write-Host "Hinweis: Quelle + Hash nach Download pruefen. Nur Full-Firmware, Region/CUST beachten." -ForegroundColor Yellow
  Write-Host "Wirklich herunterladen? 'JA' tippen (ca. 2-4 GB): " -NoNewline -ForegroundColor Yellow
  $c = Read-Host
  if ($c -ne "JA") { Write-TTLog "Download abgebrochen (kein JA)." "WARNING"; Pause-TT; return }
  $res = Invoke-FirmwareDownload $url $out
  if (-not $res.Ok) { Write-Host "Download fehlgeschlagen, siehe Log." -ForegroundColor Red; Pause-TT; return }
  if ($res.Cached) { Write-Host "Bereits vorhanden (Cache), kein Re-Download." -ForegroundColor Yellow }
  $ver = Test-DownloadedFirmware $out
  Write-Host ""
  foreach ($n in $ver.Notes) { Write-Host (" - " + $n) -ForegroundColor White }
  Write-Host ("SHA-256: " + $ver.SHA256) -ForegroundColor Gray
  if ($ver.Ok) { Write-TTLog ("Firmware bereit: " + $out) "SUCCESS"; Write-Host "Weiter: TUI Step 4 (UPDATE.APP -> RECOVERY_RAMDISK)." -ForegroundColor Green }
  else { Write-Host "WARN: Paket unplausibel (siehe oben). NICHT als Restore-Basis nutzen, andere Quelle waehlen." -ForegroundColor Red }
  Pause-TT
}

function Screen-Extract {
  Show-TTHeader "Step 4 - RECOVERY_RAMDISK extrahieren/validieren (UPDATE.APP)"
  Write-Host ""
  Write-Host "Erwartet: UPDATE.APP (Full-Firmware) in data/firmware/ ablegen." -ForegroundColor Gray
  $apps = Get-ChildItem -Path $TTFirmDir -Filter "*.APP" -File -Recurse -ErrorAction SilentlyContinue
  if ($apps) {
    Write-Host "Gefunden:" -ForegroundColor Cyan
    $i = 1
    foreach ($a in $apps) { Write-Host (" [$i] " + $a.FullName + " (" + [math]::Round($a.Length/1MB,1) + " MB)"); $i++ }
  } else { Write-Host "Keine UPDATE.APP in data/firmware/." -ForegroundColor Yellow }
  Write-Host ""
  Write-Host "UPDATE.APP-Pfad (Enter = ueberspringen, nur vorhandene Images suchen): " -NoNewline -ForegroundColor Yellow
  $p = Read-Host
  if ($p -eq "" -and $apps -and $apps.Count -ge 1) { $p = $apps[0].FullName }
  if ($p -ne "" -and (Test-Path $p)) {
    $info = Invoke-TTUpdateAppAnalysis $p
    Write-Host ""
    Write-Host ("Groesse: " + $info.Size) -ForegroundColor White
    Write-Host ("Header: " + $info.HeaderHex) -ForegroundColor Gray
    Write-Host $info.Note -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Extraktoren (in data/tools/ ablegen, falls vorhanden): huawei-update-extractor / splitupdate." -ForegroundColor Gray
    Write-Host "Manuell: UPDATE.APP mit vertrauenswuerdigem Extractor oeffnen, RECOVERY_RAMDIS(K).img nach data/firmware/ kopieren." -ForegroundColor Gray
    Write-Host "WICHTIG: Exakte Bezeichnung beibehalten (RECOVERY_RAMDIS.img vs RECOVERY_RAMDISK.img), NICHT umbenennen." -ForegroundColor Red
  }
  $found = Find-TTRecoveryImage
  Write-Host ""
  if ($found.Count -gt 0) {
    Write-Host "RECOVERY-Images gefunden:" -ForegroundColor Green
    foreach ($f in $found) { Write-Host (" - " + $f) -ForegroundColor White }
    $TT.StockImage = $found[0]
    $chk = Test-RecoveryImageFile $TT.StockImage
    Write-Host ("Pruefung: " + $chk.Verdict + " | Size=" + $chk.Hash.Size + " | " + $chk.HeaderHex) -ForegroundColor White
    foreach ($n in $chk.Notes) { Write-Host ("   " + $n) -ForegroundColor Gray }
    $TT.StockHash = $chk.Hash
    if ($chk.Verdict -ne "PASS") { Write-Host "Image nicht verifizierbar -> Cannot safely flash." -ForegroundColor Red }
  } else {
    Write-Host "Keine RECOVERY_RAMDIS(K).img gefunden. Erst beschaffen, dann fortfahren." -ForegroundColor Red
  }
  Pause-TT
}

function Screen-Patch {
  Show-TTHeader "Step 5 - Magisk (kompatibel waehlen, echt patchen)"
  Write-Host ""
  Write-Host "Kompatibilitaet (nicht blind latest):" -ForegroundColor Cyan
  foreach ($r in $MagiskCompatTable) { Write-Host (" - " + $r.Android + ": " + $r.Tested + " -- " + $r.Note) -ForegroundColor Gray }
  Write-Host ""
  Write-Host "Offiziell: https://github.com/topjohnwu/Magisk/releases" -ForegroundColor White
  $apk = Find-TTMagiskApk
  if ($apk -eq "") {
    Write-Host "Keine Magisk-APK in data/magisk/. APK dort ablegen (offizielles GitHub)." -ForegroundColor Yellow
    Write-Host "APK-Pfad (Enter=spaeter): " -NoNewline -ForegroundColor Yellow
    $p = Read-Host
    if ($p -ne "" -and (Test-Path $p)) {
      $dst = Join-Path $TTMagDir ([System.IO.Path]::GetFileName($p))
      Copy-Item $p $dst -Force
      $apk = $dst
    }
  }
  if ($apk -ne "") {
    $TT.MagiskApk = $apk
    $TT.MagiskInfo = Get-TTMagiskInfo $apk
    Write-Host ("APK: " + $TT.MagiskInfo.Path) -ForegroundColor White
    Write-Host ("Version: " + $TT.MagiskInfo.Version + " | SHA256: " + $TT.MagiskInfo.SHA256) -ForegroundColor White
  }
  if ($TT.StockImage -eq "" -or -not (Test-Path $TT.StockImage)) {
    Write-Host "Kein Stock-Image registriert -> erst Step 4." -ForegroundColor Red
    Pause-TT; return
  }
  Write-Host ""
  Write-Host ("Input: " + $TT.StockImage) -ForegroundColor White
  Write-Host ("Target partition: " + $DeviceProfiles[$TT.ProfileId].TargetPartition) -ForegroundColor White
  Write-Host ("Device: Huawei " + $TT.ProfileId) -ForegroundColor White
  Write-Host ("Firmware: " + $TT.FirmwareBaseline) -ForegroundColor White
  Write-Host ("Magisk: " + $(if ($TT.MagiskInfo) { $TT.MagiskInfo.Version } else { "(APK fehlt)" })) -ForegroundColor White
  Write-Host ("SHA-256: " + $TT.StockHash.SHA256) -ForegroundColor White
  Write-Host "Explizite Bestaetigung vor Patch-Vorbereitung noetig (kein boot.img/recovery.img patchen)." -ForegroundColor Yellow
  Write-Host "[1] Patch vorbereiten (to-patch + Anleitung)  [2] Gepatchte Datei registrieren  [Esc]" -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  if ($k.KeyChar -eq "1") {
    Prepare-TTMagiskPatch $TT.StockImage | Out-Null
    Write-Host "Vorbereitet. Jetzt am Geraet patchen (Anleitung in data/magisk/to-patch/)." -ForegroundColor Green
  } elseif ($k.KeyChar -eq "2") {
    Write-Host ""
    # adb pull Versuch (echt, kein Mock)
    Write-Host "Versuche adb pull /sdcard/Download/magisk_patched-*.img ..." -ForegroundColor Gray
    try {
      $lst = (Invoke-TTAdb @("shell","ls /sdcard/Download/magisk_patched*.img 2>&1") -join "`n").Trim()
      Write-Host $lst -ForegroundColor White
    } catch {}
    Write-Host "Pfad zur gepatchten Datei (data/magisk/*.img): " -NoNewline -ForegroundColor Yellow
    $pp = Read-Host
    if ($pp -ne "" -and (Test-Path $pp)) {
      $chk = Test-RecoveryImageFile $pp
      if ($chk.Hash.SHA256 -eq $TT.StockHash.SHA256) {
        Write-Host "FEHLER: Gepatcht == Stock (Hash identisch). KEIN Fake-Patch akzeptiert." -ForegroundColor Red
        Write-TTLog "Fake-Patch abgelehnt (Hash identisch)." "ERROR"
      } elseif ($chk.Verdict -eq "PASS") {
        $TT.PatchedImage = $pp; $TT.PatchedHash = $chk.Hash
        Write-TTLog ("Patched registriert: $pp SHA256=" + $chk.Hash.SHA256) "SUCCESS"
      } else {
        Write-Host "Image-Pruefung FAIL, Details oben." -ForegroundColor Red
      }
    }
  }
  Pause-TT
}

function Screen-Backup {
  Show-TTHeader "Step 6 - Backup (Pflicht vor Flash)"
  if ($TT.StockImage -eq "") { Write-Host "Kein Stock-Image -> erst Step 4." -ForegroundColor Red; Pause-TT; return }
  $r = New-TTBackup $TT.StockImage
  Write-Host ""
  Write-Host ("Backup: " + $r.Dir + " OK=" + $r.Ok) -ForegroundColor White
  Write-Host "Enthaelt: original.img + metadata.json + sha256.txt (+partition-probe)." -ForegroundColor Gray
  Pause-TT
}

function Screen-Flash {
  Show-TTHeader "Step 7 - Flash (nur nach Safety-Gate, abgeleitet aus Profil)"
  if ($TT.Mode -ne "fastboot") {
    Write-Host "Nicht in Fastboot. Jetzt 'adb reboot bootloader' + warten? [J/N]: " -NoNewline -ForegroundColor Yellow
    $k = [Console]::ReadKey($true); Write-Host $k.KeyChar
    if ($k.KeyChar -eq "j" -or $k.KeyChar -eq "J") {
      Invoke-TTAdb @("reboot","bootloader") | Out-Null
      for ($i = 0; $i -lt 30; $i++) {
        Start-Sleep -Seconds 1
        $o = Invoke-TTFastboot @("devices")
        $d = ConvertFrom-FastbootDevices $o
        if ($d.Count -gt 0) { $TT.Mode = "fastboot"; $TT.FbSerial = $d[0].Serial; break }
      }
    }
  }
  Invoke-TTSafeFlash | Out-Null
  Pause-TT
}

function Screen-RebootVerify {
  Show-TTHeader "Step 8+9 - Reboot (Huawei-Prozedur) + Verify (echt)"
  $r = Invoke-TTRootVerification
  Write-Host ""
  Write-Host ("Ergebnis: " + $r.Root) -ForegroundColor $(if ($r.Root -eq "ROOTED") { "Green" } else { "Yellow" })
  if ($r.Root -ne "ROOTED") {
    Write-Host "Kein Erfolg vortaeuschen: Boot allein != Root. Trick wiederholen, Magisk-App pruefen, Diagnose-ZIP erzeugen." -ForegroundColor Yellow
  }
  Pause-TT
}

function Screen-Restore {
  Show-TTHeader "Restore / Unroot (Safe Restore: nur Original-Partition)"
  Invoke-TTRestoreFlow | Out-Null
  Pause-TT
}

function Screen-Tools {
  while ($true) {
    $c = Show-TTMenu "Tools (read-only wo moeglich)" @(
      "adb devices -l",
      "Reboot-Menue (bootloader/recovery/fastbootd/system)",
      "fastboot devices + getvar (tolerant, Huawei FAILED ok)",
      "getprop Full-Dump -> logs/",
      "adb kill-server/start-server",
      "Diagnose-ZIP erzeugen",
      "Zurueck"
    )
    if ($c -eq -1 -or $c -eq 6) { return }
    if ($c -eq 0) { Show-TTHeader "adb devices"; Write-Host ""; Write-Host ((Invoke-TTAdb @("devices","-l") -join "`n") ) -ForegroundColor White; Pause-TT }
    elseif ($c -eq 1) {
      $s = Show-TTMenu "Reboot-Ziel" @("bootloader","recovery","fastbootd","system","Abbrechen")
      if ($s -eq 0) { Invoke-TTAdb @("reboot","bootloader") | Out-Null }
      elseif ($s -eq 1) { Invoke-TTAdb @("reboot","recovery") | Out-Null }
      elseif ($s -eq 2) { Invoke-TTAdb @("reboot","fastboot") | Out-Null }
      elseif ($s -eq 3) { Invoke-TTAdb @("reboot") | Out-Null }
      Pause-TT
    }
    elseif ($c -eq 2) {
      Show-TTHeader "fastboot";
      Invoke-TTFastbootAnalysis | Out-Null
      Write-Host $TT.FbRaw -ForegroundColor White
      Write-Host "`n'waiting for device'/FAILED = Treiber/Modus/Huawei-Eigenheit, nicht auto=locked." -ForegroundColor Yellow
      Pause-TT
    }
    elseif ($c -eq 3) {
      $o = (Invoke-TTAdb @("shell","getprop") -join "`n")
      $f = Join-Path $TTLogDir ("getprop-full-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".txt")
      $o | Out-File $f -Encoding utf8
      Write-TTLog "Dump: $f" "SUCCESS"; Pause-TT
    }
    elseif ($c -eq 4) { Invoke-TTAdb @("kill-server") | Out-Null; Invoke-TTAdb @("start-server") | Out-Null; Write-TTLog "ADB-Server reset." "SUCCESS"; Pause-TT }
    elseif ($c -eq 5) {
      $z = New-TTDiagnostic
      Write-Host ("ZIP: " + $z) -ForegroundColor White
      Pause-TT
    }
  }
}

function Screen-Bootkeys {
  Show-TTHeader "Huawei Boot-Mechanismus (aus #2542, exakt)"
  Write-Host ""
  Write-Host "- Magisk-Boot: Vol-Up + Power bis Huawei-Logo, dann loslassen (Magisk boot cheat)." -ForegroundColor White
  Write-Host "- Ohne Trick: Stock-Boot (kein Root). Verhalten NICHT persistent." -ForegroundColor White
  Write-Host "- Persistent-Byte setzen: Warnscreen -> eRecovery (Vol-Up 3s) -> Wipe/Factory Reset bestaetigen+reboot" -ForegroundColor White
  Write-Host "  -> bootet Magisk-Root (Wipe wird dabei NICHT ausgefuehrt)." -ForegroundColor White
  Write-Host "- Byte loeschen: /dload entfernen! Power off -> Vol-Up+Vol-Down+Power bis Logo" -ForegroundColor White
  Write-Host "  -> EMUI 'OS Upgrade not successful' -> Reboot -> clean." -ForegroundColor White
  Write-Host "- /dload darf NICHT auf Speicher liegen, sonst EMUI-Updater statt Recovery." -ForegroundColor Red
  Write-Host "- Keine Pixel-/A-B-Anleitung verwenden." -ForegroundColor Yellow
  Pause-TT
}

function Screen-Logs {
  Show-TTHeader "Logs (logs/) + Diagnose"
  $files = Get-ChildItem $TTLogDir -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 15
  if (-not $files) { Write-Host "Keine Logs." -ForegroundColor Yellow; Pause-TT; return }
  $names = @()
  foreach ($f in $files) { $names += ($f.Name) }
  $names += "Log-Ordner oeffnen"; $names += "Diagnose-ZIP erzeugen"; $names += "Zurueck"
  $c = Show-TTMenu "Log waehlen" $names
  if ($c -eq -1 -or $c -eq ($names.Count-1)) { return }
  if ($c -eq ($names.Count-3)) { try { Start-Process explorer.exe $TTLogDir | Out-Null } catch {} ; return }
  if ($c -eq ($names.Count-2)) { $z = New-TTDiagnostic; Write-Host $z -ForegroundColor White; Pause-TT; return }
  $file = $files[$c].FullName
  Show-TTHeader ("Log: " + $files[$c].Name)
  Get-Content $file -TotalCount 200 | ForEach-Object { Write-Host $_ -ForegroundColor Gray }
  Pause-TT
}

function Start-TTWizard {
  $steps = @("Detect","Analyse","Firmware","Extract","Patch","Backup","Flash","Reboot+Verify")
  $i = 0
  while ($i -lt $steps.Count) {
    if ($i -eq 0) { Screen-Detect }
    elseif ($i -eq 1) { Screen-Analyze }
    elseif ($i -eq 2) { Screen-Firmware }
    elseif ($i -eq 3) { Screen-Extract }
    elseif ($i -eq 4) { Screen-Patch }
    elseif ($i -eq 5) { Screen-Backup }
    elseif ($i -eq 6) { Screen-Flash }
    elseif ($i -eq 7) { Screen-RebootVerify }
    Show-TTHeader ("Wizard Step " + ($i+1) + "/" + $steps.Count + " fertig: " + $steps[$i])
    Write-Host ""
    Write-Host "[Enter]=weiter  [Z]=zurueck  [Esc]=Wizard verlassen" -ForegroundColor DarkGray
    $k = [Console]::ReadKey($true)
    if ($k.Key -eq "Escape") { break }
    elseif ($k.KeyChar -eq "z" -or $k.KeyChar -eq "Z") { if ($i -gt 0) { $i-- } }
    else { $i++ }
  }
}

function Start-TTTui {
  Find-TTTools
  Update-TTMode | Out-Null
  # Defaults fuer Zielgeraet (Beispielwerte aus Auftrag, ueberschreibbar durch Analyse)
  if ($TT.ProfileId -eq "") { $TT.ProfileId = "VTR-L29" }
  if (-not $NoElevateCheck -and -not (Test-TTAdmin)) {
    Write-TTLog "Kein Admin. Fastboot/USB-Treiber brauchen ggf. Admin (BAT fragt Elevation, Menue=Admin-Neustart)." "WARNING"
  }
  while ($true) {
    $c = Show-TTMenu "Hauptmenue - Huawei P10 Root Manager (OS-unabhaengig)" @(
      "Status-Uebersicht",
      "Wizard Step 1-9 (gefuehrt)",
      "Step 1 - Device erkennen",
      "Step 2 - Analyse (OS/Partitionen/Bootchain)",
      "Step 3 - Firmware bestimmen",
      "Step 4 - RECOVERY_RAMDISK extrahieren/validieren",
      "Step 5 - Magisk vorbereiten/patchen",
      "Step 6 - Backup erstellen",
      "Step 7 - Flash recovery_ramdisk (Safety-Gate)",
      "Step 8+9 - Reboot + Root verifizieren",
      "Restore / Unroot",
      "Boot-Tricks (Huawei, exakt)",
      "Tools + Diagnose-ZIP",
      "Logs",
      "Admin-Neustart",
      "Beenden"
    ) "GSI bleibt erhalten | Nie userdata loeschen | Nie Bootloader-Unlock"
    if ($c -eq -1 -or $c -eq 15) { Write-TTLog ("Beendet. Log: " + $TT.Log) "SUCCESS"; break }
    if ($c -eq 0) { Show-TTStatus }
    elseif ($c -eq 1) { Start-TTWizard }
    elseif ($c -eq 2) { Screen-Detect }
    elseif ($c -eq 3) { Screen-Analyze }
    elseif ($c -eq 4) { Screen-Firmware }
    elseif ($c -eq 5) { Screen-Extract }
    elseif ($c -eq 6) { Screen-Patch }
    elseif ($c -eq 7) { Screen-Backup }
    elseif ($c -eq 8) { Screen-Flash }
    elseif ($c -eq 9) { Screen-RebootVerify }
    elseif ($c -eq 10) { Screen-Restore }
    elseif ($c -eq 11) { Screen-Bootkeys }
    elseif ($c -eq 12) { Screen-Tools }
    elseif ($c -eq 13) { Screen-Logs }
    elseif ($c -eq 14) {
      try {
        $exe = (Get-Process -Id $PID).Path
        $sp = $MyInvocation.MyCommand.Path
        if ([string]::IsNullOrEmpty($sp)) { $sp = $PSCommandPath }
        Start-Process -FilePath $exe -ArgumentList ("-NoProfile -ExecutionPolicy Bypass -File `"$sp`"") -Verb RunAs | Out-Null
        exit 0
      } catch { Write-TTLog "Elevation abgebrochen: $($_.Exception.Message)" "ERROR"; Pause-TT }
    }
  }
}

# ============================================================ CLI
function Show-TTHelp {
  Write-Host "Huawei P10 Root Manager v$TTVersion" -ForegroundColor Cyan
  Write-Host "Usage: Treble-Toolkit.ps1 [detect|analyze|firmware|download|extract|patch|backup|flash|verify|restore|diagnostic|wizard|help] [--json] [--yes] [--image <pfad>] [--firmware-file <url|pfad>] [--anonymize] [--no-reboot]" -ForegroundColor White
  Write-Host "Ohne Args: TUI. Download/Flash/Restore brauchen explizite Bestaetigung (--yes = dokumentierte Zustimmung)." -ForegroundColor Gray
}

if ($Help) { Show-TTHelp; exit 0 }

# Args wie --json/--yes tolerieren (PS-Bindung + manuell)
foreach ($a in $args) {
  if ($a -eq "--json") { $Json = $true }
  if ($a -eq "--yes") { $Yes = $true }
  if ($a -eq "--anonymize") { $Anonymize = $true }
  if ($a -eq "--no-reboot") { $NoReboot = $true }
}

$cmd = $Command.ToLower().Trim()
if ($cmd -eq "" -and $args.Count -gt 0 -and -not $args[0].StartsWith("-")) { $cmd = $args[0].ToLower().Trim() }
if ($cmd.StartsWith("--")) { $cmd = "" }

if ($cmd -eq "") {
  Start-TTTui
  exit 0
}

Find-TTTools
Update-TTMode | Out-Null
if ($TT.ProfileId -eq "") { $TT.ProfileId = "VTR-L29" }

if ($cmd -eq "help") { Show-TTHelp; exit 0 }
elseif ($cmd -eq "detect") {
  $o = @{ mode = $TT.Mode; adb = $TT.AdbSerial; fastboot = $TT.FbSerial; adb_path = $TT.Adb; fastboot_path = $TT.Fastboot }
  if ($Json) { ($o | ConvertTo-Json -Depth 3) | Write-Host } else { ($o.GetEnumerator() | ForEach-Object { "$($_.Key): $($_.Value)" }) | ForEach-Object { Write-Host $_ } }
}
elseif ($cmd -eq "analyze") {
  if ($TT.Mode -eq "android") { Invoke-TTAndroidAnalysis | Out-Null }
  if ($TT.Mode -eq "fastboot" -or $TT.Fastboot) { Invoke-TTFastbootAnalysis | Out-Null }
  $o = @{ mode = $TT.Mode; os = $TT.OS; props = $TT.Props; byname = ($TT.ByName | ForEach-Object { "$($_.Name) -> $($_.Target)" }); fastboot = $TT.FbVars }
  if ($Json) { ($o | ConvertTo-Json -Depth 6) | Write-Host }
  else {
    Write-Host ("OS: " + $TT.OS.Kind + " | " + $TT.OS.Detail)
    Write-Host "Partitionen:"; foreach ($n in $TT.ByName) { Write-Host (" " + $n.Name + " -> " + $n.Target) }
  }
}
elseif ($cmd -eq "diagnostic") {
  if ($TT.Mode -eq "android") { Invoke-TTAndroidAnalysis | Out-Null }
  $z = New-TTDiagnostic -Anonymize:$Anonymize
  if ($Json) { (@{ zip = $z } | ConvertTo-Json) | Write-Host } else { Write-Host ("ZIP: " + $z) }
}
elseif ($cmd -eq "firmware") {
  if ($FirmwareFile -ne "") { $TT.FirmwareBaseline = $FirmwareFile }
  Get-TTFirmwareBaseline | Out-Null
  $TT.FirmwareCompat = Test-FirmwareCompatibility $TT.ProfileId $TT.FirmwareBaseline ""
  if ($Json) { (@{ baseline = $TT.FirmwareBaseline; compat = $TT.FirmwareCompat } | ConvertTo-Json -Depth 4) | Write-Host }
  else { Write-Host ($TT.FirmwareBaseline + " [" + $TT.FirmwareCompat.Status + "]") }
}
elseif ($cmd -eq "extract") {
  # Read-only: UPDATE.APP analysieren + vorhandene RECOVERY-Images validieren. Kein Flash.
  $appPath = $FirmwareFile
  if ($appPath -eq "" -and $Image -ne "") { $appPath = $Image }
  if ($appPath -eq "") {
    $apps = Get-ChildItem -Path $TTFirmDir -Filter "*.APP" -File -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($apps) { $appPath = $apps.FullName }
  }
  $appInfo = $null
  if ($appPath -ne "" -and (Test-Path $appPath)) { $appInfo = Invoke-TTUpdateAppAnalysis $appPath }
  else { $appInfo = @{ Exists = $false; Size = 0; HeaderHex = ""; Note = "Keine UPDATE.APP gefunden. Lege Full-Firmware nach data/firmware/ (kein Auto-Download aus dubiosen Quellen)." } }
  $found = Find-TTRecoveryImage
  $validated = @()
  foreach ($f in $found) {
    $chk = Test-RecoveryImageFile $f
    $validated += @{ path = $f; verdict = $chk.Verdict; size = $chk.Hash.Size; sha256 = $chk.Hash.SHA256; header = $chk.HeaderHex; notes = ($chk.Notes -join " | ") }
    if ($TT.StockImage -eq "") { $TT.StockImage = $f; $TT.StockHash = $chk.Hash }
  }
  $o = @{ update_app = $appPath; app_info = $appInfo; images = $validated }
  if ($Json) { ($o | ConvertTo-Json -Depth 5) | Write-Host }
  else {
    Write-Host ("UPDATE.APP: " + $appPath)
    Write-Host ($appInfo.Note)
    if ($validated.Count -eq 0) { Write-Host "Keine RECOVERY_RAMDIS(K).img in data/firmware/ - erst beschaffen." }
    foreach ($v in $validated) { Write-Host (" - " + $v.path + " [" + $v.verdict + "] SHA256=" + $v.sha256) }
  }
  if ($validated.Count -eq 0) { exit 3 }
}
elseif ($cmd -eq "patch") {
  # Read-only Vorbereitung: Stock validieren + to-patch staging + Anleitung. KEIN Fake-Patch.
  $stock = $Image
  if ($stock -eq "" -and $TT.StockImage -ne "") { $stock = $TT.StockImage }
  if ($stock -eq "") {
    $found = Find-TTRecoveryImage
    if ($found.Count -gt 0) { $stock = $found[0] }
  }
  if ($stock -eq "" -or -not (Test-Path $stock)) { Write-Host "Kein Stock-Image gefunden. Erst 'extract' (UPDATE.APP -> data/firmware/)." ; exit 3 }
  $chk = Test-RecoveryImageFile $stock
  if ($chk.Verdict -ne "PASS") { Write-Host ("Stock-Image FAIL: " + ($chk.Notes -join " | ")) ; exit 3 }
  $TT.StockImage = $stock; $TT.StockHash = $chk.Hash
  $apk = Find-TTMagiskApk
  $mi = $null
  if ($apk -ne "") { $mi = Get-TTMagiskInfo $apk }
  $staged = Prepare-TTMagiskPatch $stock
  $o = @{ stock = $stock; sha256 = $chk.Hash.SHA256; magisk_apk = $apk; magisk = $mi; staged = $staged; next = "Am Geraet: Magisk -> Install -> Select and Patch a File -> adb pull /sdcard/Download/magisk_patched-*.img data/magisk/" }
  if ($Json) { ($o | ConvertTo-Json -Depth 5) | Write-Host }
  else { Write-Host ("Stock OK. Staged: " + $staged); Write-Host "Naechster Schritt: on-device patchen, dann in TUI 'Gepatchte Datei registrieren'." }
}
elseif ($cmd -eq "backup") {
  $stock = $Image
  if ($stock -eq "" -and $TT.StockImage -ne "") { $stock = $TT.StockImage }
  if ($stock -eq "") {
    $found = Find-TTRecoveryImage
    if ($found.Count -gt 0) { $stock = $found[0] }
  }
  if ($stock -eq "" -or -not (Test-Path $stock)) { Write-Host "Kein Stock-Image fuer Backup. Erst 'extract'." ; exit 3 }
  $chk = Test-RecoveryImageFile $stock
  if ($chk.Verdict -ne "PASS") { Write-Host "Stock-Image nicht verifizierbar - Backup wird NICHT vorgetaeuscht." ; exit 3 }
  $TT.StockImage = $stock; $TT.StockHash = $chk.Hash
  $r = New-TTBackup $stock
  if ($Json) { ($r | ConvertTo-Json -Depth 3) | Write-Host } else { Write-Host ("Backup: " + $r.Dir + " OK=" + $r.Ok) }
  if (-not $r.Ok) { exit 3 }
}
elseif ($cmd -eq "download") {
  # Stock-Firmware herunterladen (original). Ohne --yes nur Kandidat zeigen, kein Traffic.
  $url = $FirmwareFile
  if ($url -eq "" -and $Image -ne "") { $url = $Image }
  if ($url -eq "") { Write-Host "URL fehlt. Beispiel: Treble-Toolkit.ps1 download --firmware-file <https-URL-zum-Full-ZIP> [--yes]"; exit 4 }
  $chk = Test-FirmwareUrl $url
  if (-not $chk.Ok) { Write-Host ("URL abgelehnt: " + $chk.Reason); exit 4 }
  $fname = "stock-firmware-" + $TT.ProfileId + "-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".zip"
  $out = Join-Path $TTFirmDir $fname
  if (-not $Yes) {
    $o = @{ url = $url; host = $chk.Host; out = $out; note = "Bestaetigung fehlt. Mit --yes herunterladen (2-4 GB)." }
    if ($Json) { ($o | ConvertTo-Json -Depth 3) | Write-Host } else { Write-Host ("Bereit (nicht geladen): " + $url + " -> " + $out + " | Mit --yes bestaetigen.") }
    exit 4
  }
  $res = Invoke-FirmwareDownload $url $out
  if (-not $res.Ok) { exit 1 }
  $ver = Test-DownloadedFirmware $out
  $o = @{ path = $out; sha256 = $ver.SHA256; ok = $ver.Ok; notes = $ver.Notes }
  if ($Json) { ($o | ConvertTo-Json -Depth 4) | Write-Host } else { Write-Host ("Fertig: " + $out + " OK=" + $ver.Ok) }
  if (-not $ver.Ok) { exit 3 }
}
elseif ($cmd -eq "flash") {
  if ($Image -ne "") { $TT.PatchedImage = $Image; $TT.PatchedHash = (Test-RecoveryImageFile $Image).Hash }
  $ok = Invoke-TTSafeFlash -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "verify") {
  $r = Invoke-TTRootVerification -NoReboot:$NoReboot
  if ($Json) { ($r | ConvertTo-Json -Depth 4) | Write-Host }
  if ($r.Root -ne "ROOTED") { exit 2 }
}
elseif ($cmd -eq "restore") {
  $ok = Invoke-TTRestoreFlow -BackupPick $Image -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "wizard") { Start-TTTui }
else {
  Write-Host "Unbekanntes Kommando: $cmd" -ForegroundColor Yellow
  Show-TTHelp
  exit 1
}
