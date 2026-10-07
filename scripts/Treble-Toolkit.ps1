<#
.SYNOPSIS
  Huawei P10 (VTR-L29 and others) TrebleDroid / Magisk Root Manager - PowerShell TUI + CLI.
  OS-independent: Stock EMUI, TrebleDroid/Lineage GSI, AOSP/PE GSI, custom ROMs.

.DESCRIPTION
  Technical basis:
   - https://github.com/phhusson/treble_experimentations/discussions/2542
   - https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus
  Verified design decisions (not guessed):
   - Target partition on P10 = recovery_ramdisk (NOT boot, NOT recovery)
   - Source file = RECOVERY_RAMDIS(K).img from UPDATE.APP of matching EMUI 9.1 full firmware
   - Example (NOT exclusive): VTR-L29 9.1.0.297(C432E5R1P9)
   - Magisk path: stock image -> Magisk "Select and Patch a File" -> fastboot flash recovery_ramdisk
   - Boot: Vol-Up + Power until Huawei logo (Magisk boot cheat, not persistent)
   - Huawei fastboot often answers FAILED (remote: Command not allowed) -> NOT proof of lock
   - GSI (system/vendor) is preserved, only recovery_ramdisk is touched
   - NEVER automatic: erase/format userdata, flashing unlock, bootloader unlock

  OS support (any OS, custom or not):
   - Stock EMUI 8.0 / 9.0 / 9.1, Harmony fake 9.1 base
   - TrebleDroid / Lineage GSI (bgN/bvS/bgS...), Android 10-14
   - PixelExperience / SuperiorOS / AOSP GSI, other custom ROMs
   - Detection via getprop classification, no guessing. GSI hides the Huawei base ->
     firmware baseline is determined assisted (user input + fastboot product + CUST).

  Language: repository and default UI are English; German UI if system language is German.

.PARAMETER Command
  CLI mode: detect|analyze|firmware|download|extract|patch|backup|flash|verify|restore|diagnostic|wizard|help
  Without Command the TUI starts.

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
  [switch]$Help,
  [string]$Mode = "safe",
  [string]$Goal = ""
)

$ErrorActionPreference = "Continue"
$TTVersion = "2.17.0"

function Get-TTScriptRoot {
  # Script directory under -File AND irm|iex. Never Split-Path $null:
  # every candidate is checked BEFORE splitting. $PSScriptRoot works inside
  # functions too (script scope); $MyInvocation there points at the function.
  if (-not [string]::IsNullOrEmpty($PSScriptRoot)) { return $PSScriptRoot }
  if (-not [string]::IsNullOrEmpty($PSCommandPath)) { return (Split-Path -Parent $PSCommandPath) }
  $inv = $MyInvocation.MyCommand.Path
  if (-not [string]::IsNullOrEmpty($inv)) { return (Split-Path -Parent $inv) }
  return (Get-Location).Path
}

# Self-bootstrap for remote single-file runs (irm|iex, temp download):
# without repo layout (no data/compatibility) fetch the FULL release ZIP
# (same trust root: this repo) and relaunch from it. Full run, not degraded.
# Tag order: own script version first (no network guesswork), then raw VERSION,
# API last (rate-limited). Guard TT_BOOTSTRAPPED=1 prevents loops.
# Offline -> degraded warning, no crash.
function Get-BootstrapReleaseFile {
  # Downloads+verifies+extracts the FULL release ZIP for $Tag. Returns the full
  # path of $RelPath inside it, or "" on any failure (caller narrates).
  param([string]$Tag, [string]$RelPath)
  try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $__w = New-Object Net.WebClient
    $__w.Headers.Add("User-Agent", "trebleManager")
    $__b = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "trebleManager"
    if (-not (Test-Path $__b)) { New-Item -ItemType Directory -Path $__b -Force | Out-Null }
    $__d = Join-Path $__b $Tag
    $__t = Join-Path $__d $RelPath
    if (-not (Test-Path $__t)) {
      $__z = Join-Path $__b ("trebleManager-" + $Tag + ".zip")
      $__w.DownloadFile("https://github.com/mleem97/trebleManager/releases/download/" + $Tag + "/trebleManager-" + $Tag + ".zip", $__z)
      try {
        $__e = (([string]$__w.DownloadString("https://github.com/mleem97/trebleManager/releases/download/" + $Tag + "/trebleManager-" + $Tag + ".zip.sha256") -split '\s+')[0]).Trim().ToUpper()
        $__a = (Get-FileHash $__z -Algorithm SHA256).Hash.ToUpper()
        if ($__e -ne "" -and $__e -ne $__a) { Remove-Item $__z -Force; throw "SHA256 MISMATCH, deleted, aborting bootstrap" }
        Write-Host "SHA256 OK." -ForegroundColor Green
      } catch {
        if ($_.Exception.Message -match "MISMATCH") { throw }
        Write-Host ("WARN: hash check skipped (" + $_.Exception.Message + ")") -ForegroundColor Yellow
      }
      if (-not (Test-Path $__d)) { New-Item -ItemType Directory -Path $__d -Force | Out-Null }
      Expand-Archive -Path $__z -DestinationPath $__d -Force
    }
    if (Test-Path $__t) { return $__t }
  } catch { Write-Host ("WARN: fetch " + $Tag + " failed (" + $_.Exception.Message + ")") -ForegroundColor Yellow }
  return ""
}
if (-not $env:TT_BOOTSTRAPPED) {
  $__root = Get-TTScriptRoot
  if ((Split-Path -Leaf $__root) -eq "scripts") { $__root = Split-Path -Parent $__root }
  if (-not (Test-Path (Join-Path $__root "data/compatibility"))) {
    Write-Host "Single-file run detected - fetching full toolkit layout ..." -ForegroundColor Cyan
    Write-Host "Einzeldatei erkannt - lade volles Toolkit-Layout ..." -ForegroundColor Cyan
    $__target = Get-BootstrapReleaseFile ("v" + $TTVersion) "scripts/Treble-Toolkit.ps1"
    if ([string]::IsNullOrEmpty($__target)) {
      try {
        $__wc2 = New-Object Net.WebClient
        $__wc2.Headers.Add("User-Agent", "trebleManager")
        $__tag2 = ([string](($__wc2.DownloadString("https://api.github.com/repos/mleem97/trebleManager/releases/latest") | ConvertFrom-Json).tag_name)).Trim()
        if (-not [string]::IsNullOrEmpty($__tag2)) { $__target = Get-BootstrapReleaseFile $__tag2 "scripts/Treble-Toolkit.ps1" }
      } catch { Write-Host ("WARN: API fallback failed (" + $_.Exception.Message + ")") -ForegroundColor Yellow }
    }
    if (-not [string]::IsNullOrEmpty($__target)) {
      $env:TT_BOOTSTRAPPED = "1"
      $__fw = @()
      if ($Command -ne "") { $__fw += $Command }
      foreach ($a in $args) { $__fw += $a }
      if ($Image -ne "") { $__fw += @("--image", $Image) }
      if ($FirmwareFile -ne "") { $__fw += @("--firmware-file", $FirmwareFile) }
      if ($Mode -ne "" -and $Mode -ne "safe") { $__fw += @("--mode", $Mode) }
      if ($Goal -ne "") { $__fw += @("--goal", $Goal) }
      if ($Json) { $__fw += "--json" }
      if ($Yes) { $__fw += "--yes" }
      if ($NoReboot) { $__fw += "--no-reboot" }
      if ($Anonymize) { $__fw += "--anonymize" }
      $__exe = [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
      & $__exe -NoProfile -ExecutionPolicy Bypass -File $__target @__fw
      exit $LASTEXITCODE
    }
    Write-Host "WARN: bootstrap failed - continuing degraded without registry." -ForegroundColor Yellow
  }
  Remove-Variable __root,__target,__fw,__exe,__wc2,__tag2 -ErrorAction SilentlyContinue
}

# Spec error cases (handled explicitly, SEARCHABLE):
# ADB not found / No device detected / USB debugging authorization required (ADB unauthorized) /
# Fastboot not found / Command not allowed (Huawei getvar refused, NOT auto locked) /
# Unsupported partition layout (recovery_ramdisk missing) / Unsupported model (wrong device) /
# Firmware mismatch (wrong firmware) / Cannot safely flash image (image not verifiable) /
# Patch failed (Magisk patching failed) / Flash failed (Fastboot flash failed) /
# Boot verification failed (Android won't boot -> offer restore).

# ============================================================ Language (repo is English; TUI follows system language)
$TTLang = "en"
try {
  $cult = [System.Globalization.CultureInfo]::CurrentUICulture.TwoLetterISOLanguageName
  if ($cult -eq "de") { $TTLang = "de" }
} catch {}
function L {
  # Bilingual UI helper: L "English" "Deutsch" -> German only if system language is German.
  param([string]$En, [string]$De)
  if ($TTLang -eq "de") { return $De }
  return $En
}
function Unquote-Path {
  # Drag-drop paths arrive quoted/with spaces: isolate safely for every system.
  param([string]$P)
  $t = ([string]$P).Trim()
  if ($t.Length -ge 2) {
    if (($t.StartsWith('"') -and $t.EndsWith('"')) -or ($t.StartsWith("'") -and $t.EndsWith("'"))) {
      $t = $t.Substring(1, $t.Length - 2).Trim()
    }
  }
  return $t
}

# ============================================================ Paths / state
function Get-TTToolRoot {
  $d = Get-TTScriptRoot
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
  Scrcpy    = $null
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
  InstalledRom = ""
  SkipFixOffer = $false
  FlashSystemPreset = ""
  FirmwareBaseline = ""
  FirmwareCompat   = $null
  StockImage  = ""
  StockHash   = $null
  PatchedImage = ""
  PatchedHash  = $null
  RootMethod   = "magisk-recovery"
  Compat       = $null
  CompatProfile = ""
  Storage      = "unknown"
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

function Invoke-TTSelfElevate {
  # Relaunch in a NEW elevated window unless already admin, opted out
  # (-NoElevateCheck), non-Windows, or remote (irm|iex has no file to relaunch).
  if ($NoElevateCheck) { return }
  if ([System.Environment]::OSVersion.Platform -ne "Win32NT") { return }
  try { if (Test-TTAdmin) { return } } catch { return }
  $sp = $MyInvocation.MyCommand.Path
  if ([string]::IsNullOrEmpty($sp)) { $sp = $PSCommandPath }
  if ([string]::IsNullOrEmpty($sp)) {
    Write-TTLog (L "Not admin and remote session: start an elevated shell and re-run for driver/fastboot work." "Kein Admin und Remote-Sitzung: erhoehtes Fenster oeffnen und neu starten fuer Treiber/Fastboot.") "WARNING"
    return
  }
  $argList = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$sp`"")
  if ($Command -ne "") { $argList += $Command }
  if ($Image -ne "") { $argList += @("-Image", "`"$Image`"") }
  if ($FirmwareFile -ne "") { $argList += @("-FirmwareFile", "`"$FirmwareFile`"") }
  if ($Json) { $argList += "-Json" }
  if ($Yes) { $argList += "-Yes" }
  if ($NoReboot) { $argList += "-NoReboot" }
  if ($Anonymize) { $argList += "-Anonymize" }
  if ($Mode -ne "" -and $Mode -ne "safe") { $argList += @("-Mode", $Mode) }
  foreach ($a in $args) { $argList += $a }
  try {
    $exe = (Get-Process -Id $PID).Path
  } catch { $exe = "powershell.exe" }
  Write-Host (L "Not admin - reopening elevated (UAC) ..." "Kein Admin - oeffne erhoeht neu (UAC) ...") -ForegroundColor Cyan
  try {
    Start-Process -FilePath $exe -ArgumentList $argList -Verb RunAs | Out-Null
    exit 0
  } catch {
    Write-TTLog (L "Elevation denied, continuing without admin (fastboot drivers may fail)." "Elevation abgelehnt, weiter ohne Admin (Fastboot-Treiber koennen scheitern).") "WARNING"
  }
}

# ============================================================ Device profiles (modular)
# Primary test device: plain Huawei P10 (VTR family - no Lite, no Plus).
# P10 Plus (VKY) profiles are supported but secondary (no test device).
# New Huawei device = add one block here, rest stays the same.
# Verified = method proven from Discussion #2542 + P10 wiki (flash allowed).
# Unverified = hypothesis only (analyze + export allowed, flash BLOCKED until verified).
# LineageOS note: a running LineageOS/GSI is a valid STARTING point (it hides the
# Huawei base, so the baseline is assisted), but the Magisk SOURCE stays the stock
# RECOVERY_RAMDISK.img - Lineage packages contain no Huawei recovery_ramdisk.
# Wiki: https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus
$DeviceProfiles = @{
  "VTR-L29" = @{
    Id = "VTR-L29"; Marketing = "Huawei P10"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $true; Variant = "Global market (UFS storage)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only images (e.g. *-arm64_bgN.img); slim builds if system partition is small"
    KnownGoodAdvisory = @(
      "VTR-L29 9.1.0.297(C432E5R1P9)",
      "VTR-L29 9.1.0.275(C432E2R1P9T8)"
    )
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release (Magisk boot cheat, not persistent)"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV (USER LOCK + BL LOCK, fastboot oem unlock <code>)"
    FirmwareFinder = "https://professorjtj.github.io/v2/ (HUAWEI FIRM FINDER V2)"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VTR-L09" = @{
    Id = "VTR-L09"; Marketing = "Huawei P10"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $true; Variant = "Europe (UFS storage)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only images; slim builds if system partition is small"
    KnownGoodAdvisory = @("VTR-L09 EMUI 9.1 with matching CUST (C432/C185/...) - region must match device")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VKY-L29" = @{
    Id = "VKY-L29"; Marketing = "Huawei P10 Plus"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $true; Variant = "Global market (UFS storage)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only images; slim builds if system partition is small"
    KnownGoodAdvisory = @("VKY-L29 EMUI 9.1 with matching CUST")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VTR-AL00" = @{
    Id = "VTR-AL00"; Marketing = "Huawei P10"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $false; Variant = "China, no SIM restriction (eMMC or UFS storage - check!)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only; CN units with eMMC behave differently - see wiki storage note"
    KnownGoodAdvisory = @("VTR-AL00 EMUI 9.1 with matching CUST (C00/...) - UNVERIFIED, submit device data first")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VKY-L09" = @{
    Id = "VKY-L09"; Marketing = "Huawei P10 Plus"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $false; Variant = "Europe (UFS storage)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only images; slim builds if system partition is small"
    KnownGoodAdvisory = @("VKY-L09 EMUI 9.1 with matching CUST - UNVERIFIED, submit device data first")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VTR-TL00" = @{
    Id = "VTR-TL00"; Marketing = "Huawei P10"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $false; Variant = "China Mobile customized (eMMC or UFS storage - check!)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only; carrier-customized CN unit - check CUST + storage chip, see wiki"
    KnownGoodAdvisory = @("VTR-TL00 EMUI 9.1 with matching CUST - UNVERIFIED, submit device data first")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VKY-AL00" = @{
    Id = "VKY-AL00"; Marketing = "Huawei P10 Plus"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $false; Variant = "China, no SIM restriction (eMMC or UFS storage - check!)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only; CN units with eMMC behave differently - see wiki storage note"
    KnownGoodAdvisory = @("VKY-AL00 EMUI 9.1 with matching CUST (C00/...) - UNVERIFIED, submit device data first")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "VKY-TL00" = @{
    Id = "VKY-TL00"; Marketing = "Huawei P10 Plus"; Arch = "arm64"; SoC = "Kirin 960"
    Verified = $false; Variant = "China Mobile customized (eMMC or UFS storage - check!)"
    TargetPartition = "recovery_ramdisk"
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @("RECOVERY_RAMDISK.img","RECOVERY_RAMDIS.img","recovery_ramdisk.img")
    EmuiRequired = "9.1"
    GsiAdvice = "arm64 A-only; carrier-customized CN unit - check CUST + storage chip, see wiki"
    KnownGoodAdvisory = @("VKY-TL00 EMUI 9.1 with matching CUST - UNVERIFIED, submit device data first")
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions/2542"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"
    BootKeys = "Vol-Up + Power until Huawei logo, then release"
    UnlockTool = "https://github.com/mashed-potatoes/PotatoNV"
    FirmwareFinder = "https://professorjtj.github.io/v2/"
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
  "GENERIC-TREBLE" = @{
    Id = "GENERIC-TREBLE"; Marketing = "Generic Treble device"; Arch = "arm64"; SoC = "unknown"
    Verified = $false; Variant = "Fallback for non-Huawei Treble devices (analyze only)"
    TargetPartition = ""
    ForbiddenPartitions = @("boot","recovery","system","vendor","userdata")
    StockFileNames = @()
    EmuiRequired = ""
    GsiAdvice = "No verified method - analysis and recovery export only"
    KnownGoodAdvisory = @()
    Discuss = "https://github.com/phhusson/treble_experimentations/discussions"
    Wiki = "https://github.com/phhusson/treble_experimentations/wiki"
    BootKeys = ""
    UnlockTool = ""
    FirmwareFinder = ""
    MagiskGuide = "https://topjohnwu.github.io/Magisk/install.html"
  }
}

# Root methods in priority order. Magisk patched recovery_ramdisk is PREFERRED.
# TWRP shares the SAME partition slot (recovery_ramdisk) - Magisk slot and TWRP slot
# overwrite each other; restore switches back. Never flash both without a backup.
$RootMethods = @(
  @{ Id = "magisk-recovery"; Name = "Magisk patched recovery_ramdisk (PREFERRED)"; Preferred = $true;
     Needs = "Stock RECOVERY_RAMDISK.img + Magisk app (on-device patch)";
     WorksOn = "VTR-L29/L09, VKY-L29 (verified); any OS (stock/GSI/custom)";
     Notes = "Vol-Up + Power boot cheat. Modules + systemless hosts (AdAway) supported." }
  @{ Id = "magisk-twrp"; Name = "Magisk via TWRP zip (alternative)"; Preferred = $false;
     Needs = "TWRP in recovery_ramdisk slot + Magisk APK/zip flashed from TWRP";
     WorksOn = "Same verified profiles; needs working TWRP build for the device";
     Notes = "Overwrites the Magisk-recovery slot. Keep a backup to switch back." }
  @{ Id = "phh-su"; Name = "phh superuser (legacy, no modules)"; Preferred = $false;
     Needs = "phh-su capable GSI (F-Droid me.phh.superuser)";
     WorksOn = "Treble GSIs with su support; per #2542 comments enough for AdAway";
     Notes = "No Magisk modules, no systemless hosts. Boot scripts tricky (apTouch starts late)." }
  @{ Id = "kernelsu"; Name = "KernelSU (experimental)"; Preferred = $false;
     Needs = "KernelSU-patched kernel (Pangu-based for EMUI9)";
     WorksOn = "Huawei Kirin: v0.9.2 ONLY, not v0.9.5+ (per wiki)";
     Notes = "Kernel replacement = higher risk. Only for experts with full backup." }
)

function Get-PreferredRootMethod {
  # Pure, unit-testable: returns methods with preferred first.
  $p = $RootMethods | Where-Object { $_.Preferred -eq $true } | Select-Object -First 1
  $rest = $RootMethods | Where-Object { $_.Preferred -ne $true }
  return @($p) + @($rest)
}

# TWRP knowledge (P10 wiki + XDA). TWRP lives in the recovery_ramdisk slot on Kirin.
$TwrpKnowledge = @{
  Sources = @(
    "XDA P10 Plus TWRP 3.2.1-0 (oreo): forum.xda-developers.com/p10-plus/development/recovery-twrp-3-2-1-0-oreo-t3734993",
    "Only use a TWRP built for YOUR exact model (VTR vs VKY differ)."
  )
  Rules = @(
    "TWRP and Magisk-recovery share ONE slot (recovery_ramdisk) - flashing one overwrites the other.",
    "Back up the current slot first (tool does it automatically).",
    "NEVER factory-reset userdata from TWRP on this device - breaks internal storage (use stock recovery).",
    "Boot TWRP: Vol-Up held until loaded. Leave system unmodified when TWRP asks.",
    "Magisk from TWRP: flash Magisk zip, then boot with Vol-Up + Power for rooted system."
  )
}
$WikiKnowledge = @{
  UnlockSteps = @(
    "Huawei locks TWO levels: USER LOCK (kernel/ODM/product) and BL LOCK (system/boot/recovery/userdata).",
    "1. PotatoNV testpoint method per its official tutorial: https://github.com/mashed-potatoes/PotatoNV",
    "2. Boot the engineering image to special fastboot, choose 'Disable FBLock' (unlocks USER LOCK).",
    "3. Reboot to normal fastboot (shows unlocked, but NOT fully). PotatoNV shows a random 16-digit code.",
    "4. Run: fastboot oem unlock XXXXXXXXXXXXXXXX  (your code) - now fully unlocked.",
    "This tool NEVER runs unlock commands itself."
  )
  KernelNotes = @(
    "Some GSIs need permissive SELinux -> custom kernel required (stock kernel enforces).",
    "EMUI 8: Proto8 (all P10) or HyperPlus (EU/Global, or CN only with UFS chip).",
    "EMUI 9 CN: Pangu kernel (maimaiguanfan/android_kernel_huawei_hi3660).",
    "KernelSU: Huawei supports v0.9.2 only, NOT v0.9.5+."
  )
  InstallRules = @(
    "Base: stock EMUI 8.0/9.0/9.1 (GSI needs matching vendor).",
    "Back up internal storage first. Factory reset ONLY via stock recovery (never TWRP wipe - breaks userdata).",
    "Flash GSI: fastboot flash system <gsi.img>, then eRecovery wipe data/factory reset.",
    "System partition can be too small: use slim GSI builds (or TWRP+Parted resize, expert only).",
    "GApps: MindTheGapps (OpenGApps fails detection on Oreo GSIs; Pie+ OpenGApps works)."
  )
  Android13Warning = "Android 13 on P10/P10 Plus is unstable per wiki: no SIM/signal possible, hardware may fail or not boot. Android 10 (Q) GSIs are the recommended daily drivers on Kirin 960."
  Fixes = @(
    "Speakers: as root per boot: chown root:audio /dev/nxp_smartpa_dev; chmod 0660 /dev/nxp_smartpa_dev (fixed since AOSP 12 v400.e).",
    "Touchscreen edges: as root per boot: stop aptouch (or GSI_Generic_Fix Magisk module).",
    "Decrypt: needs EMUI 9.0 vendor fstab (9.1 is read-only erofs) - see dfe-neo-v2 tool."
  )
}

$MagiskCompatTable = @(
  @{ Android = "8.x (EMUI 8)"; Tested = "Magisk v25+"; Note = "Wiki: v25+ ok on EMUI8/9 base. Check changelog." }
  @{ Android = "9.x (EMUI 9/9.1)"; Tested = "Magisk v25+"; Note = "Wiki explicit. No blind latest." }
  @{ Android = "10-12 (GSI/custom)"; Tested = "Magisk v26+"; Note = "Check compatibility per release (topjohnwu releases)." }
  @{ Android = "13-14 (TrebleDroid/Lineage GSI)"; Tested = "Magisk v27/v28+ per changelog"; Note = "Current: lineage_arm64_bgN A13 works; document Magisk version + SHA-256." }
)

# ============================================================ Firmware sources (curated, honest)
# No direct stock ZIP is guaranteed officially available in 2026. Hence: show sources,
# user confirms download explicitly, then verify hash/size. No dubious auto-download.
$FirmwareSources = @(
  @{ Id = "hisuite"; Name = "Huawei HiSuite (official, recommended for re-install)"; NameDe = "Huawei HiSuite (offiziell, empfohlen f. Re-Install)"; Url = "https://consumer.huawei.com/de/support/hisuite/"; Kind = "official"; Note = "No direct ZIP. Restore stock via USB or save UPDATE.APP from HiSuite cache."; NoteDe = "Kein Direkt-ZIP. Stock per USB zurueckspielen oder UPDATE.APP aus HiSuite-Cache sichern." }
  @{ Id = "consumer"; Name = "Huawei Consumer Support (official search)"; NameDe = "Huawei Consumer Support (offizielle Suche)"; Url = "https://consumer.huawei.com/de/support/"; Kind = "official"; Note = "Search model VTR-L29 manually. Old EMUI 9.1 packages partly removed."; NoteDe = "Modell VTR-L29 manuell suchen. Alte EMUI-9.1-Pakete teils entfernt." }
  @{ Id = "firmfinder"; Name = "HUAWEI FIRM FINDER V2 (historic proxy)"; NameDe = "HUAWEI FIRM FINDER V2 (historischer Proxy)"; Url = "https://professorjtj.github.io/v2/"; Kind = "proxy-historic"; Note = "Former Huawei-cloud proxy. Check status, may be offline. Research only."; NoteDe = "Frueher Huawei-Cloud-Proxy. Status pruefen, evtl. offline. Nur als Recherche." }
  @{ Id = "androidhost"; Name = "androidhost.ru Huawei archive (community, XDA-referenced)"; NameDe = "androidhost.ru Huawei-Archiv (Community, XDA-referenziert)"; Url = "https://androidhost.ru/search.html?search=VTR-L29"; Kind = "community-archive"; Note = "Established community archive for full OTA (UPDATE.APP). Pick direct link there, paste here, verify hash."; NoteDe = "Etabliertes Community-Archiv f. Full-OTA (UPDATE.APP). Direktlink dort waehlen, hier einfuegen, Hash pruefen." }
  @{ Id = "wiki"; Name = "phhusson wiki/discussion (docs, no download)"; NameDe = "phhusson Wiki/Discussion (Doku, kein Download)"; Url = "https://github.com/phhusson/treble_experimentations/wiki/Huawei-P10-and-P10-Plus"; Kind = "docs"; Note = "Reference for method/partition. Example firmware VTR-L29 9.1.0.297(C432E5R1P9) is NOT exclusive."; NoteDe = "Referenz f. Methode/Partition. Beispiel-Firmware VTR-L29 9.1.0.297(C432E5R1P9) ist NICHT exklusiv." }
  @{ Id = "custom"; Name = "Paste own URL (full firmware ZIP)"; NameDe = "Eigene URL einfuegen (Full-Firmware ZIP)"; Url = ""; Kind = "user"; Note = "Only http(s), only your own verified source. Validated + hashed."; NoteDe = "Nur http(s), nur eigene gepruefte Quelle. Wird validiert + gehasht." }
)

function Test-FirmwareUrl {
  # Pure, unit-testable. Allows: http/https, firmware archive types, max 2048 chars.
  param([string]$Url)
  if ([string]::IsNullOrWhiteSpace($Url)) { return @{ Ok = $false; Reason = (L "URL empty." "URL leer.") } }
  $u = $Url.Trim()
  if ($u.Length -gt 2048) { return @{ Ok = $false; Reason = (L "URL too long." "URL zu lang.") } }
  if (-not ($u -match "^https?://")) { return @{ Ok = $false; Reason = (L "Only http(s) allowed (no ftp/file/javascript)." "Nur http(s) erlaubt (kein ftp/file/javascript).") } }
  try { $uri = New-Object System.Uri($u) } catch { return @{ Ok = $false; Reason = (L "URL invalid." "URL ungueltig.") } }
  if ([string]::IsNullOrEmpty($uri.Host)) { return @{ Ok = $false; Reason = (L "Host missing." "Host fehlt.") } }
  if ($u -match "@" -and $u -match "://[^/]*:.*@") { return @{ Ok = $false; Reason = (L "No credentials in URL." "Keine Credentials in URL.") } }
  $low = $u.ToLower().Split("?")[0]
  if ($low.EndsWith("/download")) { $low = $low.Substring(0, $low.Length - 9) }  # SourceForge direct links
  $allowed = @(".zip",".7z",".tar",".gz",".tgz",".xz",".app",".rar",".img")
  $hit = $false
  foreach ($e in $allowed) { if ($low.EndsWith($e)) { $hit = $true } }
  if (-not $hit) { return @{ Ok = $false; Reason = (L "File must be a firmware/ROM archive (zip/7z/tar/gz/app/rar/img)." "Dateityp muss Firmware/ROM-Archiv sein (zip/7z/tar/gz/app/rar/img).") } }
  return @{ Ok = $true; Reason = "OK"; Host = $uri.Host }
}

function Invoke-FirmwareDownload {
  # BITS (with progress) preferred, WebClient+Write-Progress as fallback. Read-only except target file.
  param([string]$Url, [string]$OutFile)
  $chk = Test-FirmwareUrl $Url
  if (-not $chk.Ok) { Write-TTLog ((L "URL rejected: " "URL abgelehnt: ") + $chk.Reason) "ERROR"; return @{ Ok = $false; Path = "" } }
  $dir = Split-Path -Parent $OutFile
  if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
  if (Test-Path $OutFile) {
    Write-TTLog ((L "Target file already exists (offline cache, no re-download): " "Zieldatei existiert bereits (Offline-Cache, kein Re-Download): ") + $OutFile) "WARNING"
    return @{ Ok = $true; Path = $OutFile; Cached = $true }
  }
  Write-TTLog ((L "Download starting: " "Download startet: ") + $Url) "INFO"
  Write-TTLog ((L "Target: " "Ziel: ") + $OutFile) "INFO"
  # --- Attempt 1: BITS (Windows, resume + progress) ---
  try {
    $bits = Get-Command Start-BitsTransfer -ErrorAction SilentlyContinue
    if ($bits) {
      Write-Host (L "Downloading via BITS (progress in BITS manager + below) ..." "Download via BITS (Fortschritt im BITS-Manager + unten) ...") -ForegroundColor Cyan
      Start-BitsTransfer -Source $Url -Destination $OutFile -DisplayName "TrebleToolkit Firmware" -Description $Url -ErrorAction Stop | Out-Null
      if (Test-Path $OutFile) {
        $mb = [math]::Round((Get-Item $OutFile).Length/1MB,1)
        Write-Progress -Activity "Firmware download" -Completed
        Write-TTLog ((L "BITS download done (" "BITS-Download fertig (") + $mb + " MB).") "SUCCESS"
        return @{ Ok = $true; Path = $OutFile }
      }
    }
  } catch { Write-TTLog ((L "BITS failed, WebClient fallback: " "BITS fehlgeschlagen, Fallback WebClient: ") + $_.Exception.Message) "WARNING" }
  # --- Attempt 2: WebClient with visual progress ---
  try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $wc = New-Object System.Net.WebClient
    $progId = Get-Random -Minimum 1000 -Maximum 9999
    Register-ObjectEvent -InputObject $wc -EventName DownloadProgressChanged -SourceIdentifier ("TTDL" + $progId) -Action {
      $p = $Event.SourceEventArgs.ProgressPercentage
      $rec = $Event.SourceEventArgs.BytesReceived
      $tot = $Event.SourceEventArgs.TotalBytesToReceive
      $mbR = [math]::Round($rec/1MB,1); $mbT = [math]::Round($tot/1MB,1)
      Write-Progress -Activity "Firmware download (stock, original)" -Status "$mbR / $mbT MB ($p %)" -PercentComplete $p
    } | Out-Null
    Register-ObjectEvent -InputObject $wc -EventName DownloadFileCompleted -SourceIdentifier ("TTDONE" + $progId) -Action {
      Write-Progress -Activity "Firmware download" -Completed
    } | Out-Null
    Write-Host (L "Downloading ... (bar + MB display, abort: Ctrl+C)" "Lade ... (Balken + MB-Anzeige, Abbruch: Strg+C)") -ForegroundColor Cyan
    $task = $wc.DownloadFileTaskAsync($Url, $OutFile)
    while (-not $task.IsCompleted) { Start-Sleep -Milliseconds 300 }
    Unregister-Event -SourceIdentifier ("TTDL" + $progId) -ErrorAction SilentlyContinue
    Unregister-Event -SourceIdentifier ("TTDONE" + $progId) -ErrorAction SilentlyContinue
    $wc.Dispose()
    if ($task.IsFaulted) { throw $task.Exception }
    if (Test-Path $OutFile) {
      $mb = [math]::Round((Get-Item $OutFile).Length/1MB,1)
      Write-TTLog ((L "Download done (" "Download fertig (") + $mb + " MB).") "SUCCESS"
      return @{ Ok = $true; Path = $OutFile }
    }
  } catch {
    Write-Progress -Activity "Firmware download" -Completed
    Write-TTLog ((L "Download failed: " "Download fehlgeschlagen: ") + $_.Exception.Message) "ERROR"
    if (Test-Path $OutFile) { Remove-Item $OutFile -Force -ErrorAction SilentlyContinue }
    return @{ Ok = $false; Path = "" }
  }
  return @{ Ok = $false; Path = "" }
}

function Test-DownloadedFirmware {
  param([string]$Path)
  if (-not (Test-Path $Path)) { return @{ Ok = $false; Note = (L "File missing." "Datei fehlt.") } }
  $info = Get-FileHashInfo $Path
  $mb = [math]::Round($info.Size/1MB,1)
  $notes = @((L "Size: " "Groesse: ") + "$mb MB")
  $ok = $true
  if ($info.Size -lt 100MB) { $ok = $false; $notes += (L "WARN: < 100 MB - implausibly small for full firmware (maybe delta/short package)." "WARN: < 100 MB - unplausibel klein fuer Full-Firmware (evtl. nur OTA-Delta/verkuerzt).") }
  $shaFile = $Path + ".sha256"
  $info.SHA256 | Out-File -FilePath $shaFile -Encoding ascii
  $notes += (L "SHA-256 saved in " "SHA-256 in ") + "$shaFile."
  # Check ZIP content for UPDATE.APP (if ZIP)
  if ($Path.ToLower().EndsWith(".zip")) {
    try {
      Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue | Out-Null
      $zip = [System.IO.Compression.ZipFile]::OpenRead($Path)
      $names = @()
      foreach ($e in $zip.Entries) { $names += $e.FullName }
      $zip.Dispose()
      $hasApp = ($names | Where-Object { $_ -like "*UPDATE.APP" }).Count -gt 0
      if ($hasApp) { $notes += (L "UPDATE.APP found inside ZIP." "UPDATE.APP im ZIP enthalten.") }
      else { $notes += (L "WARN: no UPDATE.APP in ZIP - maybe wrong package/region." "WARN: keine UPDATE.APP im ZIP gefunden - evtl. falsches Paket/Region."); $ok = $false }
    } catch { $notes += (L "ZIP check not possible: " "ZIP-Check nicht moeglich: ") + $_.Exception.Message }
  } else { $notes += (L "Not a ZIP - unpack / UPDATE.APP search manually, then TUI step 4." "Kein ZIP - Entpacken/UPDATE.APP-Suche manuell, dann TUI Step 4.") }
  return @{ Ok = $ok; SHA256 = $info.SHA256; Size = $info.Size; Notes = $notes }
}

# ============================================================ Pure parsers (unit-testable)
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
    # Form: recovery_ramdisk -> /dev/block/mmcblk0pXX  or  lrwxrwxrwx ... recovery_ramdisk -> ...
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
  if ($isGsi) { $isStock = $false }  # GSI wins (hides Huawei base)

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
  if ($kind -like "*GSI*") { $notes += "GSI hides Huawei firmware base -> determine baseline assisted." }
  if ($kind -like "Stock*") { $notes += "Stock: EMUI version directly readable, baseline verifiable." }
  if (-not $supported) { $notes += "Android release '$release' untested (8-14 supported)." }

  return @{
    Kind = $kind; Detail = "$model / $pname / Android $release ($display)"
    AndroidRelease = $release; GsiVariant = $variant
    IsStock = $isStock; IsGsi = $isGsi; SupportedAndroid = $supported
    Emui = $emui; Notes = $notes
  }
}

function Get-CompatRegistry {
  # Loads data/compatibility/huawei/p10/<profile>.json (generated from .yaml). Cached.
  if ($TT.Compat -ne $null -and $TT.CompatProfile -eq $TT.ProfileId) { return $TT.Compat }
  $TT.Compat = $null; $TT.CompatProfile = $TT.ProfileId
  $f = Join-Path $TTRoot ("data/compatibility/huawei/p10/" + $TT.ProfileId + ".json")
  if (-not (Test-Path $f)) { return $null }
  try {
    $TT.Compat = Get-Content $f -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
  } catch {
    Write-TTLog ((L "Registry unreadable: " "Registry unlesbar: ") + $f) "ERROR"
    return $null
  }
  return $TT.Compat
}

function Test-RomAgainstRegistry {
  # Pure logic on registry object + filename: returns @{ Status; Reasons }.
  # Researched BROKEN builds block (not warn) - brick protection.
  param($Reg, [string]$FileName)
  $low = $FileName.ToLower()
  if ($Reg -eq $null) { return @{ Status = "WARN"; Reasons = @("No registry for this profile - cannot cross-check ROM.") } }
  foreach ($r in $Reg.roms) {
    if ($r.status -ne "broken") { continue }
    $hit = $false
    foreach ($m in @($r.markers)) { if ($m -ne $null -and $low.Contains([string]$m.ToLower())) { $hit = $true } }
    if (-not $hit) {
      $rn = [string]$r.name
      if ($rn -ne "" -and $low.Contains($rn.ToLower().Split(" ")[0])) {
        # Name alone is not enough (e.g. Lineage vs Lineage Light) - need marker too.
      }
    }
    if ($hit) {
      return @{ Status = "FAIL"; Reasons = @("BLOCKED: '" + $r.name + "' is researched BROKEN (" + $r.reason + ").") }
    }
  }
  return @{ Status = "PASS"; Reasons = @("No researched-broken markers matched.") }
}

function Get-VendorAdvice {
  # EMUI base -> vendor generation -> boot expectations (TrebleDroid matrix).
  param([string]$Emui, [string]$TargetAndroid)
  if ([string]::IsNullOrWhiteSpace($Emui)) { return "Vendor base unknown (GSI hides it) - assume nothing about Q/R/S boot."
  }
  if ($Emui -match "8\.") {
    return "Oreo vendor: Q/8.1 problematic, P good. Target Android $TargetAndroid on Oreo vendor = RISK (see registry vendor_boot)."
  }
  if ($Emui -match "9\.") {
    return "Pie vendor: Q/R/S boot. Target Android $TargetAndroid expected to boot."
  }
  return "Vendor base '$Emui' unclassified - verify manually."
}

# ============================================================ Installed ROM (what is ON the phone)
function Get-RomSuggested {
  # Pure: OS class -> suggested ROM id ("stock" for EMUI, "" = user must pick).
  # Never guesses a custom ROM - the user states it.
  param([string]$OsKind)
  if ([string]::IsNullOrWhiteSpace($OsKind)) { return "" }
  if ($OsKind -like "*Stock*" -or $OsKind -like "*EMUI*") { return "stock" }
  return ""
}

function Get-RomLabel {
  # Pure: ROM id -> short display name.
  param([string]$Id)
  if ([string]::IsNullOrWhiteSpace($Id)) { return "?" }
  if ($Id -eq "stock") { return "Stock EMUI" }
  if ($Id -eq "other") { return "Other custom ROM" }
  if ($Id.StartsWith("rom:")) { return $Id.Substring(4) }
  return $Id
}

function Get-RomEntryLabel {
  # Pure: registry ROM entry -> the same label Get-RomOptions shows.
  param($Entry)
  $nm = [string]$Entry.name
  if ([string]::IsNullOrWhiteSpace($nm)) { return "" }
  $ver = [string]$Entry.version; if ($ver -eq "") { $ver = [string]$Entry.android }
  if ($ver -eq "") { $ver = [string]$Entry.build }
  $label = $nm; if ($ver -ne "") { $label += " " + $ver }
  $variant = [string]$Entry.variant
  if ($variant -ne "" -and -not $label.Contains($variant)) { $label += " " + $variant }
  $bld = [string]$Entry.build
  if ($bld -ne "" -and -not $label.Contains($bld)) { $label += " (" + $bld + ")" }
  return $label
}

function Get-RomEntry {
  # Registry entry for an installed ROM id (or $null for stock/other/unknown).
  param([string]$Id)
  if ([string]::IsNullOrWhiteSpace($Id) -or $Id -eq "stock" -or $Id -eq "other") { return $null }
  $want = $Id
  if ($want.StartsWith("rom:")) { $want = $want.Substring(4) }
  $reg = Get-CompatRegistry
  if ($reg -eq $null) { return $null }
  foreach ($r in $reg.roms) {
    if ((Get-RomEntryLabel $r) -eq $want) { return $r }
  }
  return $null
}

function Get-RomOptions {
  # Selectable systems: stock first, supported registry ROMs, then other.
  $opts = @(@{ Id = "stock"; Label = "Stock EMUI (Huawei original)"; Status = "supported" })
  $reg = Get-CompatRegistry
  if ($reg -ne $null) {
    foreach ($r in $reg.roms) {
      $label = Get-RomEntryLabel $r
      if ([string]::IsNullOrWhiteSpace($label)) { continue }
      $st = [string]$r.status
      if ($st -eq "working" -or $st -eq "working-slim" -or $st -eq "working-with-fixes" -or $st -eq "variant-dependent") {
        $id = "rom:" + $label
        if (-not ($opts | Where-Object { $_.Id -eq $id })) { $opts += @(@{ Id = $id; Label = $label; Status = $st }) }
      }
    }
  }
  $opts += @(@{ Id = "other"; Label = "Other custom ROM (not in list)"; Status = "custom" })
  return $opts
}

function Get-InstalledRomFile { return (Join-Path $TTRoot "data/installed-rom.txt") }

function Save-InstalledRom {
  param([string]$Id)
  try {
    $f = Get-InstalledRomFile
    $d = Split-Path -Parent $f
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
    ([string]$Id).Trim() | Out-File -FilePath $f -Encoding ascii -NoNewline
    $TT.InstalledRom = ([string]$Id).Trim()
  } catch {}
}

function Load-InstalledRom {
  try {
    $f = Get-InstalledRomFile
    if (Test-Path $f) {
      $v = ([string](Get-Content $f -Raw -ErrorAction Stop)).Trim()
      if ($v -ne "") { $TT.InstalledRom = $v }
    }
  } catch {}
  return $TT.InstalledRom
}

function Select-InstalledRom {
  # THE question: which system is on the phone. Plain words, persisted answer.
  $reg = Get-CompatRegistry
  $opts = Get-RomOptions
  Show-TTHeader (L "Which system is on your phone right now?" "Welches System ist gerade auf deinem Handy?")
  Write-Host ""
  Write-Host (L "This decides everything: patch source, skipped steps, blocked ROMs." "Das entscheidet alles: Patch-Quelle, uebersprungene Steps, blockierte ROMs.") -ForegroundColor Cyan
  Write-Host ""
  $suggest = Get-RomSuggested ([string]$TT.OS.Kind)
  for ($i = 0; $i -lt $opts.Count; $i++) {
    $mark = ""
    if ($opts[$i].Id -eq $TT.InstalledRom) { $mark = (L "  <-- current, Enter keeps it" "  <-- aktuell, Enter behaelt es") }
    elseif ($TT.InstalledRom -eq "" -and $opts[$i].Id -eq $suggest) { $mark = (L "  <-- detected, Enter selects it" "  <-- erkannt, Enter waehlt es") }
    Write-Host (" [" + ($i+1) + "] " + $opts[$i].Label + $mark) -ForegroundColor White
  }
  if ($reg -ne $null) {
    $broken = @($reg.roms | Where-Object { $_.status -eq "broken" })
    if ($broken.Count -gt 0) {
      Write-Host ""
      Write-Host (L "NOT supported (researched broken - cannot be selected):" "NICHT supported (nachweislich kaputt - nicht waehlbar):") -ForegroundColor Red
      foreach ($b in $broken) { Write-Host ("  X " + $b.name + " - " + $b.reason) -ForegroundColor Red }
    }
  }
  Write-Host ""
  Write-Host (L "Number + Enter (Enter = keep): " "Nummer + Enter (Enter = behalten): ") -NoNewline -ForegroundColor Yellow
  $s = Read-Host
  if ($s -match "^\d+$") {
    $idx = [int]$s - 1
    if ($idx -ge 0 -and $idx -lt $opts.Count) { Save-InstalledRom $opts[$idx].Id }
  } elseif ($TT.InstalledRom -eq "" -and $suggest -ne "") { Save-InstalledRom $suggest }
  $label = Get-RomLabel $TT.InstalledRom
  Write-Host ""
  Write-Host ((L "Phone runs: " "Handy laeuft mit: ") + $label) -ForegroundColor Green
  if ($TT.InstalledRom -eq "" -or $TT.InstalledRom -eq "stock") {
    Write-Host (L "Rule: patch base = stock UPDATE.APP recovery image. Nothing else." "Regel: Patch-Basis = Stock-UPDATE.APP-Recovery. Nichts anderes.") -ForegroundColor White
    return $TT.InstalledRom
  }
  $entry = Get-RomEntry $TT.InstalledRom
  if ($entry -ne $null -and -not [string]::IsNullOrWhiteSpace([string]$entry.gsi)) {
    Write-Host (L "GSI (system-only): your recovery is untouched stock, so the patch base IS the stock recovery. Correct, not a workaround." "GSI (nur System): dein Recovery ist unberuehrt Stock, also ist die Patch-Basis das Stock-Recovery. Korrekt, kein Workaround.") -ForegroundColor Green
    return $TT.InstalledRom
  }
  Write-Host (L "RULE: your Magisk patch file MUST come from this ROM package." "REGEL: Deine Magisk-Patch-Datei MUSS aus diesem ROM-Paket kommen.") -ForegroundColor Red
  Write-Host (L "NOT from stock firmware. A stock-based patched image will NOT boot on this ROM." "NICHT aus der Stock-Firmware. Ein Stock-basiertes Image bootet auf diesem ROM NICHT.") -ForegroundColor Red
  return $TT.InstalledRom
}

function Find-TTRomBaseImage {
  # Exported base image from the installed custom ROM's package (data/recovery/*).
  $dir = Join-Path $TTRoot "data/recovery"
  if (-not (Test-Path $dir)) { return "" }
  $hits = Get-ChildItem -Path $dir -Recurse -File -Include "*.img" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
  if ($hits -and $hits.Count -gt 0) { return $hits[0].FullName }
  return ""
}

function Get-PatchBase {
  # Single source of truth: which image gets Magisk-patched.
  # - stock -> stock recovery.
  # - GSI ROM (system-only, registry has a gsi field) -> STOCK recovery too:
  #   a GSI never touches recovery_ramdisk, so the partition is still stock.
  # - full device ROM (own boot/recovery) -> exported image from its package.
  $rom = $TT.InstalledRom
  if ([string]::IsNullOrWhiteSpace($rom) -or $rom -eq "stock") {
    return @{ Image = $TT.StockImage; Hash = $TT.StockHash; Source = "stock"; RomLabel = "Stock EMUI" }
  }
  $entry = Get-RomEntry $rom
  if ($entry -ne $null -and -not [string]::IsNullOrWhiteSpace([string]$entry.gsi)) {
    return @{ Image = $TT.StockImage; Hash = $TT.StockHash; Source = "stock-gsi"; RomLabel = (Get-RomLabel $rom) }
  }
  $img = Find-TTRomBaseImage
  $h = $null
  if ($img -ne "" -and (Test-Path $img)) { $h = Get-FileHashInfo $img }
  return @{ Image = $img; Hash = $h; Source = "rom"; RomLabel = (Get-RomLabel $rom) }
}

function Resolve-RunMode {
  # Pure, unit-testable. safe = confirm everything; unattended = --yes auto-confirms
  # (gates still enforced); developer = unlocks dump-* commands.
  param([string]$Name)
  $n = ([string]$Name).ToLower().Trim()
  if ($n -eq "unattended" -or $n -eq "developer") { return $n }
  return "safe"
}
$TTMode = Resolve-RunMode $Mode
$TTMode = Resolve-RunMode $Mode

function Invoke-TTValidate {
  # Post-flash system/hardware validation (read-only). Returns report object.
  $items = @()
  $add = { param($Name, $Pass, $Detail)
    $script:__v += @(New-Object PSObject -Property @{ name = $Name; pass = [bool]$Pass; detail = [string]$Detail })
  }
  $script:__v = @()
  if ($TT.Mode -ne "android") {
    Update-TTMode | Out-Null
    if ($TT.Mode -ne "android") {
      & $add "ADB" $false "no android device"
      return @{ items = $script:__v; ok = $false }
    }
  }
  & $add "ADB" $true $TT.AdbSerial
  $rel = Get-Prop "ro.build.version.release"
  $disp = Get-Prop "ro.build.display.id"
  & $add "OS" ($rel -ne "") "$rel / $disp"
  $se = (Invoke-TTAdb @("shell","getenforce") -join "").Trim()
  & $add "SELinux" ($se -ne "") $se
  $idOut = (Invoke-TTAdb @("shell","su -c id 2>&1") -join "").Trim()
  & $add "ROOT" ($idOut -match "uid=0") $idOut
  $mnt = (Invoke-TTAdb @("shell","mount 2>&1") -join "`n")
  & $add "MOUNTS-system" ($mnt -match "/system") "system mounted"
  & $add "MOUNTS-vendor" ($mnt -match "/vendor") "vendor mounted"
  $wifi = (Invoke-TTAdb @("shell","dumpsys wifi 2>&1 | grep -i -m1 'Wi-Fi is'") -join "").Trim()
  & $add "WIFI" ($wifi -ne "") $wifi
  $bt = (Invoke-TTAdb @("shell","settings get global bluetooth_on 2>&1") -join "").Trim()
  & $add "BLUETOOTH" ($bt -eq "1" -or $bt -eq "0") "state=$bt"
  $bat = (Invoke-TTAdb @("shell","dumpsys battery 2>&1 | grep -i -m1 level") -join "").Trim()
  & $add "BATTERY" ($bat -ne "") $bat
  $sens = (Invoke-TTAdb @("shell","dumpsys sensorservice 2>&1 | grep -c -i sensor") -join "").Trim()
  & $add "SENSORS" ($sens -ne "" -and $sens -ne "0") "entries=$sens"
  $ok = $true
  foreach ($i in $script:__v) { if (-not $i.pass) { $ok = $false } }
  return @{ items = $script:__v; ok = $ok }
}

function Write-ValidationReport {
  param($Report)
  $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $f = Join-Path $TTLogDir ("validation-" + $stamp + ".json")
  (@{ tool = "trebleManager v$TTVersion"; timestamp = (Get-Date -Format "yyyy-MM-dd HH:mm:ss"); mode = $TT.Mode; profile = $TT.ProfileId; os = $(if ($TT.OS) { $TT.OS.Kind } else { "" }); results = $Report.items } | ConvertTo-Json -Depth 5) | Out-File $f -Encoding utf8
  Write-TTLog ("Validation report: " + $f) "SUCCESS"
  foreach ($i in $Report.items) {
    Write-Host ((if ($i.pass) { " [PASS] " } else { " [FAIL] " }) + $i.name + " -- " + $i.detail) -ForegroundColor $(if ($i.pass) { "Green" } else { "Red" })
  }
  return $f
}

function Test-FirmwareCompatibility {
  param([string]$Model, [string]$FirmwareString, [string]$Region)
  $reasons = @(); $status = "PASS"
  if ([string]::IsNullOrWhiteSpace($FirmwareString)) {
    return @{ Status = "FAIL"; Reasons = @("No firmware given (baseline missing).") }
  }
  $fw = $FirmwareString.ToUpper()
  $mo = $Model.ToUpper()
  # Model family: VTR vs VKY strict
  if ($mo.StartsWith("VTR") -and -not ($fw.Contains("VTR"))) {
    $status = "FAIL"; $reasons += "Firmware contains no VTR (P10), device is $Model."
  }
  if ($mo.StartsWith("VKY") -and -not ($fw.Contains("VKY"))) {
    $status = "FAIL"; $reasons += "Firmware contains no VKY (P10 Plus), device is $Model."
  }
  # Exact submodel (L29 vs L09): WARN not FAIL, often cross-compatible but check CUST
  if ($mo -eq "VTR-L29" -and $fw.Contains("VTR-L09") -and -not $fw.Contains("VTR-L29")) {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "Submodel mismatch: device L29, firmware L09 -> double-check CUST/region."
  }
  if ($mo -eq "VTR-L09" -and $fw.Contains("VTR-L29") -and -not $fw.Contains("VTR-L09")) {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "Submodel mismatch: device L09, firmware L29 -> double-check CUST/region."
  }
  # Region Cxxx
  $m1 = [regex]::Match($fw, "\(C(\d+)")
  $fwCust = ""; if ($m1.Success) { $fwCust = "C" + $m1.Groups[1].Value }
  if ($Region -ne "" -and $fwCust -ne "" -and $Region.ToUpper() -ne $fwCust.ToUpper()) {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "Region: device $Region vs firmware $fwCust -> flash only with matching CUST."
  }
  # EMUI 9.1 required for recovery_ramdisk method
  if ($fw -match "9\.1\.0") { $reasons += "EMUI 9.1 base ok (recovery_ramdisk method)." }
  elseif ($fw -match "9\.0") {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "EMUI 9.0 instead of 9.1 -> method possible, but prefer full 9.1 firmware."
  }
  elseif ($fw -match "8\.") {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "EMUI 8 base -> different boot chain possible, see wiki/kernel notes."
  } else {
    if ($status -eq "PASS") { $status = "WARN" }
    $reasons += "EMUI version not recognizable from firmware string -> verify manually."
  }
  if ($reasons.Count -eq 0) { $reasons += "Base check passed." }
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
  if (-not (Test-Path $Path)) { $res.Notes += (L "File missing: " "Datei fehlt: ") + $Path; return $res }
  $res.Exists = $true
  $info = Get-FileHashInfo $Path
  $res.Hash = $info
  if ($info.Size -ge 1MB -and $info.Size -le 100MB) { $res.SizeOk = $true }
  else { $res.Notes += (L "Size implausible: " "Groesse unplausibel: ") + "$($info.Size) bytes (expected 1-100 MB)." }
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
    if ($ascii.StartsWith("ANDROID!")) { $res.Notes += "Header ANDROID! (Android bootimg) detected." }
    elseif ($hex.StartsWith("1F 8B")) { $res.Notes += "Header gzip (1F 8B) detected." }
    else { $res.Notes += (L "Header unspecific ($hex) -> no exclusion, compare against stock manually." "Header unspezifisch ($hex) -> kein Ausschluss, aber manuell gegen Stock vergleichen.") }
  } catch { $res.Notes += (L "Header unreadable: " "Header nicht lesbar: ") + $_.Exception.Message }
  if ($res.Exists -and $res.SizeOk) { $res.Verdict = "PASS" } else { $res.Verdict = "FAIL" }
  return $res
}

function Test-BootImageMagic {
  # Pure, unit-testable: checks ANDROID! magic of first 8 bytes. Returns version int or -1.
  param([string]$Path)
  try {
    $fs = [System.IO.File]::OpenRead($Path)
    $buf = New-Object byte[] 12
    $n = $fs.Read($buf, 0, 12)
    $fs.Close()
    if ($n -lt 8) { return -1 }
    $magic = [System.Text.Encoding]::ASCII.GetString($buf, 0, 8)
    if ($magic -ne "ANDROID!") { return -1 }
    return [int]$buf[8]
  } catch { return -1 }
}

function Test-ImageKind {
  # Pure: boot (ANDROID! magic) | system (sparse/ext4 filesystem) | unknown.
  # A system image shows folders when opened - Magisk can never patch it.
  param([string]$Path)
  try {
    $fs = [System.IO.File]::OpenRead($Path)
    $buf = New-Object byte[] 1082
    $n = $fs.Read($buf, 0, 1082)
    $fs.Close()
    if ($n -ge 8 -and [System.Text.Encoding]::ASCII.GetString($buf, 0, 8) -eq "ANDROID!") { return "boot" }
    if ($n -ge 4 -and $buf[0] -eq 0x3A -and $buf[1] -eq 0xFF -and $buf[2] -eq 0x26 -and $buf[3] -eq 0xED) { return "system" }
    if ($n -ge 1082 -and $buf[1080] -eq 0x53 -and $buf[1081] -eq 0xEF) { return "system" }
  } catch {}
  return "unknown"
}

# ============================================================ Recovery export from custom ROMs
# Supports: recovery.img/boot.img direct, custom ROM .zip (boot.img/recovery.img inside),
# payload.bin (needs payload-dumper-go in data/tools/), GSI system .img (refused honestly).
$TTRomDir = Join-Path $TTData "roms"
$TTRecDir = Join-Path $TTData "recovery"
foreach ($p in @($TTRomDir,$TTRecDir)) {
  if (-not (Test-Path $p)) { New-Item -ItemType Directory -Path $p -Force | Out-Null }
}

function Get-RomImageEntries {
  # Pure ZIP listing: returns entry names. No extraction.
  param([string]$ZipPath)
  try {
    Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue | Out-Null
    $zip = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
    $names = @()
    foreach ($e in $zip.Entries) { $names += $e.FullName }
    $zip.Dispose()
    return $names
  } catch { return @() }
}

function Expand-TTRomArchive {
  # Extracts .tar/.tar.gz/.tgz ROM packages to $DestDir. tar.exe (Win10+) first,
  # python tarfile fallback. Returns $true on success.
  param([string]$Archive, [string]$DestDir)
  if (-not (Test-Path $DestDir)) { New-Item -ItemType Directory -Path $DestDir -Force | Out-Null }
  try {
    $tar = (Get-Command "tar.exe" -ErrorAction SilentlyContinue).Source
    if ($tar -ne "" -and (Test-Path $tar)) {
      & $tar -xf $Archive -C $DestDir 2>&1 | Out-Null
      if ($LASTEXITCODE -eq 0) { return $true }
    }
  } catch {}
  try {
    $py = (Get-Command "python" -ErrorAction SilentlyContinue).Source
    if ([string]::IsNullOrEmpty($py)) { $py = (Get-Command "python3" -ErrorAction SilentlyContinue).Source }
    if ($py -ne "" -and (Test-Path $py)) {
      & $py -c "import tarfile,sys; tarfile.open(sys.argv[1]).extractall(sys.argv[2])" $Archive $DestDir 2>&1 | Out-Null
      if ($LASTEXITCODE -eq 0) { return $true }
    }
  } catch {}
  return $false
}

function Expand-TTGzipImage {
  # Decompresses .gz/.img.gz to $DestDir via .NET GZipStream (no external tools).
  # Returns dest path or "".
  param([string]$Archive, [string]$DestDir)
  try {
    if (-not (Test-Path $DestDir)) { New-Item -ItemType Directory -Path $DestDir -Force | Out-Null }
    $name = [System.IO.Path]::GetFileName($Archive)
    if ($name.ToLower().EndsWith(".gz")) { $name = $name.Substring(0, $name.Length - 3) }
    if ([string]::IsNullOrWhiteSpace($name)) { $name = "image.img" }
    $dst = Join-Path $DestDir $name
    $fsIn = [System.IO.File]::OpenRead($Archive)
    try {
      $gz = New-Object System.IO.Compression.GzipStream($fsIn, [System.IO.Compression.CompressionMode]::Decompress)
      try {
        $fsOut = [System.IO.File]::Create($dst)
        try { $gz.CopyTo($fsOut) } finally { $fsOut.Close() }
      } finally { $gz.Close() }
    } finally { $fsIn.Close() }
    if ((Test-Path $dst) -and ((Get-Item $dst).Length -gt 0)) { return $dst }
  } catch { Write-TTLog (("GZip decompress failed: " + $_.Exception.Message)) "ERROR" }
  return ""
}

function Expand-TTXzImage {
  # Decompresses .xz/.img.xz to $DestDir (tar.exe first, python lzma fallback).
  # Returns dest path or "".
  param([string]$Archive, [string]$DestDir)
  try {
    if (-not (Test-Path $DestDir)) { New-Item -ItemType Directory -Path $DestDir -Force | Out-Null }
    $name = [System.IO.Path]::GetFileName($Archive)
    if ($name.ToLower().EndsWith(".xz")) { $name = $name.Substring(0, $name.Length - 3) }
    if ([string]::IsNullOrWhiteSpace($name)) { $name = "image.img" }
    $dst = Join-Path $DestDir $name
    # python lzma (tar.exe only handles tar containers, not single-file .xz):
    $py = (Get-Command "python" -ErrorAction SilentlyContinue).Source
    if ([string]::IsNullOrEmpty($py)) { $py = (Get-Command "python3" -ErrorAction SilentlyContinue).Source }
    if ($py -ne "" -and (Test-Path $py)) {
      & $py -c "import lzma,shutil,sys; shutil.copyfileobj(lzma.open(sys.argv[1],'rb'),open(sys.argv[2],'wb'))" $Archive $dst 2>&1 | Out-Null
      if ($LASTEXITCODE -eq 0 -and (Test-Path $dst) -and ((Get-Item $dst).Length -gt 0)) { return $dst }
    }
  } catch { Write-TTLog (("XZ decompress failed: " + $_.Exception.Message)) "ERROR" }
  return ""
}

function Export-RecoveryFromRom {
  # Exports boot/recovery images from a custom ROM package to data/recovery/<name>/.
  # Never fakes: GSI system images and unknown formats are refused with reasons.
  param([string]$RomPath)
  $res = @{ Ok = $false; Files = @(); Dir = ""; Notes = @() }
  if (-not (Test-Path $RomPath)) { $res.Notes += (L "ROM file missing: " "ROM-Datei fehlt: ") + $RomPath; return $res }
  $rawBase = [System.IO.Path]::GetFileName($RomPath)
  $base = $rawBase
  foreach ($sfx in @(".tar.gz",".tar.xz",".tgz",".gz",".xz",".zip",".img",".tar",".app")) {
    if ($base.ToLower().EndsWith($sfx)) { $base = $base.Substring(0, $base.Length - $sfx.Length); break }
  }
  if ([string]::IsNullOrWhiteSpace($base)) { $base = "rom" }
  $dir = Join-Path $TTRecDir ($base + "-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
  New-Item -ItemType Directory -Path $dir -Force | Out-Null
  $res.Dir = $dir
  $low = $RomPath.ToLower()
  # Normalize single-file compression first (.gz/.xz, but NOT tar containers):
  if (($low.EndsWith(".gz") -and -not $low.EndsWith(".tar.gz")) -or ($low.EndsWith(".xz") -and -not $low.EndsWith(".tar.xz"))) {
    if ($low.EndsWith(".gz")) { $raw = Expand-TTGzipImage $RomPath $dir }
    else { $raw = Expand-TTXzImage $RomPath $dir }
    if ($raw -eq "") {
      $res.Notes += (L "Decompress failed (corrupt file or missing python for .xz?)." "Dekomprimieren fehlgeschlagen (Datei kaputt oder python fehlt fuer .xz?).")
      return $res
    }
    $RomPath = $raw; $low = $RomPath.ToLower()
    $res.Notes += (L "Decompressed archive wrapper." "Archiv-Huelle dekomprimiert.")
  }
  if ($low.EndsWith(".img")) {
    $v = (Test-BootImageMagic $RomPath)
    if ($v -ge 0) {
      $dst = Join-Path $dir ([System.IO.Path]::GetFileName($RomPath))
      Copy-Item $RomPath $dst -Force
      $h = Get-FileHashInfo $dst
      $h.SHA256 | Out-File (Join-Path $dir "sha256.txt") -Encoding ascii
      (@{ source = $RomPath; kind = "direct-img"; bootimg_version = $v; sha256 = $h.SHA256; size = $h.Size; exported = (Get-Date -Format "yyyy-MM-dd HH:mm:ss") } | ConvertTo-Json -Depth 3) | Out-File (Join-Path $dir "metadata.json") -Encoding utf8
      $res.Ok = $true; $res.Files += $dst
      $res.Notes += (L "Direct boot image exported (bootimg v$v). Validate target partition from device profile before patch/flash." "Direktes Boot-Image exportiert (bootimg v$v). Zielpartition aus Geraeteprofil pruefen vor Patch/Flash.")
    } else {
      if ((Test-ImageKind $RomPath) -eq "system") {
        $res.Notes += (L "This is a SYSTEM image (filesystem with folders, e.g. lineage-...-signed.img). Magisk cannot patch system - it needs boot/recovery. A GSI leaves recovery untouched stock: use the stock UPDATE.APP recovery as patch base." "Das ist ein SYSTEM-Image (Dateisystem mit Ordnern, z.B. lineage-...-signed.img). Magisk kann kein System patchen - es braucht boot/recovery. Ein GSI laesst Recovery unberuehrt Stock: nimm das Stock-UPDATE.APP-Recovery als Patch-Basis.")
      } else {
        $res.Notes += (L "Not an Android boot image (no ANDROID! magic). GSI system images contain no recovery - use stock UPDATE.APP path instead." "Kein Android-Boot-Image (kein ANDROID!-Magic). GSI-System-Images enthalten kein Recovery - Stock-UPDATE.APP-Weg nutzen.")
      }
    }
    return $res
  }
  if ($low.EndsWith(".zip")) {
    $entries = Get-RomImageEntries $RomPath
    $cands = $entries | Where-Object { $_ -like "*recovery*.img" -or $_ -eq "boot.img" -or $_ -like "*/boot.img" }
    $hasPayload = ($entries | Where-Object { $_ -like "*payload.bin" }).Count -gt 0
    if ($cands.Count -gt 0) {
      Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue | Out-Null
      $zip = [System.IO.Compression.ZipFile]::OpenRead($RomPath)
      foreach ($c in $cands) {
        $e = $zip.Entries | Where-Object { $_.FullName -eq $c } | Select-Object -First 1
        if ($e -ne $null) {
          $dst = Join-Path $dir ([System.IO.Path]::GetFileName($c))
          [System.IO.Compression.ZipFileExtensions]::ExtractToFile($e, $dst, $true)
          if ((Test-BootImageMagic $dst) -ge 0) { $res.Files += $dst }
          else { $res.Notes += (L "Extracted but no boot magic: " "Extradiert aber kein Boot-Magic: ") + $c }
        }
      }
      $zip.Dispose()
      foreach ($f in $res.Files) { $h = Get-FileHashInfo $f; $h.SHA256 | Out-File ($f + ".sha256") -Encoding ascii }
      (@{ source = $RomPath; kind = "rom-zip"; files = ($res.Files | ForEach-Object { [System.IO.Path]::GetFileName($_) }); exported = (Get-Date -Format "yyyy-MM-dd HH:mm:ss") } | ConvertTo-Json -Depth 4) | Out-File (Join-Path $dir "metadata.json") -Encoding utf8
      if ($res.Files.Count -gt 0) { $res.Ok = $true; $res.Notes += (L "Exported from ROM zip. This is your patch base when this ROM is installed (never stock for it)." "Aus ROM-ZIP exportiert. Das ist deine Patch-Basis wenn dieses ROM installiert ist (niemals Stock dafuer).") }
    } elseif ($hasPayload) {
      $dumper = Get-ChildItem -Path $TTToolDir -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*payload*dumper*" -or $_.Name -eq "payload-dumper-go.exe" } | Select-Object -First 1
      if ($dumper -ne $null) {
        $res.Notes += (L "payload.bin ROM: extractor found, dumping boot/recovery ..." "payload.bin-ROM: Extractor gefunden, dumpe boot/recovery ...")
        try {
          & $dumper.FullName -o $dir -p "boot,recovery" $RomPath 2>&1 | Out-File (Join-Path $dir "dumper.log") -Encoding utf8
          $outs = Get-ChildItem -Path $dir -Filter "*.img" -ErrorAction SilentlyContinue
          foreach ($o in $outs) { if ((Test-BootImageMagic $o.FullName) -ge 0) { $res.Files += $o.FullName } }
          if ($res.Files.Count -gt 0) { $res.Ok = $true }
          else { $res.Notes += (L "Dumper ran but no valid boot images found." "Dumper lief, aber keine gueltigen Boot-Images gefunden.") }
        } catch { $res.Notes += (L "Dumper failed: " "Dumper fehlgeschlagen: ") + $_.Exception.Message }
      } else {
        $res.Notes += (L "payload.bin ROM without dumped images: place payload-dumper-go in data/tools/ or extract boot.img manually, then re-run export." "payload.bin-ROM ohne extrahierte Images: payload-dumper-go nach data/tools/ legen oder boot.img manuell extrahieren, dann Export wiederholen.")
      }
    } else {
      $res.Notes += (L "No boot.img/recovery.img/payload.bin in this ZIP. Probably a GSI system package - it has no recovery. Use stock UPDATE.APP path." "Kein boot.img/recovery.img/payload.bin in diesem ZIP. Wahrscheinlich GSI-System-Paket - hat kein Recovery. Stock-UPDATE.APP-Weg nutzen.")
    }
    return $res
  }
  if ($low.EndsWith(".tar") -or $low.EndsWith(".tar.gz") -or $low.EndsWith(".tar.xz") -or $low.EndsWith(".tgz")) {
    $stage = Join-Path $dir "_archive"
    if (-not (Expand-TTRomArchive $RomPath $stage)) {
      $res.Notes += (L "Archive extract failed (need tar.exe or python). Extract boot.img manually, then re-run export." "Archiv-Extrakt fehlgeschlagen (braucht tar.exe oder python). boot.img manuell extrahieren, dann Export wiederholen.")
      return $res
    }
    $all = Get-ChildItem -Path $stage -Recurse -File -ErrorAction SilentlyContinue
    $cands = @($all | Where-Object { $_.Name -like "*recovery*.img" -or $_.Name -eq "boot.img" })
    $pay = @($all | Where-Object { $_.Name -eq "payload.bin" } | Select-Object -First 1)
    if ($cands.Count -gt 0) {
      foreach ($c in $cands) {
        $dst = Join-Path $dir $c.Name
        Copy-Item $c.FullName $dst -Force
        if ((Test-BootImageMagic $dst) -ge 0) { $res.Files += $dst }
        else { $res.Notes += (L "Extracted but no boot magic: " "Extradiert aber kein Boot-Magic: ") + $c.Name }
      }
      foreach ($f in $res.Files) { $h = Get-FileHashInfo $f; $h.SHA256 | Out-File ($f + ".sha256") -Encoding ascii }
      (@{ source = $RomPath; kind = "rom-tar"; files = ($res.Files | ForEach-Object { [System.IO.Path]::GetFileName($_) }); exported = (Get-Date -Format "yyyy-MM-dd HH:mm:ss") } | ConvertTo-Json -Depth 4) | Out-File (Join-Path $dir "metadata.json") -Encoding utf8
      if ($res.Files.Count -gt 0) { $res.Ok = $true; $res.Notes += (L "Exported from ROM tar archive. This is your patch base when this ROM is installed (never stock for it)." "Aus ROM-TAR-Archiv exportiert. Das ist deine Patch-Basis wenn dieses ROM installiert ist (niemals Stock dafuer).") }
    } elseif ($pay.Count -gt 0) {
      $dumper = Get-ChildItem -Path $TTToolDir -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*payload*dumper*" -or $_.Name -eq "payload-dumper-go.exe" } | Select-Object -First 1
      if ($dumper -ne $null) {
        try {
          & $dumper.FullName -o $dir -p "boot,recovery" $pay[0].FullName 2>&1 | Out-File (Join-Path $dir "dumper.log") -Encoding utf8
          $outs = Get-ChildItem -Path $dir -Filter "*.img" -ErrorAction SilentlyContinue
          foreach ($o in $outs) { if ((Test-BootImageMagic $o.FullName) -ge 0) { $res.Files += $o.FullName } }
          if ($res.Files.Count -gt 0) { $res.Ok = $true }
          else { $res.Notes += (L "Dumper ran but no valid boot images found." "Dumper lief, aber keine gueltigen Boot-Images gefunden.") }
        } catch { $res.Notes += (L "Dumper failed: " "Dumper fehlgeschlagen: ") + $_.Exception.Message }
      } else {
        $res.Notes += (L "payload.bin inside tar without dumped images: place payload-dumper-go in data/tools/ or extract boot.img manually, then re-run export." "payload.bin im TAR ohne extrahierte Images: payload-dumper-go nach data/tools/ legen oder boot.img manuell extrahieren, dann Export wiederholen.")
      }
    } else {
      $res.Notes += (L "No boot.img/recovery.img/payload.bin in this archive. Probably a GSI system package - it has no recovery." "Kein boot.img/recovery.img/payload.bin in diesem Archiv. Wahrscheinlich GSI-System-Paket - hat kein Recovery.")
    }
    return $res
  }
  # NOTE: .tar.gz/.tgz/.tar.xz are caught by the tar branch above; plain
  # .gz/.xz were already decompressed by the normalization step (.img/.tar
  # dispatch rules below apply to the decompressed file).
  $res.Notes += (L "Unsupported format (use .img, .img.gz/.img.xz, ROM .zip or .tar/.tar.gz/.tgz)." "Nicht unterstuetztes Format (.img, .img.gz/.img.xz, ROM-.zip oder .tar/.tar.gz/.tgz nutzen).")
  return $res
}

# ============================================================ Tool discovery / wrappers
function Find-TTTools {
  # Saved setup config first (Setup-TrebleToolkit.bat -> data/config.json)
  try {
    $cfg = Join-Path $TTRoot "data\config.json"
    if (Test-Path $cfg) {
      $j = Get-Content $cfg -Raw -ErrorAction SilentlyContinue | ConvertFrom-Json -ErrorAction SilentlyContinue
      if ($j -ne $null) {
        if ($j.adb -ne "" -and (Test-Path $j.adb)) { $TT.Adb = $j.adb }
        if ($j.fastboot -ne "" -and (Test-Path $j.fastboot)) { $TT.Fastboot = $j.fastboot }
        if ($j.scrcpy -ne "" -and (Test-Path $j.scrcpy)) { $TT.Scrcpy = $j.scrcpy }
        if ($TT.Adb) { Write-TTLog "ADB (config): $($TT.Adb)" "SUCCESS" }
      }
    }
  } catch {}
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

  if ($TT.Adb) { Write-TTLog "ADB: $($TT.Adb)" "SUCCESS" } else { Write-TTLog (L "ADB not found (put Minimal ADB / platform-tools in PATH)." "ADB nicht gefunden (Minimal ADB / platform-tools in PATH legen).") "ERROR" }
  if ($TT.Fastboot) { Write-TTLog "Fastboot: $($TT.Fastboot)" "SUCCESS" } else { Write-TTLog (L "Fastboot not found." "Fastboot nicht gefunden.") "WARNING" }
  if (-not $TT.Adb -and -not $Script:TTFirstRunDone) { $Script:TTFirstRunDone = $true; Invoke-TTFirstRun }
  # scrcpy is optional (screen mirror during rooting), never required
  $TT.Scrcpy = $null
  try {
    $s = Get-Command "scrcpy.exe" -ErrorAction SilentlyContinue
    if (-not $s) { $s = Get-Command "scrcpy" -ErrorAction SilentlyContinue }
    if ($s) {
      $TT.Scrcpy = $s.Source
      try { $sv = (& $s.Source --version 2>&1 | Select-Object -First 1) } catch { $sv = "?" }
      Write-TTLog "scrcpy: $($TT.Scrcpy) ($sv)" "SUCCESS"
    } else { Write-TTLog (L "scrcpy not found (optional, screen mirror only: https://github.com/Genymobile/scrcpy)." "scrcpy nicht gefunden (optional, nur Screen-Mirror: https://github.com/Genymobile/scrcpy).") "INFO" }
  } catch {}
}

function Invoke-TTAdb {
  param([string[]]$Arguments)
  if (-not $TT.Adb) { Write-TTLog ((L "ADB missing, command skipped: " "ADB fehlt, Befehl uebersprungen: ") + "adb $($Arguments -join ' ')") "ERROR"; return @() }
  Write-TTLog ">> adb $($Arguments -join ' ')" "DEBUG"
  try {
    $o = & $TT.Adb @Arguments 2>&1
    $lines = @()
    foreach ($x in $o) { $lines += [string]$x }
    ($lines -join "`n") | Out-File -FilePath $TT.Log -Encoding utf8 -Append
    # Defense: bare/broken adb prints its help text - never treat that as data.
    if ((($lines -join "`n") -match "Android Debug Bridge version") -and -not ($Arguments -contains "version")) {
      Write-TTLog (L "adb printed help instead of answering (broken invocation or no device) - output ignored." "adb gab Hilfe statt Antwort aus (kaputter Aufruf oder kein Geraet) - Ausgabe ignoriert.") "ERROR"
      return @()
    }
    return $lines
  } catch { Write-TTLog ((L "ADB error: " "ADB-Fehler: ") + $_.Exception.Message) "ERROR"; return @() }
}

function Invoke-TTFastboot {
  param([string[]]$Arguments)
  if (-not $TT.Fastboot) { Write-TTLog (L "Fastboot missing, command skipped." "Fastboot fehlt, Befehl uebersprungen.") "ERROR"; return @() }
  Write-TTLog ">> fastboot $($Arguments -join ' ')" "DEBUG"
  try {
    $o = & $TT.Fastboot @Arguments 2>&1
    $lines = @()
    foreach ($x in $o) { $lines += [string]$x }
    ($lines -join "`n") | Out-File -FilePath $TT.Log -Encoding utf8 -Append
    return $lines
  } catch { Write-TTLog ((L "Fastboot error: " "Fastboot-Fehler: ") + $_.Exception.Message) "ERROR"; return @() }
}

function Invoke-FastbootLogged {
  # Logged fastboot call returning string lines (wrapper kept for flash flows).
  param([string[]]$Arguments)
  return @(Invoke-TTFastboot @Arguments)
}

function Get-FlashVerdict {
  # Pure evaluator for fastboot flash/erase output. FAILED lines veto everything:
  # progress words like "Writing"/"Erasing" alone are NOT success. OK requires
  # at least one OKAY plus a Finished line. Returns hashtable, no side effects.
  param([string[]]$Lines)
  $res = @{ Verdict = "UNCLEAR"; Okay = 0; Failed = @(); TotalTime = "" }
  foreach ($l in $Lines) {
    if ($l -match "OKAY") { $res.Okay++ }
    if ($l -match "FAILED|remote:|^\s*error") { $res.Failed += $l.Trim() }
    if ($l -match "Finished\.?\s*Total time:\s*(\S+)") { $res.TotalTime = $Matches[1] }
  }
  if ($res.Failed.Count -gt 0) { $res.Verdict = "FAILED" }
  elseif ($res.Okay -gt 0 -and $res.TotalTime -ne "") { $res.Verdict = "OK" }
  return $res
}

function Show-FlashVerdict {
  # Clear on-screen result for a flash/erase verdict. Full raw output always
  # stays in the session log ($TT.Log) - the screen shows verdict + reasons.
  param($V, [string]$What = "flash")
  if ($V.Verdict -eq "OK") {
    $t = $V.Okay.ToString() + " OKAY"
    if ($V.TotalTime -ne "") { $t += ", " + $V.TotalTime }
    Write-Host ((L "FLASH RESULT: OK (" "FLASH-ERGEBNIS: OK (") + $t + ")") -ForegroundColor Green
    Write-TTLog ("Flash result OK (" + $t + ").") "SUCCESS"
  } elseif ($V.Verdict -eq "FAILED") {
    Write-Host (L "FLASH RESULT: FAILED - nothing claimed as done." "FLASH-ERGEBNIS: FEHLGESCHLAGEN - nichts als erledigt behaupten.") -ForegroundColor Red
    foreach ($f in ($V.Failed | Select-Object -First 3)) { Write-Host (" ! " + $f) -ForegroundColor Red }
    Write-Host (L "Hints: 'Command not allowed' = Huawei refused (retry, cable, TROUBLESHOOTING). 'too large' = image bigger than partition. Full output is in the log." "Hinweise: 'Command not allowed' = Huawei lehnt ab (Retry, Kabel, TROUBLESHOOTING). 'too large' = Image groesser als Partition. Voll-Output im Log.") -ForegroundColor Yellow
    Write-TTLog (("Flash result FAILED (" + $What + "): " + (($V.Failed | Select-Object -First 1) -join "; "))) "ERROR"
  } else {
    Write-Host (L "FLASH RESULT: UNCLEAR - no FAILED, but no Finished either. Verify manually before rebooting." "FLASH-ERGEBNIS: UNKLAR - kein FAILED, aber auch kein Finished. Vor Reboot manuell verifizieren.") -ForegroundColor Yellow
    Write-TTLog (("Flash result UNCLEAR (" + $What + "): no FAILED, no Finished.")) "WARNING"
  }
}

function Get-TTProp {
  param([string]$Name)
  $o = Invoke-TTAdb @("shell","getprop",$Name)
  return ((($o -join "") -replace "`r","") -replace "`n","").Trim()
}

# ============================================================ Mode / analysis (OS-independent)
function Update-TTMode {
  $TT.Mode = "none"; $TT.AdbSerial = ""; $TT.FbSerial = ""
  if ($TT.Adb) {
    $o = Invoke-TTAdb @("devices")
    $devs = ConvertFrom-AdbDevices $o
    foreach ($d in $devs) { if ($d.State -eq "device") { $TT.Mode = "android"; $TT.AdbSerial = $d.Serial; break } }
    if ($TT.Mode -eq "none") {
      foreach ($d in $devs) {
        if ($d.State -eq "unauthorized") { Write-TTLog (L "ADB authorization pending (confirm dialog on device)." "ADB autorisierung ausstehend (Dialog am Geraet bestaetigen).") "WARNING" }
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
  "ro.boot.vbmeta.device_state","ro.secure","ro.debuggable","ro.lineage.version","ro.lineageos.version"
)

function Invoke-TTAndroidAnalysis {
  Write-TTLog (L "Android analysis (read-only, OS-independent) ..." "Android-Analyse (read-only, OS-unabhaengig) ...") "INFO"
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
  # Storage detection (eMMC vs UFS): informational, never assumed. CN units ship both.
  $TT.Storage = "unknown"
  try {
    $blk = (Invoke-TTAdb @("shell","ls /sys/block/ 2>&1") -join "`n")
    $hasMmc = $blk -match "mmcblk"
    $hasSd = ($blk -split "`n") | Where-Object { $_ -match "^\s*sd[a-z]\s*$|^\s*sd[a-z]\s" }
    if ($hasSd) { $TT.Storage = "UFS (sd* block devices present)" }
    elseif ($hasMmc) { $TT.Storage = "eMMC (mmcblk present, no sd*)" }
  } catch {}
  # Derive profile from model (GSI reports TrebleDroid instead of VTR -> note, profile stays VTR-L29 default)
  $model = [string]$h["ro.product.model"]
  $prof = ""
  if ($model -match "VTR-L29") { $prof = "VTR-L29" }
  elseif ($model -match "VTR-L09") { $prof = "VTR-L09" }
  elseif ($model -match "VTR-AL00") { $prof = "VTR-AL00" }
  elseif ($model -match "VTR-TL00") { $prof = "VTR-TL00" }
  elseif ($model -match "VKY-L29") { $prof = "VKY-L29" }
  elseif ($model -match "VKY-L09") { $prof = "VKY-L09" }
  elseif ($model -match "VKY-AL00") { $prof = "VKY-AL00" }
  elseif ($model -match "VKY-TL00") { $prof = "VKY-TL00" }
  else {
    # GSI/stock without Huawei model string -> default VTR-L29 (target device), flagged as WARN
    $prof = "VTR-L29"
    Write-TTLog (L "Model string is GSI ('$model'), not VTR/VKY. Profile default VTR-L29 (target device), verification before flash mandatory." "Modellstring ist GSI ('$model'), kein VTR/VKY. Profil-Default VTR-L29 (Zielgeraet), Verifikation vor Flash Pflicht.") "WARNING"
  }
  $TT.ProfileId = $prof
  Write-TTLog "OS: $($TT.OS.Kind) | $($TT.OS.Detail)" "SUCCESS"
  Write-TTLog ((L "Profile: " "Profil: ") + "$prof | " + (L "target partition: " "Zielpartition: ") + "$($DeviceProfiles[$prof].TargetPartition)") "INFO"
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
  Write-TTLog (L "Fastboot analysis (read-only) ..." "Fastboot-Analyse (read-only) ...") "INFO"
  # Never probe getvar without a fastboot device (fastboot would wait forever).
  if ($TT.Mode -ne "fastboot") {
    try {
      $d = & $TT.Fastboot devices 2>&1
      $has = $false
      foreach ($l in $d) { if ($l -match "fastboot\s*$") { $has = $true } }
      if (-not $has) {
        Write-TTLog (L "Not in fastboot mode, skipping getvar probes (would wait forever)." "Nicht im Fastboot-Modus, getvar-Abfragen uebersprungen (wuerden ewig warten).") "WARNING"
        return @{}
      }
      $TT.Mode = "fastboot"
    } catch {}
  }
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
    Write-TTLog (L "Huawei refuses getvar (Command not allowed). This is NOT proof of lock, just Huawei behavior." "Huawei verweigert getvar (Command not allowed). Das ist KEIN Lock-Beweis, nur Huawei-Eigenheit.") "WARNING"
  }
  return $all
}

# ============================================================ Firmware baseline (OS-dependent, assisted)
function Get-TTFirmwareBaseline {
  param([switch]$Interactive)
  $os = $TT.OS
  $base = ""
  if ($os -ne $null -and $os.IsStock) {
    $disp = [string]$TT.Props["ro.build.display.id"]
    $incr = Get-TTProp "ro.build.version.incremental"
    $base = ($disp + " " + $incr).Trim()
    Write-TTLog ((L "Stock detected -> baseline from build: " "Stock erkannt -> Baseline aus Build: ") + $base) "SUCCESS"
  } else {
    Write-TTLog (L "GSI/Custom detected -> Huawei base is hidden. Assisted baseline required." "GSI/Custom erkannt -> Huawei-Basis ist versteckt. Assistierte Baseline noetig.") "WARNING"
    Write-TTLog (L "Fastboot 'product' (if available) + your original firmware (e.g. VTR-L29 9.1.0.xxx(Cxxx...))." "Fastboot 'product' (falls verfuegbar) + deine Original-Firmware (z.B. VTR-L29 9.1.0.xxx(Cxxx...)).") "INFO"
    if ($TT.FbVars.ContainsKey("product")) { Write-TTLog "fastboot product: $($TT.FbVars['product'])" "INFO" }
  }
  if ($TT.FirmwareBaseline -ne "") { $base = $TT.FirmwareBaseline }
  if ($Interactive -and [string]::IsNullOrWhiteSpace($base)) {
    Write-Host ""
    Write-Host (L "Enter original Huawei firmware (e.g. VTR-L29 9.1.0.297(C432E5R1P9))." "Original Huawei-Firmware (Bsp: VTR-L29 9.1.0.297(C432E5R1P9)) eingeben.") -ForegroundColor Yellow
    Write-Host (L "On GSI: the firmware the GSI was flashed on top of, or matching device/region." "Bei GSI: die Firmware, auf deren Basis das GSI geflasht wurde bzw. die zum Geraet/Region passt.") -ForegroundColor Gray
    Write-Host (L "Firmware string (Enter = later): " "Firmware-String (Enter = spaeter): ") -NoNewline -ForegroundColor Yellow
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
    # Fallback: anything with RECOVERY_RAMDIS in the name (never guess/rename exact name)
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
  } catch { $res.Note = (L "Header unreadable: " "Header nicht lesbar: ") + $_.Exception.Message }
  $res.Note = (L "UPDATE.APP detected (" "UPDATE.APP erkannt (") + $([math]::Round($res.Size/1MB,1)) + (L " MB). Extraction: respect Huawei format (e.g. put huawei-update-extractor / splitupdate in data/tools/ or extract manually). Do NOT rename RECOVERY_RAMDIS(K).img, document original name." " MB). Extraktion: Huawei-Format beachten (z.B. huawei-update-extractor / splitupdate in data/tools/ ablegen oder manuell extrahieren). Danach RECOVERY_RAMDIS(K).img NICHT umbenennen, Originalnamen dokumentieren.")
  return $res
}

# ============================================================ Magisk (no fake patching)
function Invoke-TTFirstRun {
  # Very first start without tools: ask for a path OR install into user PATH
  # (needed tools + optional scrcpy), then save data/config.json.
  Write-Host ""
  Write-Host (L "FIRST RUN: required tools are missing (adb/fastboot)." "ERSTER START: benoetigte Tools fehlen (adb/fastboot).") -ForegroundColor Yellow
  $setup = Join-Path $TTRoot "scripts\Setup-Windows.ps1"
  if (Test-Path $setup) {
    Write-Host (L "Run the setup now? It asks for paths or installs into user PATH (scrcpy optional). [Y/n]: " "Setup jetzt starten? Fragt Pfade ab oder installiert in User-PATH (scrcpy optional). [J/n]: ") -NoNewline -ForegroundColor Cyan
    $a = Read-Host
    if ($a -eq "" -or $a -eq "Y" -or $a -eq "y" -or $a -eq "J" -or $a -eq "j") {
      try {
        $exe = (Get-Process -Id $PID).Path
      } catch { $exe = "powershell.exe" }
      & $exe -NoProfile -ExecutionPolicy Bypass -File "$setup"
      Find-TTTools
      return
    }
    Write-TTLog (L "Setup skipped by user - tools still missing." "Setup uebersprungen - Tools fehlen weiter.") "WARNING"
    return
  }
  # Remote run (irm|iex): no local setup file - print guidance instead.
  Write-Host (L "Remote run: run Setup-TrebleToolkit.bat from a local copy once, or place adb/fastboot on PATH." "Remote-Start: einmal Setup-TrebleToolkit.bat lokal ausfuehren oder adb/fastboot in PATH legen.") -ForegroundColor Yellow
}

function Find-TTMagiskApk {
  $hits = Get-ChildItem -Path $TTMagDir -Filter "*.apk" -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
  if ($hits -and $hits.Count -gt 0) { return $hits[0].FullName }
  return ""
}

function Get-TTMagiskInfo {
  param([string]$ApkPath)
  if ([string]::IsNullOrWhiteSpace($ApkPath) -or -not (Test-Path $ApkPath)) { return $null }
  $info = Get-FileHashInfo $ApkPath
  $ver = "unknown (check filename)"
  $m = [regex]::Match([System.IO.Path]::GetFileName($ApkPath), "[Vv]?(\d+\.\d+[\.\d]*)")
  if ($m.Success) { $ver = $m.Groups[1].Value }
  return @{ Path = $ApkPath; Version = $ver; SHA256 = $info.SHA256; Size = $info.Size; Source = "official: https://github.com/topjohnwu/Magisk/releases (verify manually)" }
}

function Prepare-TTMagiskPatch {
  param([string]$StockImage)
  $stage = Join-Path $TTMagDir "to-patch"
  if (-not (Test-Path $stage)) { New-Item -ItemType Directory -Path $stage -Force | Out-Null }
  $dest = Join-Path $stage ([System.IO.Path]::GetFileName($StockImage))
  Copy-Item -Path $StockImage -Destination $dest -Force
  $guide = Join-Path $stage "PATCH-INSTRUCTIONS.txt"
  @(
    (L "Magisk patch (real, never copy = claim patched)" "Magisk-Patch (echt, kein Kopieren = gepatcht behaupten)"),
    "=======================================================",
    "",
    (L "1. Install Magisk APK from official source:" "1. Magisk-APK aus offizieller Quelle installieren:"),
    "   https://github.com/topjohnwu/Magisk/releases",
    (L "   APK ideally in: data/magisk/" "   APK liegt idealerweise in: data/magisk/"),
    "",
    (L "2. Copy this file to the device:" "2. Diese Datei aufs Geraet kopieren:"),
    "   $dest",
    (L "   e.g.: adb push `"$dest`" /sdcard/Download/" "   z.B.: adb push `"$dest`" /sdcard/Download/"),
    "",
    (L "3. On device (any OS, stock or GSI/custom):" "3. Am Geraet (jedes OS, Stock wie GSI/Custom):"),
    (L "   Open Magisk -> Install -> 'Select and Patch a File'" "   Magisk oeffnen -> Installieren -> 'Datei auswählen und patchen'"),
    (L "   -> select RECOVERY_RAMDIS(K).img (exactly this file, NOT boot.img/recovery.img)" "   -> RECOVERY_RAMDIS(K).img auswaehlen (exakt diese Datei, NICHT boot.img/recovery.img)"),
    "",
    (L "4. Result on device: /sdcard/Download/magisk_patched-*.img" "4. Ergebnis am Geraet: /sdcard/Download/magisk_patched-*.img"),
    (L "   Copy back to PC:" "   Zurueck auf PC holen:"),
    "   adb pull /sdcard/Download/magisk_patched-XXXX.img data/magisk/",
    "",
    (L "5. In this tool choose 'Register patched file' and let it validate." "5. Hier im Tool 'Gepatchte Datei registrieren' waehlen und validieren lassen."),
    (L "   Validation: hash != stock, size plausible, header documented." "   Validierung: Hash != Stock, Groesse plausibel, Header dokumentiert.")
  ) | Out-File -FilePath $guide -Encoding utf8
  Write-TTLog ((L "Patch preparation: " "Patch-Vorbereitung: ") + $dest) "SUCCESS"
  Write-TTLog ((L "Instructions: " "Anleitung: ") + $guide) "INFO"
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
  } else { $note = "no stock image present - backup NOT faked" }
  # Attempt: read partition directly (honest, usually fails without root) - documented only
  $ddOut = ""
  if ($TT.Mode -eq "android") {
    $link = $TT.ByName | Where-Object { $_.Name -eq $part } | Select-Object -First 1
    if ($link -ne $null) {
      $ddOut = (Invoke-TTAdb @("shell","dd if=$($link.Target) of=/sdcard/recovery_ramdisk_dump.img bs=4096 count=8192 2>&1; echo EXIT:$?") -join "`n")
      Write-TTLog "dd attempt ($part): $ddOut" "DEBUG"
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
  else { Write-TTLog (L "Backup NOT possible (no original). Nothing is faked." "Backup NICHT moeglich (kein Original). Es wird nichts vorgetaeuscht.") "ERROR" }
  return @{ Ok = $ok; Dir = $dir }
}

function Test-TTFlashReadiness {
  $checks = @()
  $prof = $DeviceProfiles[$TT.ProfileId]
  if ($prof -eq $null) { $prof = $DeviceProfiles["VTR-L29"] }

  $c1 = ($TT.ProfileId -match "VTR|VKY")
  $checks += New-Object PSObject -Property @{ Name = "Model matches (VTR/VKY)"; Pass = [bool]$c1; Detail = "Profil: $($TT.ProfileId)" }

  $profVerified = ($prof.Verified -eq $true)
  $checks += New-Object PSObject -Property @{ Name = "Profile verified (flash allowed)"; Pass = [bool]$profVerified; Detail = $(if ($profVerified) { "verified method" } else { "UNVERIFIED - submit device data first, analyze/export only" }) }

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
  $checks += New-Object PSObject -Property @{ Name = "Image hash known (SHA-256)"; Pass = [bool]$hashOk; Detail = $(if ($TT.PatchedHash) { $TT.PatchedHash.SHA256.Substring(0,16)+"..." } else { "(no hash)" }) }

  $sizeOk = ($TT.PatchedHash -ne $null -and $TT.PatchedHash.Size -ge 1MB -and $TT.PatchedHash.Size -le 100MB)
  $checks += New-Object PSObject -Property @{ Name = "Image size plausible (1-100 MB)"; Pass = [bool]$sizeOk; Detail = $(if ($TT.PatchedHash) { [string]$TT.PatchedHash.Size } else { "?" }) }

  $fwOk = ($TT.FirmwareCompat -ne $null -and $TT.FirmwareCompat.Status -ne "FAIL" -and -not [string]::IsNullOrWhiteSpace($TT.FirmwareBaseline))
  $checks += New-Object PSObject -Property @{ Name = "Firmware compatibility established"; Pass = [bool]$fwOk; Detail = "$($TT.FirmwareBaseline) [$($TT.FirmwareCompat.Status)]" }

  $bakOk = ($TT.BackupDir -ne "" -and (Test-Path (Join-Path $TT.BackupDir "original.img")))
  $checks += New-Object PSObject -Property @{ Name = "Backup available (original.img)"; Pass = [bool]$bakOk; Detail = [string]$TT.BackupDir }

  $fbOk = ($TT.Mode -eq "fastboot" -and $TT.Fastboot -ne $null)
  # Re-evaluate fastboot live (never trust cache blindly)
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
  $checks += New-Object PSObject -Property @{ Name = "Patched != stock (no fake patch)"; Pass = [bool]$diffOk; Detail = "Hashes must differ" }

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
  Write-Host (L "=== Safety check before flash ===" "=== Sicherheitspruefung vor Flash ===") -ForegroundColor Cyan
  foreach ($c in $readiness.Checks) {
    if ($c.Pass) { Write-Host (" [OK]   " + $c.Name + " -- " + $c.Detail) -ForegroundColor Green }
    else { Write-Host (" [FAIL] " + $c.Name + " -- " + $c.Detail) -ForegroundColor Red }
  }
  if (-not $readiness.Go) {
    Write-Host ""
    Write-Host (L "DO NOT FLASH - at least one check failed." "DO NOT FLASH - mindestens eine Pruefung ist fehlgeschlagen.") -ForegroundColor Red -BackgroundColor Black
    Write-TTLog (L "Flash blocked (safety gate)." "Flash blockiert (Safety-Gate).") "ERROR"
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
  Write-Host (L "GSI (system/vendor) stays untouched. userdata is NEVER wiped." "GSI (system/vendor) bleibt unangetastet. NIEMALS wird userdata geloescht.") -ForegroundColor Gray
  Write-Host "Continue?" -ForegroundColor Yellow
  if (-not $ForceYes) {
    Write-Host (L "Type 'FLASH' to continue (1/2): " "Zum Fortfahren 'FLASHEN' tippen (1/2): ") -NoNewline -ForegroundColor Yellow
    $a = Read-Host
    if ($a -ne "FLASHEN" -and $a -ne "FLASH") { Write-TTLog (L "Flash aborted (1st confirmation missing)." "Flash abgebrochen (1. Bestaetigung fehlt).") "WARNING"; return $false }
    Write-Host (L "Really sure? Type 'YES' again (2/2): " "Wirklich sicher? Nochmal 'JA' (2/2): ") -NoNewline -ForegroundColor Yellow
    $b = Read-Host
    if ($b -ne "JA" -and $b -ne "YES") { Write-TTLog (L "Flash aborted (2nd confirmation missing)." "Flash abgebrochen (2. Bestaetigung fehlt).") "WARNING"; return $false }
  } else {
    Write-TTLog (L "CLI --yes: explicit consent via flag documented." "CLI --yes: explizite Bestaetigung via Flag dokumentiert.") "WARNING"
  }
  # Command derived from profile (NOT hardcoded to other partitions)
  Write-TTLog "Starting: fastboot flash $part <patched>" "WARNING"
  $o = Invoke-TTFastboot @("flash",$part,$TT.PatchedImage)
  $v = Get-FlashVerdict $o
  Show-FlashVerdict $v ("flash " + $part)
  $ok = ($v.Verdict -eq "OK")
  return [bool]$ok
}

function Invoke-TTRootVerification {
  param([switch]$NoReboot)
  if (-not $NoReboot -and $TT.Mode -eq "fastboot" -and $TT.Fastboot) {
    Write-TTLog "fastboot reboot ..." "INFO"
    Invoke-TTFastboot @("reboot") | Out-Null
  }
  Write-Host ""
  Write-Host (L "Huawei boot procedure (mandatory, otherwise no root):" "Huawei Boot-Prozedur (Pflicht, sonst kein Root):") -ForegroundColor Cyan
  Write-Host (L "  Vol-Up + Power until Huawei logo, then release (Magisk boot cheat)." "  Vol-Up + Power bis Huawei-Logo, dann loslassen (Magisk boot cheat).") -ForegroundColor White
  Write-Host (L "  Without trick it boots stock (no root). If needed open Magisk app and grant root." "  Ohne Trick bootet Stock (ohne Root). Ggf. Magisk-App oeffnen und Root freigeben.") -ForegroundColor Gray
  Write-Host ""
  if ($TT.Adb) {
    Write-TTLog (L "Waiting for adb (wait-for-device, up to 120s, first boot takes a while) ..." "Warte auf adb (wait-for-device, bis 120s, erster Boot dauert) ...") "INFO"
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
    $res.Root = "INCONCLUSIVE"; Write-TTLog (L "su present but no uid=0 (approval / Magisk app / first boot?). INCONCLUSIVE." "su vorhanden, aber kein uid=0 (Freigabe/Magisk-App/first-boot?). INCONCLUSIVE.") "WARNING"
  } else {
    $res.Root = "NOT_ROOTED"; Write-TTLog (L "No root verifiable. Repeat boot trick + check Magisk app." "Kein Root nachweisbar. Boot-Trick wiederholen + Magisk-App pruefen.") "WARNING"
  }
  Write-TTLog (L "Note: boot alone != root. Only uid=0 counts." "Hinweis: Boot allein != Root. Nur uid=0 zaehlt.") "INFO"
  return $res
}

function Invoke-TTRestoreFlow {
  param([string]$BackupPick = "", [switch]$ForceYes)
  $cands = Get-ChildItem -Path (Join-Path (Join-Path $TTBackDir $TT.ProfileId) $DeviceProfiles[$TT.ProfileId].TargetPartition) -Directory -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
  if (-not $cands -or $cands.Count -eq 0) {
    # Fallback: all backups
    $cands = Get-ChildItem -Path $TTBackDir -Recurse -Directory -ErrorAction SilentlyContinue | Where-Object { Test-Path (Join-Path $_.FullName "original.img") } | Sort-Object LastWriteTime -Descending
  }
  if (-not $cands -or $cands.Count -eq 0) { Write-TTLog (L "No backup with original.img found." "Kein Backup mit original.img gefunden.") "ERROR"; return $false }
  $pick = $cands[0].FullName
  if ($BackupPick -ne "" -and (Test-Path $BackupPick)) { $pick = $BackupPick }
  # Never flash without a live fastboot device.
  Update-TTMode | Out-Null
  if ($TT.Mode -ne "fastboot") {
    Write-TTLog (L "Not in fastboot mode. Reboot to fastboot first, then restore." "Nicht im Fastboot-Modus. Erst nach Fastboot booten, dann Restore.") "ERROR"
    return $false
  }
  Write-TTLog ((L "Restore candidate: " "Restore-Kandidat: ") + $pick) "INFO"
  $orig = Join-Path $pick "original.img"
  $metaF = Join-Path $pick "metadata.json"
  try {
    $meta = Get-Content $metaF -Raw | ConvertFrom-Json
    Write-Host ("Backup: " + $meta.model + " / " + $meta.partition + " / " + $meta.firmware) -ForegroundColor White
  } catch { Write-TTLog (L "metadata.json unreadable, continuing with hash check." "metadata.json nicht lesbar, fahre mit Hash-Pruefung fort.") "WARNING" }
  $h = Get-FileHashInfo $orig
  if ($h -eq $null) { Write-TTLog (L "original.img unreadable." "original.img unlesbar.") "ERROR"; return $false }
  Write-TTLog ("Restore hash SHA-256: " + $h.SHA256) "INFO"
  $part = $DeviceProfiles[$TT.ProfileId].TargetPartition
  Write-Host ""
  Write-Host "Restore: fastboot flash $part original.img" -ForegroundColor Yellow
  if (-not $ForceYes) {
    Write-Host (L "Type 'RESTORE' + 'YES' to continue." "Zum Fortfahren 'RESTORE' + 'JA' tippen.") -ForegroundColor Yellow
    $a = Read-Host "1/2 (RESTORE)"
    if ($a -ne "RESTORE") { return $false }
    $b = Read-Host (L "2/2 (YES)" "2/2 (JA)")
    if ($b -ne "JA" -and $b -ne "YES") { return $false }
  }
  $o = Invoke-TTFastboot @("flash",$part,$orig)
  Write-Host ($o -join "`n") -ForegroundColor White
  Write-TTLog (L "Restore executed, output above." "Restore ausgefuehrt, Ausgabe oben.") "WARNING"
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
  } catch { Write-TTLog ((L "ZIP failed: " "ZIP fehlgeschlagen: ") + $_.Exception.Message) "ERROR"; return "" }
  return $zip
}

# ============================================================ Orchestrator (P0): registry, states, preflight, gates
# Principles: unknown is never compatible; --yes never bypasses prerequisites;
# unauthorized/offline/multiple devices are explicit states, never "absent".

$ToolRegistry = @(
  @{ Id = "adb"; Required = $true;  Bins = @("adb.exe","adb"); Kind = "platform-tools" }
  @{ Id = "fastboot"; Required = $true;  Bins = @("fastboot.exe","fastboot"); Kind = "platform-tools" }
  @{ Id = "scrcpy"; Required = $false; Bins = @("scrcpy.exe","scrcpy"); Kind = "optional-mirror" }
)

function Get-DeviceStates {
  # Pure parsing + live queries. Never guesses: unauthorized/offline/multiple are own states.
  $adbDevs = @()
  $adbState = "ADB_NOT_FOUND"
  if ($TT.Adb) {
    $o = & $TT.Adb devices 2>&1
    $lines = @()
    foreach ($x in $o) { $lines += [string]$x }
    $adbDevs = ConvertFrom-AdbDevices $lines
    $ready = @($adbDevs | Where-Object { $_.State -eq "device" }).Count
    $unauth = @($adbDevs | Where-Object { $_.State -eq "unauthorized" }).Count
    $off = @($adbDevs | Where-Object { $_.State -eq "offline" }).Count
    if ($ready -gt 1) { $adbState = "ADB_MULTIPLE_DEVICES" }
    elseif ($ready -eq 1) { $adbState = "ADB_READY" }
    elseif ($unauth -gt 0) { $adbState = "ADB_UNAUTHORIZED" }
    elseif ($off -gt 0) { $adbState = "ADB_OFFLINE" }
    else { $adbState = "NO_DEVICE" }
  }
  $fbDevs = @()
  $fbState = "FASTBOOT_NOT_FOUND"
  if ($TT.Fastboot) {
    $o = & $TT.Fastboot devices 2>&1
    $lines = @()
    foreach ($x in $o) { $lines += [string]$x }
    $fbDevs = ConvertFrom-FastbootDevices $lines
    if ($fbDevs.Count -gt 1) { $fbState = "FASTBOOT_MULTIPLE_DEVICES" }
    elseif ($fbDevs.Count -eq 1) { $fbState = "FASTBOOT_READY" }
    else { $fbState = "FASTBOOT_NO_DEVICE" }
  }
  if ($adbState -in @("ADB_NOT_FOUND","NO_DEVICE") -and $fbState -in @("FASTBOOT_NOT_FOUND","FASTBOOT_NO_DEVICE")) {
    $overall = "NO_DEVICE"
  } elseif ($adbState -eq "ADB_READY" -or $fbState -eq "FASTBOOT_READY") {
    $overall = "READY"
  } else {
    $overall = "UNKNOWN_DEVICE_STATE"
  }
  return @{ AdbState = $adbState; AdbDevices = $adbDevs; FastbootState = $fbState; FastbootDevices = $fbDevs; Overall = $overall }
}

function Select-TargetDevice {
  # Multiple devices: explicit selection via ANDROID_SERIAL (honored by adb+fastboot).
  param($States)
  $all = @()
  foreach ($d in $States.AdbDevices) { if ($d.State -eq "device") { $all += $d.Serial + " (adb)" } }
  foreach ($d in $States.FastbootDevices) { $all += $d.Serial + " (fastboot)" }
  if ($all.Count -le 1) { return $true }
  Write-Host (L "Multiple devices - select target (ANDROID_SERIAL will be set):" "Mehrere Geraete - Ziel waehlen (ANDROID_SERIAL wird gesetzt):") -ForegroundColor Yellow
  for ($i = 0; $i -lt $all.Count; $i++) { Write-Host (" [" + ($i+1) + "] " + $all[$i]) -ForegroundColor White }
  Write-Host (L "Number: " "Nummer: ") -NoNewline -ForegroundColor Yellow
  $s = Read-Host
  if ($s -match "^\d+$") {
    $idx = [int]$s - 1
    if ($idx -ge 0 -and $idx -lt $all.Count) {
      $ser = ($all[$idx] -split " ")[0]
      $env:ANDROID_SERIAL = $ser
      Write-TTLog ("Target device selected: " + $ser + " (ANDROID_SERIAL)") "SUCCESS"
      return $true
    }
  }
  Write-TTLog (L "No target selected - aborting." "Kein Ziel gewaehlt - Abbruch.") "ERROR"
  return $false
}

function Invoke-Preflight {
  # Global startup gate. Missing global prerequisites BLOCK the main menu.
  $blocks = @()
  if ([System.Environment]::OSVersion.Platform -ne "Win32NT" -and $PSVersionTable.PSVersion.Major -lt 6) {
    $blocks += "unsupported shell (need Windows PowerShell 5.1+ or PowerShell 7+)"
  }
  Find-TTTools
  foreach ($t in ($ToolRegistry | Where-Object { $_.Required })) {
    $ok = ($t.Id -eq "adb" -and $TT.Adb) -or ($t.Id -eq "fastboot" -and $TT.Fastboot)
    if (-not $ok) { $blocks += ($t.Id + " missing (install via Setup or platform-tools on PATH)") }
  }
  foreach ($d in @($TTLogDir, $TTBackDir, $TTFirmDir, $TTMagDir)) {
    try {
      $probe = Join-Path $d ".writetest"
      "x" | Out-File $probe -Encoding ascii -ErrorAction Stop
      Remove-Item $probe -Force -ErrorAction SilentlyContinue
    } catch { $blocks += ("directory not writable: " + $d) }
  }
  $st = Get-DeviceStates
  $TT.DeviceStates = $st
  $go = ($blocks.Count -eq 0)
  return @{ Go = $go; Blocks = $blocks; States = $st }
}

function Show-PreflightBlocked {
  param($Pre)
  Show-TTHeader (L "Preflight blocked - fix first, no main menu" "Preflight blockiert - erst beheben, kein Hauptmenue")
  Write-Host ""
  Write-Host (L "Missing global prerequisites:" "Fehlende Grundvoraussetzungen:") -ForegroundColor Red
  foreach ($b in $Pre.Blocks) { Write-Host (" [NOT READY] " + $b) -ForegroundColor Red }
  Write-Host ""
  Write-Host (L "Fix: run the setup now? It asks for paths or installs into user PATH (scrcpy optional). [Y/n]: " "Fix: Setup jetzt starten? Fragt Pfade ab oder installiert in User-PATH (scrcpy optional). [J/n]: ") -NoNewline -ForegroundColor Cyan
  $a = Read-Host
  if ($a -eq "" -or $a -eq "Y" -or $a -eq "y" -or $a -eq "J" -or $a -eq "j") {
    $setup = Join-Path $TTRoot "scripts\Setup-Windows.ps1"
    if (Test-Path $setup) {
      try {
        $exe = (Get-Process -Id $PID).Path
      } catch { $exe = "powershell.exe" }
      & $exe -NoProfile -ExecutionPolicy Bypass -File "$setup"
    } else {
      Write-Host (L "No local setup file (remote run): place adb/fastboot on PATH, then restart." "Keine lokale Setup-Datei (Remote-Start): adb/fastboot in PATH legen, dann Neustart.") -ForegroundColor Yellow
    }
  }
  Write-Host ""
  Write-Host (L "Device states right now:" "Geraete-Status gerade:") -ForegroundColor Gray
  Write-Host (" ADB: " + $Pre.States.AdbState + " | Fastboot: " + $Pre.States.FastbootState) -ForegroundColor Gray
  Pause-TT
}

# Step requirement engine: every step declares needs; missing needs block the step.
function Test-StepGate {
  param([string]$Step)
  $reasons = @()
  $st = Get-DeviceStates
  $needDevice = $Step -in @("analyze","verify","patch","backup","flash","restore","flash-system","twrp","validate","export","diagnostic")
  $needFastboot = $Step -in @("flash","restore","flash-system","twrp")
  $needAndroid = $Step -in @("analyze","verify","validate")
  if ($needDevice -and $st.Overall -eq "NO_DEVICE") { $reasons += "no device connected" }
  if ($needAndroid -and $st.AdbState -ne "ADB_READY") { $reasons += ("need Android/ADB (now: " + $st.AdbState + ")") }
  if ($needFastboot -and $st.FastbootState -ne "FASTBOOT_READY") { $reasons += ("need fastboot (now: " + $st.FastbootState + ")") }
  if ($Step -in @("flash","twrp") -and $DeviceProfiles[$TT.ProfileId].Verified -ne $true) { $reasons += "profile unverified" }
  if ($Step -in @("flash-system") -and $DeviceProfiles[$TT.ProfileId].Verified -ne $true) { $reasons += "profile unverified" }
  if ($Step -eq "flash" -and [string]::IsNullOrEmpty($TT.PatchedImage)) { $reasons += "no patched image registered" }
  if ($Step -eq "backup") {
    $pb = Get-PatchBase
    if ([string]::IsNullOrWhiteSpace($pb.Image) -or -not (Test-Path $pb.Image)) {
      if ($pb.Source -eq "rom") { $reasons += "no ROM base image (recovery export first)" }
      else { $reasons += "no stock image (extract first)" }
    }
  }
  return @{ Pass = ($reasons.Count -eq 0); Reasons = $reasons; States = $st }
}

# ============================================================ TUI basics
function Show-TTHeader {
  param([string]$Title)
  Clear-Host
  $admin = "NO"; if (Test-TTAdmin) { $admin = "YES" }
  if ($TTLang -eq "de" -and $admin -eq "NO") { $admin = "NEIN" }
  if ($TTLang -eq "de" -and $admin -eq "YES") { $admin = "JA" }
  Write-Host "================================================================" -ForegroundColor DarkCyan
  Write-Host " Huawei P10 Root Manager  v$TTVersion  |  TUI (PS $($PSVersionTable.PSVersion))" -ForegroundColor Cyan
  Write-Host (" " + (L "Mode" "Modus") + ": $($TT.Mode.ToUpper())  Admin: $admin  " + (L "Profile" "Profil") + ": $($TT.ProfileId)  OS: $(if ($TT.OS) { $TT.OS.Kind } else { '?' })  ROM: $(Get-RomLabel $TT.InstalledRom)") -ForegroundColor DarkGray
  $chip = "READY"
  if ($TT.Mode -eq "none") { $chip = "WARNING: kein Geraet" }
  Write-Host (" State: $chip  |  Log: logs\") -ForegroundColor DarkGray
  Write-Host "================================================================`n" -ForegroundColor DarkCyan
  Write-Host " $Title" -ForegroundColor White
}

function Show-TTMenu {
  param([string]$Title, [string[]]$Options, [string]$Hint = "")
  if ($Hint -eq "") { $Hint = (L "Arrow keys + Enter | 1-9 | Esc = back" "Pfeiltasten + Enter | 1-9 | Esc = zurueck") }
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
  Write-Host (L "[Enter] back ..." "[Enter] zurueck ...") -ForegroundColor DarkGray -NoNewline
  [void][Console]::ReadKey($true)
}

function Show-TTStatus {
  Show-TTHeader (L "Status (any OS is detected, nothing assumed)" "Status (jedes OS wird erkannt, nichts vorausgesetzt)")
  Write-Host ""
  Write-Host ((L "Device profile : " "Device-Profil : ") + $TT.ProfileId + " (" + $DeviceProfiles[$TT.ProfileId].Marketing + ")") -ForegroundColor White
  if ($TT.OS -ne $null) {
    Write-Host ((L "OS class       : " "OS-Klasse     : ") + $TT.OS.Kind) -ForegroundColor White
    Write-Host ((L "Detail         : " "Detail        : ") + $TT.OS.Detail) -ForegroundColor Gray
    Write-Host ((L "GSI variant    : " "GSI-Variante  : ") + $TT.OS.GsiVariant) -ForegroundColor Gray
  } else { Write-Host (L "OS class       : (no analysis yet)" "OS-Klasse     : (noch keine Analyse)") -ForegroundColor Yellow }
  Write-Host ("Android       : " + [string]$TT.Props["ro.build.version.release"]) -ForegroundColor White
  Write-Host ((L "Mode          : " "Modus         : ") + $TT.Mode) -ForegroundColor White
  $rr = ($TT.ByName | Where-Object { $_.Name -eq "recovery_ramdisk" } | Select-Object -First 1)
  if ($rr) { Write-Host "RecoveryRamdisk: DETECTED ($($rr.Target))" -ForegroundColor Green }
  else { Write-Host (L "RecoveryRamdisk: UNKNOWN/NOT DETECTED (run analysis)" "RecoveryRamdisk: UNKNOWN/NOT DETECTED (Analyse laufen lassen)") -ForegroundColor Yellow }
  Write-Host ("Firmware      : " + $(if ($TT.FirmwareBaseline -ne "") { $TT.FirmwareBaseline } else { "UNKNOWN" })) -ForegroundColor White
  Write-Host ((L "Phone runs   : " "Handy laeuft: ") + (Get-RomLabel $TT.InstalledRom)) -ForegroundColor Cyan
  Write-Host ((L "Stock image    : " "Stock-Image   : ") + $(if ($TT.StockImage -ne "") { $TT.StockImage } else { (L "missing" "fehlt") })) -ForegroundColor White
  Write-Host ((L "Patched image  : " "Patched-Image : ") + $(if ($TT.PatchedImage -ne "") { $TT.PatchedImage } else { (L "missing" "fehlt") })) -ForegroundColor White
  Write-Host ("Backup        : " + $(if ($TT.BackupDir -ne "") { $TT.BackupDir } else { (L "missing" "fehlt") })) -ForegroundColor White
  Write-Host ""
  Write-Host (L "Bootloader: Unknown/Unlocked/Locked is NEVER guessed from 'Command not allowed'." "Bootloader: Unknown/Unlocked/Locked wird NICHT aus 'Command not allowed' geraten.") -ForegroundColor DarkGray
  Write-Host (L "Magisk: only report verified (uid=0), never from boot alone." "Magisk: nur verifiziert melden (uid=0), niemals aus Boot allein.") -ForegroundColor DarkGray
  Pause-TT
}

# ============================================================ TUI screens (wizard steps 1-9)
function Screen-Detect {
  Show-TTHeader (L "Step 1 - Device (detect, auto ADB/Fastboot)" "Step 1 - Device (Detect, auto ADB/Fastboot)")
  Update-TTMode | Out-Null
  Write-Host ""
  Write-Host ((L "Mode: " "Modus: ") + $TT.Mode) -ForegroundColor White
  if ($TT.AdbSerial -ne "") { Write-Host ("ADB: " + $TT.AdbSerial) -ForegroundColor White }
  if ($TT.FbSerial -ne "") { Write-Host ("Fastboot: " + $TT.FbSerial + " " + (L "(e.g. 6PQ0217B08003446)" "(z.B. 6PQ0217B08003446)")) -ForegroundColor White }
  if ($TT.Mode -eq "none") {
    Write-Host ""
    Write-Host (L "No device. Check USB debugging / fastboot drivers / cable." "Kein Geraet. USB-Debugging an / Fastboot-Treiber pruefen / Kabel wechseln.") -ForegroundColor Red
  } else { Write-TTLog ((L "Device mode: " "Device-Modus: ") + $TT.Mode) "SUCCESS" }
  Pause-TT
}

function Screen-Analyze {
  Show-TTHeader (L "Step 2 - Analyze (Android + partitions + boot chain, read-only)" "Step 2 - Analyse (Android + Partitionen + Bootchain, read-only)")
  if ($TT.Mode -eq "none") { Update-TTMode | Out-Null }
  if ($TT.Mode -eq "android") {
    Invoke-TTAndroidAnalysis | Out-Null
    Write-Host ""
    Write-Host ((L "OS class: " "OS-Klasse: ") + $TT.OS.Kind) -ForegroundColor Cyan
    Write-Host ("Detail: " + $TT.OS.Detail) -ForegroundColor White
    $linVer = [string]$TT.Props["ro.lineage.version"]
    if ([string]::IsNullOrEmpty($linVer)) { $linVer = [string]$TT.Props["ro.lineageos.version"] }
    if (-not [string]::IsNullOrEmpty($linVer)) {
      Write-Host ("LineageOS: " + $linVer) -ForegroundColor Green
      Write-Host (L "Custom ROM detected: the wizard determines the correct patch base (stock recovery for GSIs, ROM package for device builds)." "Custom-ROM erkannt: Der Wizard bestimmt die korrekte Patch-Basis (Stock-Recovery fuer GSIs, ROM-Paket fuer Device-Builds).") -ForegroundColor Cyan
    }
    $rel = [string]$TT.Props["ro.build.version.release"]
    if (($TT.OS.Kind -like "*GSI*") -and ($rel -match "^13") -and ($TT.ProfileId -match "VTR|VKY")) {
      Write-Host ("WARN: " + $WikiKnowledge.Android13Warning) -ForegroundColor Yellow
    }
    Write-Host ""
    Write-Host "--- Partitions (/dev/block/by-name, filtered) ---" -ForegroundColor Cyan
    $hit = $false
    foreach ($n in $TT.ByName) {
      if ($n.Name -match "boot|recovery|ramdisk|system|vendor|vbmeta") {
        Write-Host (" " + $n.Name.PadRight(18) + " -> " + $n.Target) -ForegroundColor White
        $hit = $true
      }
    }
    if (-not $hit) { Write-Host $TT.ByNameRaw -ForegroundColor White }
    Write-Host ""
    Write-Host "--- Boot state ---" -ForegroundColor Cyan
    Write-Host (" verifiedbootstate=" + [string]$TT.Props["ro.boot.verifiedbootstate"]) -ForegroundColor White
    Write-Host (" flash.locked=" + [string]$TT.Props["ro.boot.flash.locked"]) -ForegroundColor White
    Write-Host (" vbmeta=" + [string]$TT.Props["ro.boot.vbmeta.device_state"]) -ForegroundColor White
    Write-Host (" slot=" + [string]$TT.Props["ro.boot.slot_suffix"] + (L " (P10: do not assume classic A/B)" " (P10: kein klassisches A/B voraussetzen)")) -ForegroundColor White
    $rr = $TT.ByName | Where-Object { $_.Name -eq "recovery_ramdisk" }
    if ($rr) { Write-Host "recovery_ramdisk: DETECTED" -ForegroundColor Green }
    else { Write-Host (L "recovery_ramdisk: NOT seen in by-name -> fastboot analysis + firmware path needed." "recovery_ramdisk: NICHT in by-name gesehen -> Fastboot-Analyse + Firmware-Weg noetig.") -ForegroundColor Yellow }
    Write-Host ""
    Write-Host ("Storage: " + $TT.Storage) -ForegroundColor White
    $reg = Get-CompatRegistry
    if ($reg -ne $null -and $reg.storage -ne $null) {
      $exp = ($reg.storage.expected -join "/")
      Write-Host ((L "Registry expects: " "Registry erwartet: ") + $exp) -ForegroundColor Gray
    }
    $emui = [string]$TT.Props["ro.build.version.emui"]
    if ([string]::IsNullOrEmpty($emui)) { $emui = [string]$TT.Props["ro.emui.version"] }
    Write-Host ((L "Vendor advice: " "Vendor-Hinweis: ") + (Get-VendorAdvice $emui $rel)) -ForegroundColor Gray
  } elseif ($TT.Mode -eq "fastboot") {
    Write-Host (L "Device in fastboot -> Android analysis after reboot, fastboot analysis now." "Geraet in Fastboot -> erst Android-Analyse nach Reboot, jetzt Fastboot-Analyse.") -ForegroundColor Yellow
    Invoke-TTFastbootAnalysis | Out-Null
    Write-Host ($TT.FbRaw) -ForegroundColor White
  } else {
    Write-Host (L "No device connected." "Kein Geraet verbunden.") -ForegroundColor Red
  }
  # Installed ROM: ask once, remember answer, allow change here.
  Load-InstalledRom | Out-Null
  Write-Host ""
  if ($TT.InstalledRom -eq "") {
    Write-Host (L "I don't know your system yet - one question:" "Ich kenne dein System noch nicht - eine Frage:") -ForegroundColor Yellow
    Select-InstalledRom | Out-Null
  } else {
    Write-Host ((L "Phone runs (saved): " "Handy laeuft mit (gespeichert): ") + (Get-RomLabel $TT.InstalledRom)) -ForegroundColor Cyan
    Write-Host (L "[C] change system  [Enter] keep: " "[C] System aendern  [Enter] behalten: ") -NoNewline -ForegroundColor Yellow
    $rc = Read-Host
    if ($rc -eq "C" -or $rc -eq "c") { Select-InstalledRom | Out-Null }
  }
  # Report
  $rep = Join-Path $TTLogDir ("analyze-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".txt")
  @("OS: $(if ($TT.OS) { $TT.OS.Kind } else { '?' })",
    "Props:", (($TT.Props.GetEnumerator() | ForEach-Object { "$($_.Key) = $($_.Value)" }) -join "`n"),
    "", "by-name:", $TT.ByNameRaw) -join "`n" | Out-File $rep -Encoding utf8
  Write-TTLog ((L "Analysis report: " "Analyse-Report: ") + $rep) "SUCCESS"
  Pause-TT
}

function Screen-Firmware {
  Show-TTHeader (L "Step 3 - Firmware (determine compatible, no dubious downloads)" "Step 3 - Firmware (kompatibel bestimmen, keine dubiosen Downloads)")
  $base = Get-TTFirmwareBaseline -Interactive
  Write-Host ""
  Write-Host ("Baseline: " + $(if ($base -ne "") { $base } else { "(empty)" })) -ForegroundColor White
  Write-Host ""
  Write-Host (L "Sources (in UI, no auto-download from dubious sources):" "Quellen (im UI, kein Auto-Download aus dubiosen Quellen):") -ForegroundColor Cyan
  Write-Host " - HUAWEI FIRM FINDER V2: https://professorjtj.github.io/v2/" -ForegroundColor White
  Write-Host " - Discussion #2542: VTR-L29 9.1.0.297(C432E5R1P9) as EXAMPLE (not exclusive)" -ForegroundColor White
  Write-Host " - Wiki P10: EMUI 9.1 base, PotatoNV unlock, RECOVERY_RAMDIS.img from UPDATE.APP" -ForegroundColor White
  $region = ""
  $m = [regex]::Match([string]$base, "\(C(\d+)")
  if ($m.Success) { $region = "C" + $m.Groups[1].Value }
  else {
    Write-Host (L "Region/CUST (e.g. C432, Enter=unknown): " "Region/CUST (z.B. C432, Enter=unbekannt): ") -NoNewline -ForegroundColor Yellow
    $region = Read-Host
  }
  $model = $TT.ProfileId
  if ([string]::IsNullOrEmpty($model)) { $model = "VTR-L29" }
  $TT.FirmwareCompat = Test-FirmwareCompatibility $model $base $region
  Write-Host ""
  Write-Host ((L "Compatibility: " "Kompatibilitaet: ") + $TT.FirmwareCompat.Status) -ForegroundColor $(if ($TT.FirmwareCompat.Status -eq "PASS") { "Green" } elseif ($TT.FirmwareCompat.Status -eq "WARN") { "Yellow" } else { "Red" })
  foreach ($r in $TT.FirmwareCompat.Reasons) { Write-Host (" - " + $r) -ForegroundColor Gray }
  if ($TT.FirmwareCompat.Status -eq "FAIL") { Write-Host (L "FAIL -> do NOT continue, pick another firmware." "FAIL -> NICHT fortfahren, andere Firmware waehlen.") -ForegroundColor Red }
  Write-Host ""
  Write-Host (L "[D] Download stock firmware (with progress)  [Enter] back" "[D] Stock-Firmware herunterladen (mit Fortschritt)  [Enter] zurueck") -ForegroundColor DarkGray -NoNewline
  $k = [Console]::ReadKey($true)
  Write-Host ""
  if ($k.KeyChar -eq "d" -or $k.KeyChar -eq "D") { Screen-DownloadFirmware }
  else { return }
}

function Screen-DownloadFirmware {
  Show-TTHeader (L "Stock firmware download (original, with progress + confirmation)" "Stock-Firmware Download (original, mit Fortschritt + Bestaetigung)")
  Write-Host ""
  Write-Host (L "Goal: re-install original firmware OR just own it (UPDATE.APP -> step 4)." "Ziel: Original-Firmware neu installieren ODER nur besitzen (UPDATE.APP -> Step 4).") -ForegroundColor Gray
  Write-Host (L "Location: data/firmware/ (offline cache, no re-download). Full packages only (no deltas)." "Ablage: data/firmware/ (Offline-Cache, kein Re-Download). Nur Full-Pakete (keine Deltas).") -ForegroundColor Gray
  Write-Host ""
  Write-Host (L "Trusted sources (honest, no guaranteed direct link 2026):" "Zuverlaessige Quellen (ehrlich, kein garantierter Direktlink 2026):") -ForegroundColor Cyan
  $i = 1
  foreach ($s in $FirmwareSources) {
    $nm = $s.Name; $nt = $s.Note
    if ($TTLang -eq "de") { $nm = $s.NameDe; $nt = $s.NoteDe }
    Write-Host (" [$i] " + $nm + " [" + $s.Kind + "]") -ForegroundColor White
    Write-Host ("     " + $s.Url) -ForegroundColor DarkGray
    Write-Host ("     " + $nt) -ForegroundColor Gray
    $i++
  }
  Write-Host ""
  Write-Host (L "Free space: " "Freier Platz: ") -NoNewline -ForegroundColor Gray
  try {
    $drv = (Get-Item $TTFirmDir).PSDrive.Name
    $vol = Get-WmiObject Win32_LogicalDisk -Filter ("DeviceID='" + $drv + ":'") -ErrorAction SilentlyContinue
    if ($vol) { Write-Host ([math]::Round($vol.FreeSpace/1GB,1).ToString() + (L " GB free on " " GB frei auf ") + $drv + (L ": (full firmware approx. 2-4 GB)" ": (Full-Firmware ca. 2-4 GB)")) -ForegroundColor White }
    else { Write-Host (L "(unknown, plan approx. 2-4 GB)" "(unbekannt, ca. 2-4 GB einplanen)") -ForegroundColor White }
  } catch { Write-Host (L "(unknown, plan approx. 2-4 GB)" "(unbekannt, ca. 2-4 GB einplanen)") -ForegroundColor White }
  Write-Host ""
  Write-Host (L "Paste direct link to full firmware ZIP (Enter = just open source/abort): " "Direktlink zum Full-Firmware-ZIP einfuegen (Enter = nur Quelle oeffnen/abbruch): ") -NoNewline -ForegroundColor Yellow
  $url = Read-Host
  if ([string]::IsNullOrWhiteSpace($url)) {
    Write-Host (L "No download started. Open source in browser? [Y/N]: " "Kein Download gestartet. Quelle im Browser oeffnen? [J/N]: ") -NoNewline -ForegroundColor Yellow
    $o = Read-Host
    if ($o -eq "J" -or $o -eq "j" -or $o -eq "Y" -or $o -eq "y") {
      try { Start-Process $FirmwareSources[3].Url | Out-Null } catch { Write-TTLog $_.Exception.Message "ERROR" }
    }
    Pause-TT; return
  }
  $chk = Test-FirmwareUrl $url
  if (-not $chk.Ok) { Write-Host ((L "URL rejected: " "URL abgelehnt: ") + $chk.Reason) -ForegroundColor Red; Write-TTLog ((L "URL rejected: " "URL abgelehnt: ") + $chk.Reason) "ERROR"; Pause-TT; return }
  $fname = "stock-firmware-" + $TT.ProfileId + "-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".zip"
  $low = $url.Split("?")[0].ToLower()
  foreach ($e in @(".zip",".7z",".tar",".gz",".tgz",".app",".rar")) {
    if ($low.EndsWith($e)) {
      try { $fname = "stock-firmware-" + $TT.ProfileId + "-" + [System.IO.Path]::GetFileName($low.Split("/")[-1]) } catch {}
    }
  }
  $out = Join-Path $TTFirmDir $fname
  Write-Host ""
  Write-Host (L "=== Confirmation (required before download) ===" "=== Bestaetigung (Pflicht vor Download) ===") -ForegroundColor Cyan
  Write-Host ("URL   : " + $url) -ForegroundColor White
  Write-Host ("Host  : " + $chk.Host) -ForegroundColor White
  Write-Host ((L "Target: " "Ziel  : ") + $out) -ForegroundColor White
  Write-Host (L "Note: verify source + hash after download. Full firmware only, watch region/CUST." "Hinweis: Quelle + Hash nach Download pruefen. Nur Full-Firmware, Region/CUST beachten.") -ForegroundColor Yellow
  Write-Host (L "Really download? Type 'YES' (approx. 2-4 GB): " "Wirklich herunterladen? 'JA' tippen (ca. 2-4 GB): ") -NoNewline -ForegroundColor Yellow
  $c = Read-Host
  if ($c -ne "JA" -and $c -ne "YES") { Write-TTLog (L "Download aborted (no YES)." "Download abgebrochen (kein JA).") "WARNING"; Pause-TT; return }
  $res = Invoke-FirmwareDownload $url $out
  if (-not $res.Ok) { Write-Host (L "Download failed, see log." "Download fehlgeschlagen, siehe Log.") -ForegroundColor Red; Pause-TT; return }
  if ($res.Cached) { Write-Host (L "Already present (cache), no re-download." "Bereits vorhanden (Cache), kein Re-Download.") -ForegroundColor Yellow }
  $ver = Test-DownloadedFirmware $out
  Write-Host ""
  foreach ($n in $ver.Notes) { Write-Host (" - " + $n) -ForegroundColor White }
  Write-Host ("SHA-256: " + $ver.SHA256) -ForegroundColor Gray
  if ($ver.Ok) { Write-TTLog ((L "Firmware ready: " "Firmware bereit: ") + $out) "SUCCESS"; Write-Host (L "Next: TUI step 4 (UPDATE.APP -> RECOVERY_RAMDISK)." "Weiter: TUI Step 4 (UPDATE.APP -> RECOVERY_RAMDISK).") -ForegroundColor Green }
  else { Write-Host (L "WARN: package implausible (see above). Do NOT use as restore base, pick another source." "WARN: Paket unplausibel (siehe oben). NICHT als Restore-Basis nutzen, andere Quelle waehlen.") -ForegroundColor Red }
  Pause-TT
}

function Screen-Extract {
  Show-TTHeader (L "Step 4 - Extract/validate RECOVERY_RAMDISK (UPDATE.APP)" "Step 4 - RECOVERY_RAMDISK extrahieren/validieren (UPDATE.APP)")
  Write-Host ""
  Write-Host (L "Expected: UPDATE.APP (full firmware) placed in data/firmware/." "Erwartet: UPDATE.APP (Full-Firmware) in data/firmware/ ablegen.") -ForegroundColor Gray
  $apps = Get-ChildItem -Path $TTFirmDir -Filter "*.APP" -File -Recurse -ErrorAction SilentlyContinue
  if ($apps) {
    Write-Host (L "Found:" "Gefunden:") -ForegroundColor Cyan
    $i = 1
    foreach ($a in $apps) { Write-Host (" [$i] " + $a.FullName + " (" + [math]::Round($a.Length/1MB,1) + " MB)"); $i++ }
  } else { Write-Host (L "No UPDATE.APP in data/firmware/." "Keine UPDATE.APP in data/firmware/.") -ForegroundColor Yellow }
  Write-Host ""
  Write-Host (L "UPDATE.APP path (Enter = skip, only search existing images): " "UPDATE.APP-Pfad (Enter = ueberspringen, nur vorhandene Images suchen): ") -NoNewline -ForegroundColor Yellow
  $p = Unquote-Path (Read-Host)
  if ($p -eq "" -and $apps -and $apps.Count -ge 1) { $p = $apps[0].FullName }
  if ($p -ne "" -and (Test-Path $p)) {
    $info = Invoke-TTUpdateAppAnalysis $p
    Write-Host ""
    Write-Host ((L "Size: " "Groesse: ") + $info.Size) -ForegroundColor White
    Write-Host ("Header: " + $info.HeaderHex) -ForegroundColor Gray
    Write-Host $info.Note -ForegroundColor Yellow
    Write-Host ""
    Write-Host (L "Extractors (place in data/tools/ if available): huawei-update-extractor / splitupdate." "Extraktoren (in data/tools/ ablegen, falls vorhanden): huawei-update-extractor / splitupdate.") -ForegroundColor Gray
    Write-Host (L "Manual: open UPDATE.APP with a trusted extractor, copy RECOVERY_RAMDIS(K).img to data/firmware/." "Manuell: UPDATE.APP mit vertrauenswuerdigem Extractor oeffnen, RECOVERY_RAMDIS(K).img nach data/firmware/ kopieren.") -ForegroundColor Gray
    Write-Host (L "IMPORTANT: keep exact name (RECOVERY_RAMDIS.img vs RECOVERY_RAMDISK.img), do NOT rename." "WICHTIG: Exakte Bezeichnung beibehalten (RECOVERY_RAMDIS.img vs RECOVERY_RAMDISK.img), NICHT umbenennen.") -ForegroundColor Red
  }
  $found = Find-TTRecoveryImage
  Write-Host ""
  if ($found.Count -gt 0) {
    Write-Host (L "Recovery images found:" "RECOVERY-Images gefunden:") -ForegroundColor Green
    foreach ($f in $found) { Write-Host (" - " + $f) -ForegroundColor White }
    $TT.StockImage = $found[0]
    $chk = Test-RecoveryImageFile $TT.StockImage
    Write-Host ((L "Check: " "Pruefung: ") + $chk.Verdict + " | Size=" + $chk.Hash.Size + " | " + $chk.HeaderHex) -ForegroundColor White
    foreach ($n in $chk.Notes) { Write-Host ("   " + $n) -ForegroundColor Gray }
    $TT.StockHash = $chk.Hash
    if ($chk.Verdict -ne "PASS") { Write-Host (L "Image not verifiable -> Cannot safely flash." "Image nicht verifizierbar -> Cannot safely flash.") -ForegroundColor Red }
  } else {
    Write-Host (L "No RECOVERY_RAMDIS(K).img found. Obtain it first, then continue." "Keine RECOVERY_RAMDIS(K).img gefunden. Erst beschaffen, dann fortfahren.") -ForegroundColor Red
  }
  Pause-TT
}

function Screen-ExportRecovery {
  Show-TTHeader (L "Recovery export from compatible custom ROMs" "Recovery-Export aus kompatiblen Custom-ROMs")
  if ($TT.InstalledRom -ne "" -and $TT.InstalledRom -ne "stock") {
    Write-Host ""
    Write-Host ((L "Exporting base from YOUR rom: " "Exportiere Basis aus DEINEM ROM: ") + (Get-RomLabel $TT.InstalledRom)) -ForegroundColor Cyan
    Write-Host (L "This file becomes your Magisk patch base. Stock firmware is NOT used." "Diese Datei wird deine Magisk-Patch-Basis. Stock-Firmware wird NICHT benutzt.") -ForegroundColor White
  }
  Write-Host ""
  Write-Host (L "Drop ROM packages into data/roms/ (.zip/.tar.gz/.img.gz/.img.xz with boot/recovery.img or payload.bin, or .img directly)." "ROM-Pakete nach data/roms/ legen (.zip/.tar.gz/.img.gz/.img.xz mit boot/recovery.img oder payload.bin, oder .img direkt).") -ForegroundColor Gray
  Write-Host (L "GSI system images contain no recovery and are refused honestly." "GSI-System-Images enthalten kein Recovery und werden ehrlich abgelehnt.") -ForegroundColor Gray
  Write-Host ""
  $roms = @()
  foreach ($ext in @("*.zip","*.img","*.tar","*.tar.gz","*.tgz","*.gz","*.xz")) {
    $hits = Get-ChildItem -Path $TTRomDir -Filter $ext -File -ErrorAction SilentlyContinue
    foreach ($h in $hits) { $roms += $h.FullName }
  }
  if ($roms.Count -eq 0) {
    Write-Host (L "No ROM packages in data/roms/. Place file there first." "Keine ROM-Pakete in data/roms/. Erst Datei dort ablegen.") -ForegroundColor Yellow
    Write-Host (L "ROM path (Enter=abort): " "ROM-Pfad (Enter=Abbruch): ") -NoNewline -ForegroundColor Yellow
    $p = Read-Host
    if ([string]::IsNullOrWhiteSpace($p)) { Pause-TT; return }
    $roms = @(Unquote-Path $p)
  } else {
    Write-Host (L "Found:" "Gefunden:") -ForegroundColor Cyan
    for ($i = 0; $i -lt $roms.Count; $i++) { Write-Host (" [" + ($i+1) + "] " + $roms[$i]) -ForegroundColor White }
    Write-Host (L "Number (Enter=all): " "Nummer (Enter=alle): ") -NoNewline -ForegroundColor Yellow
    $sel = Read-Host
    if ($sel -match "^\d+$") {
      $idx = [int]$sel - 1
      if ($idx -ge 0 -and $idx -lt $roms.Count) { $roms = @($roms[$idx]) }
    }
  }
  foreach ($r in $roms) {
    Write-Host ""
    Write-Host ("=> " + $r) -ForegroundColor Cyan
    $e = Export-RecoveryFromRom $r
    foreach ($n in $e.Notes) { Write-Host (" - " + $n) -ForegroundColor Gray }
    foreach ($f in $e.Files) { Write-Host (" [+] " + $f) -ForegroundColor Green }
    if ($e.Ok) {
      Write-TTLog ("Recovery export OK: " + $e.Dir) "SUCCESS"
      Write-Host (L "Next: patch exported image in Magisk app (Select and Patch a File), then flash ONLY to the profile target partition after safety gate. Rights are granted in the Magisk app (uid=0 verify)." "Weiter: exportiertes Image in Magisk-App patchen (Select and Patch a File), dann NUR auf Zielpartition des Profils nach Safety-Gate flashen. Rechte werden in Magisk-App vergeben (uid=0 Verify).") -ForegroundColor Green
    } else { Write-TTLog ("Recovery export failed: " + $r) "ERROR" }
  }
  Pause-TT
}

function Get-MagiskStable {
  # Official Magisk stable.json (version + APK link). Never hardcoded.
  try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $wc = New-Object Net.WebClient
    $wc.Headers.Add("User-Agent", "trebleManager")
    $url = "https://raw.githubusercontent.com/topjohnwu/magisk-files/master/stable.json"
    $reg = Get-CompatRegistry
    if ($reg -ne $null -and $reg.magisk -ne $null -and -not [string]::IsNullOrWhiteSpace([string]$reg.magisk.stable_json)) {
      $url = [string]$reg.magisk.stable_json
    }
    $j = $wc.DownloadString($url) | ConvertFrom-Json
    return @{ Ok = $true; Version = [string]$j.magisk.version; Code = [string]$j.magisk.versionCode; Link = [string]$j.magisk.link }
  } catch { return @{ Ok = $false; Version = ""; Code = ""; Link = "" } }
}

function Invoke-MagiskDownload {
  # Downloads the official Magisk APK into data/magisk. Returns path or "".
  param([switch]$ForceYes)
  $st = Get-MagiskStable
  if (-not $st.Ok -or [string]::IsNullOrWhiteSpace($st.Link)) {
    Write-Host (L "Magisk stable info unreachable (offline?). Place the APK from https://github.com/topjohnwu/Magisk/releases into data/magisk/ manually." "Magisk-Stable-Info unerreichbar (offline?). APK von https://github.com/topjohnwu/Magisk/releases manuell nach data/magisk/ legen.") -ForegroundColor Yellow
    return ""
  }
  if (-not (Test-Path $TTMagDir)) { New-Item -ItemType Directory -Path $TTMagDir -Force | Out-Null }
  $fname = "Magisk-v" + $st.Version + ".apk"
  $dst = Join-Path $TTMagDir $fname
  if (Test-Path $dst) {
    Write-Host ((L "Magisk cached: " "Magisk gecached: ") + $dst) -ForegroundColor Gray
    return $dst
  }
  if (-not $ForceYes) {
    Write-Host ((L "Fetch official Magisk v" "Offizielles Magisk v") + $st.Version + " ? [Y/n]: ") -NoNewline -ForegroundColor Cyan
    $a = Read-Host
    if ($a -ne "" -and $a -ne "Y" -and $a -ne "y" -and $a -ne "J" -and $a -ne "j") { return "" }
  }
  try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $wc2 = New-Object Net.WebClient
    $wc2.Headers.Add("User-Agent", "trebleManager")
    Write-Host ((L "Downloading Magisk v" "Lade Magisk v") + $st.Version + " ...") -ForegroundColor Cyan
    $wc2.DownloadFile($st.Link, $dst)
    $h = Get-FileHashInfo $dst
    if ($h -ne $null) { $h.SHA256 | Out-File ($dst + ".sha256") -Encoding ascii }
    Write-TTLog ("Magisk downloaded: " + $dst) "SUCCESS"
    return $dst
  } catch {
    Write-TTLog (("Magisk download failed: " + $_.Exception.Message)) "ERROR"
    if (Test-Path $dst) { Remove-Item $dst -Force -ErrorAction SilentlyContinue }
    return ""
  }
}

function Screen-Patch {
  Show-TTHeader (L "Step 5 - Magisk (choose compatible, patch for real)" "Step 5 - Magisk (kompatibel waehlen, echt patchen)")
  Write-Host ""
  Write-Host (L "Compatibility (never blind latest):" "Kompatibilitaet (nicht blind latest):") -ForegroundColor Cyan
  foreach ($r in $MagiskCompatTable) { Write-Host (" - " + $r.Android + ": " + $r.Tested + " -- " + $r.Note) -ForegroundColor Gray }
  Write-Host ""
  Write-Host "Official: https://github.com/topjohnwu/Magisk/releases" -ForegroundColor White
  $apk = Find-TTMagiskApk
  if ($apk -eq "") {
    Write-Host (L "No Magisk APK in data/magisk/. Place APK there (official GitHub)." "Keine Magisk-APK in data/magisk/. APK dort ablegen (offizielles GitHub).") -ForegroundColor Yellow
    Write-Host (L "APK path (Enter=fetch automatically): " "APK-Pfad (Enter=automatisch laden): ") -NoNewline -ForegroundColor Yellow
    $p = Unquote-Path (Read-Host)
    if ($p -ne "" -and (Test-Path $p)) {
      $dst = Join-Path $TTMagDir ([System.IO.Path]::GetFileName($p))
      Copy-Item $p $dst -Force
      $apk = $dst
    } else {
      $apk = Invoke-MagiskDownload
    }
  }
  if ($apk -ne "") {
    $TT.MagiskApk = $apk
    $TT.MagiskInfo = Get-TTMagiskInfo $apk
    Write-Host ("APK: " + $TT.MagiskInfo.Path) -ForegroundColor White
    Write-Host ("Version: " + $TT.MagiskInfo.Version + " | SHA256: " + $TT.MagiskInfo.SHA256) -ForegroundColor White
  }
  $base = Get-PatchBase
  if ($base.Source -eq "rom") {
    Write-Host ""
    Write-Host ((L "Phone runs: " "Handy laeuft mit: ") + $base.RomLabel + (L " (full device ROM)" " (volles Device-ROM)")) -ForegroundColor Cyan
    Write-Host (L "RULE: patch base MUST come from this ROM package. NOT from stock firmware." "REGEL: Patch-Basis MUSS aus diesem ROM-Paket kommen. NICHT aus Stock-Firmware.") -ForegroundColor Red
  } elseif ($base.Source -eq "stock-gsi") {
    Write-Host ""
    Write-Host ((L "Phone runs: " "Handy laeuft mit: ") + $base.RomLabel + (L " (GSI, system-only)" " (GSI, nur System)")) -ForegroundColor Cyan
    Write-Host (L "A GSI never touches recovery: your recovery_ramdisk is still stock, so the patch base IS the stock recovery. Correct, not a workaround." "Ein GSI fasst Recovery nie an: dein recovery_ramdisk ist weiter Stock, also ist die Patch-Basis das Stock-Recovery. Korrekt, kein Workaround.") -ForegroundColor Green
  }
  if ($base.Image -eq "" -or -not (Test-Path $base.Image)) {
    if ($base.Source -eq "rom") {
      Write-Host (L "No ROM base image yet. Open recovery export now? [Y/n]: " "Noch kein ROM-Basis-Image. Jetzt Recovery-Export oeffnen? [J/n]: ") -NoNewline -ForegroundColor Yellow
      $oe = Read-Host
      if ($oe -eq "" -or $oe -eq "Y" -or $oe -eq "y" -or $oe -eq "J" -or $oe -eq "j") { Screen-ExportRecovery }
      $base = Get-PatchBase
    }
  }
  if ($base.Image -eq "" -or -not (Test-Path $base.Image)) {
    if ($base.Source -eq "rom") {
      Write-Host (L "Still no ROM base image: put the ROM package into data/roms/ and export first." "Immer noch kein ROM-Basis-Image: ROM-Paket nach data/roms/ legen und erst exportieren.") -ForegroundColor Red
    } else {
      Write-Host (L "No stock image registered -> step 4 first." "Kein Stock-Image registriert -> erst Step 4.") -ForegroundColor Red
    }
    Pause-TT; return
  }
  $TT.PatchBaseImage = $base.Image; $TT.PatchBaseHash = $base.Hash; $TT.PatchBaseSource = $base.Source
  Write-Host ""
  Write-Host ("Input: " + $base.Image) -ForegroundColor White
  Write-Host (L "Source: " "Quelle: ") -NoNewline -ForegroundColor White
  $srcNote = " (stock)"
  $srcCol = "White"
  if ($base.Source -eq "rom") { $srcNote = (L " (this ROM - correct)" " (dieses ROM - korrekt)"); $srcCol = "Green" }
  elseif ($base.Source -eq "stock-gsi") { $srcNote = (L " (stock recovery - correct for GSI)" " (Stock-Recovery - korrekt fuer GSI)"); $srcCol = "Green" }
  Write-Host ($base.RomLabel + $srcNote) -ForegroundColor $srcCol
  Write-Host ("Target partition: " + $DeviceProfiles[$TT.ProfileId].TargetPartition) -ForegroundColor White
  Write-Host ("Device: Huawei " + $TT.ProfileId) -ForegroundColor White
  Write-Host ("Firmware: " + $TT.FirmwareBaseline) -ForegroundColor White
  Write-Host ("Magisk: " + $(if ($TT.MagiskInfo) { $TT.MagiskInfo.Version } else { (L "(APK missing)" "(APK fehlt)") })) -ForegroundColor White
  Write-Host ("SHA-256: " + $base.Hash.SHA256) -ForegroundColor White
  Write-Host (L "Explicit confirmation required before patch prep (never patch boot.img/recovery.img)." "Explizite Bestaetigung vor Patch-Vorbereitung noetig (kein boot.img/recovery.img patchen).") -ForegroundColor Yellow
  Write-Host (L "[1] Prepare patch (to-patch + instructions)  [2] Register patched file  [Esc]" "[1] Patch vorbereiten (to-patch + Anleitung)  [2] Gepatchte Datei registrieren  [Esc]") -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  if ($k.KeyChar -eq "1") {
    Prepare-TTMagiskPatch $base.Image | Out-Null
    Write-Host (L "Prepared. Now patch on device (instructions in data/magisk/to-patch/)." "Vorbereitet. Jetzt am Geraet patchen (Anleitung in data/magisk/to-patch/).") -ForegroundColor Green
    Write-Host ((L "In the Magisk app select EXACTLY this file: " "In der Magisk-App EXAKT diese Datei auswaehlen: ") + [System.IO.Path]::GetFileName($base.Image)) -ForegroundColor Cyan
  } elseif ($k.KeyChar -eq "2") {
    Write-Host ""
    # real adb pull attempt, no mock
    Write-Host (L "Trying adb pull /sdcard/Download/magisk_patched-*.img ..." "Versuche adb pull /sdcard/Download/magisk_patched-*.img ...") -ForegroundColor Gray
    try {
      $lst = (Invoke-TTAdb @("shell","ls /sdcard/Download/magisk_patched*.img 2>&1") -join "`n").Trim()
      Write-Host $lst -ForegroundColor White
    } catch {}
    Write-Host (L "Path to patched file (data/magisk/*.img): " "Pfad zur gepatchten Datei (data/magisk/*.img): ") -NoNewline -ForegroundColor Yellow
    $pp = Unquote-Path (Read-Host)
    if ($pp.ToLower().EndsWith(".gz") -and (Test-Path $pp)) {
      Write-Host (L "GZip file: decompressing first ..." "GZip-Datei: erst dekomprimieren ...") -ForegroundColor Gray
      $pp = Expand-TTGzipImage $pp $TTMagDir
      if ($pp -eq "") { Write-Host (L "Decompress failed." "Dekomprimieren fehlgeschlagen.") -ForegroundColor Red }
    }
    if ($pp -ne "" -and (Test-Path $pp)) {
      $chk = Test-RecoveryImageFile $pp
      $baseSha = ""
      if ($base.Hash -ne $null) { $baseSha = $base.Hash.SHA256 }
      if ($baseSha -ne "" -and $chk.Hash.SHA256 -eq $baseSha) {
        Write-Host (L "ERROR: patched == base (identical hash). NO fake patch accepted." "FEHLER: Gepatcht == Basis (Hash identisch). KEIN Fake-Patch akzeptiert.") -ForegroundColor Red
        Write-TTLog (L "Fake patch rejected (identical hash)." "Fake-Patch abgelehnt (Hash identisch).") "ERROR"
      } elseif ($chk.Verdict -eq "PASS") {
        $TT.PatchedImage = $pp; $TT.PatchedHash = $chk.Hash
        Write-TTLog ("Patched registered: $pp SHA256=" + $chk.Hash.SHA256) "SUCCESS"
      } else {
        Write-Host (L "Image check FAIL, details above." "Image-Pruefung FAIL, Details oben.") -ForegroundColor Red
      }
    }
  }
  Pause-TT
}

function Screen-Backup {
  Show-TTHeader (L "Step 6 - Backup (mandatory before flash)" "Step 6 - Backup (Pflicht vor Flash)")
  $base = Get-PatchBase
  if ($base.Image -eq "" -or -not (Test-Path $base.Image)) {
    if ($base.Source -eq "rom") { Write-Host (L "No ROM base image -> recovery export first." "Kein ROM-Basis-Image -> erst Recovery-Export.") -ForegroundColor Red }
    else { Write-Host (L "No stock image -> step 4 first." "Kein Stock-Image -> erst Step 4.") -ForegroundColor Red }
    Pause-TT; return
  }
  Write-Host ((L "Backing up base from: " "Sichere Basis aus: ") + $base.RomLabel) -ForegroundColor Cyan
  $r = New-TTBackup $base.Image
  Write-Host ""
  Write-Host ("Backup: " + $r.Dir + " OK=" + $r.Ok) -ForegroundColor White
  Write-Host (L "Contains: original.img + metadata.json + sha256.txt (+partition-probe)." "Enthaelt: original.img + metadata.json + sha256.txt (+partition-probe).") -ForegroundColor Gray
  Pause-TT
}

function Screen-Flash {
  Show-TTHeader (L "Step 7 - Flash (only after safety gate, derived from profile)" "Step 7 - Flash (nur nach Safety-Gate, abgeleitet aus Profil)")
  if ($TT.Mode -ne "fastboot") {
    Write-Host (L "Not in fastboot. 'adb reboot bootloader' + wait now? [Y/N]: " "Nicht in Fastboot. Jetzt 'adb reboot bootloader' + warten? [J/N]: ") -NoNewline -ForegroundColor Yellow
    $k = [Console]::ReadKey($true); Write-Host $k.KeyChar
    if ($k.KeyChar -eq "j" -or $k.KeyChar -eq "J" -or $k.KeyChar -eq "y" -or $k.KeyChar -eq "Y") {
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
  Show-TTHeader (L "Step 8+9 - Reboot (Huawei procedure) + verify (real)" "Step 8+9 - Reboot (Huawei-Prozedur) + Verify (echt)")
  $r = Invoke-TTRootVerification
  Save-RootState $r.Root
  Write-Host ""
  Write-Host ((L "Result: " "Ergebnis: ") + $r.Root) -ForegroundColor $(if ($r.Root -eq "ROOTED") { "Green" } else { "Yellow" })
  if ($r.Root -ne "ROOTED") {
    Write-Host (L "No faked success: boot alone != root. Repeat trick, check Magisk app, create diagnostic ZIP." "Kein Erfolg vortaeuschen: Boot allein != Root. Trick wiederholen, Magisk-App pruefen, Diagnose-ZIP erzeugen.") -ForegroundColor Yellow
  } elseif (-not $TT.SkipFixOffer) {
    Write-Host ""
    Write-Host (L "Root is live right now - install the permanent APTouch fix? (service.d script + immediate stop, non-destructive, removable) [Y/n]: " "Root ist gerade live - permanenten APTouch-Fix installieren? (service.d-Skript + sofortiger Stopp, zerstoerungsfrei, entfernbar) [J/n]: ") -NoNewline -ForegroundColor Cyan
    $pf = Read-Host
    if ($pf -eq "" -or $pf -eq "Y" -or $pf -eq "y" -or $pf -eq "J" -or $pf -eq "j") {
      if (Install-PersistFixes) {
        Write-Host (L "APTouch fix active now and on every rooted boot." "APTouch-Fix jetzt aktiv und bei jedem gerooteten Boot.") -ForegroundColor Green
      } else {
        Write-Host (L "Persist install failed - will offer again next rooted boot. Immediate stop was attempted (see log)." "Persist-Install fehlgeschlagen - wird beim naechsten Root-Boot erneut angeboten. Sofort-Stopp wurde versucht (siehe Log).") -ForegroundColor Yellow
      }
    }
    Write-Host (L "Want every power-on rooted (no Vol-Up trick)? Boot tricks -> persistent boot. Conscious choice: it needs an eRecovery wipe." "Jeden Power-On gerootet (ohne Vol-Up-Trick)? Boot-Tricks -> persistenter Boot. Bewusste Entscheidung: braucht eRecovery-Wipe.") -ForegroundColor Gray
  }
  Pause-TT
}

function Screen-Restore {
  Show-TTHeader (L "Restore / unroot (safe restore: original partition only)" "Restore / Unroot (Safe Restore: nur Original-Partition)")
  Invoke-TTRestoreFlow | Out-Null
  Pause-TT
}

function Screen-Unlock {
  Show-TTHeader (L "Bootloader unlock (PotatoNV, wiki method - guided only)" "Bootloader-Unlock (PotatoNV, Wiki-Methode - nur Anleitung)")
  Write-Host ""
  Write-Host (L "The tool NEVER unlocks anything itself." "Das Tool unlockt NIEMALS selbst.") -ForegroundColor Red
  foreach ($s in $WikiKnowledge.UnlockSteps) { Write-Host (" - " + $s) -ForegroundColor White }
  Write-Host ""
  Write-Host (L "Already booting a GSI? Then unlock is done - continue with step 2." "Bootet bereits ein GSI? Dann ist Unlock erledigt - weiter mit Step 2.") -ForegroundColor Green
  Pause-TT
}

function Screen-KernelFixes {
  Show-TTHeader (L "Kernels + known fixes (P10 wiki)" "Kernel + bekannte Fixes (P10-Wiki)")
  Write-Host ""
  Write-Host (L "Kernels (permissive SELinux for some GSIs):" "Kernel (permissive SELinux fuer manche GSIs):") -ForegroundColor Cyan
  foreach ($s in $WikiKnowledge.KernelNotes) { Write-Host (" - " + $s) -ForegroundColor White }
  Write-Host ""
  Write-Host (L "Fixes (need root, verified uid=0 first):" "Fixes (brauchen Root, erst uid=0 verifizieren):") -ForegroundColor Cyan
  foreach ($s in $WikiKnowledge.Fixes) { Write-Host (" - " + $s) -ForegroundColor White }
  Write-Host ""
  Write-Host (L "TWRP rule: factory reset ONLY via stock recovery - TWRP wipe breaks userdata." "TWRP-Regel: Factory-Reset NUR via Stock-Recovery - TWRP-Wipe zerstoert userdata.") -ForegroundColor Red
  Write-Host (L "GApps: MindTheGapps (OpenGApps fails on Oreo GSIs; Pie+ works)." "GApps: MindTheGapps (OpenGApps scheitert auf Oreo-GSIs; Pie+ geht).") -ForegroundColor Gray
  Pause-TT
}

function Test-SystemImageFile {
  param([string]$Path)
  $res = @{ Exists = $false; SizeOk = $false; Header = ""; Verdict = "FAIL"; Notes = @() }
  if (-not (Test-Path $Path)) { $res.Notes += (L "File missing: " "Datei fehlt: ") + $Path; return $res }
  $res.Exists = $true
  $len = (Get-Item $Path).Length
  if ($len -ge 500MB) { $res.SizeOk = $true }
  else { $res.Notes += (L "Size implausible for a system image (< 500 MB). Full GSI expected, no delta/OTA." "Groesse unplausibel fuer System-Image (< 500 MB). Full-GSI erwartet, kein Delta/OTA.") }
  try {
    $fs = [System.IO.File]::OpenRead($Path)
    $buf = New-Object byte[] 8
    [void]$fs.Read($buf, 0, 8)
    $fs.Close()
    $hex = (($buf[0..3] | ForEach-Object { $_.ToString("X2") }) -join " ")
    $res.Header = $hex
    if ($hex -eq "3A FF 26 ED") { $res.Notes += "Android sparse image detected." }
    elseif ($hex.StartsWith("53 EF")) { $res.Notes += "ext4 image detected." }
    else { $res.Notes += (L "Header unspecific - verify it is a real system/GSI image." "Header unspezifisch - verifizieren dass es ein echtes System/GSI-Image ist.") }
  } catch { $res.Notes += (L "Header unreadable: " "Header nicht lesbar: ") + $_.Exception.Message }
  $fn = [System.IO.Path]::GetFileName($Path).ToLower()
  if ($fn -match "arm64") { $res.Notes += "Filename suggests arm64." }
  else { $res.Notes += (L "WARN: filename does not suggest arm64 - P10 needs arm64 A-only." "WARN: Dateiname deutet nicht auf arm64 - P10 braucht arm64 A-only.") }
  if ($fn -match "_ab|a/b") { $res.Notes += (L "WARN: looks like an A/B image - P10 needs A-only (aonly). Do NOT flash." "WARN: sieht nach A/B-Image aus - P10 braucht A-only. NICHT flashen.") }
  if ($res.Exists -and $res.SizeOk) { $res.Verdict = "PASS" } else { $res.Verdict = "FAIL" }
  return $res
}

function Invoke-SystemFlash {
  param([string]$Image, [switch]$ForceYes)
  $chk = Test-SystemImageFile $Image
  $regChk = Test-RomAgainstRegistry (Get-CompatRegistry) ([System.IO.Path]::GetFileName($Image))
  Write-Host ""
  Write-Host (L "=== System image check (ROM install) ===" "=== System-Image-Pruefung (ROM-Installation) ===") -ForegroundColor Cyan
  Write-Host ((L "Result: " "Ergebnis: ") + $chk.Verdict) -ForegroundColor $(if ($chk.Verdict -eq "PASS") { "Green" } else { "Red" })
  foreach ($n in $chk.Notes) { Write-Host (" - " + $n) -ForegroundColor Gray }
  foreach ($n in $regChk.Reasons) {
    Write-Host (" - REGISTRY: " + $n) -ForegroundColor $(if ($regChk.Status -eq "FAIL") { "Red" } else { "Gray" })
  }
  $profOk = ($DeviceProfiles[$TT.ProfileId].Verified -eq $true)
  if ($chk.Verdict -ne "PASS" -or $regChk.Status -eq "FAIL" -or -not $profOk) {
    Write-Host "DO NOT FLASH" -ForegroundColor Red -BackgroundColor Black
    if (-not $profOk) { Write-TTLog (L "System flash blocked: profile unverified." "System-Flash blockiert: Profil unverifiziert.") "ERROR" }
    if ($regChk.Status -eq "FAIL") { Write-TTLog "System flash blocked: researched-broken ROM." "ERROR" }
    return $false
  }
  Write-Host ""
  Write-Host "WARNING" -ForegroundColor Red
  Write-Host (L "You are about to REPLACE the Android system (fastboot flash system)." "Du ersetzt gleich das Android-System (fastboot flash system).") -ForegroundColor Yellow
  Write-Host (L "Back up internal storage first. userdata is NOT wiped automatically." "Sichere vorher den internen Speicher. userdata wird NICHT automatisch geloescht.") -ForegroundColor Yellow
  Write-Host (L "Afterwards: eRecovery wipe data/factory reset, then first boot (takes a while)." "Danach: eRecovery Wipe data/factory reset, dann Erstboot (dauert).") -ForegroundColor Gray
  if (-not $ForceYes) {
    Write-Host (L "Type 'FLASH' to continue (1/2): " "Zum Fortfahren 'FLASHEN' tippen (1/2): ") -NoNewline -ForegroundColor Yellow
    $a = Read-Host
    if ($a -ne "FLASHEN" -and $a -ne "FLASH") { return $false }
    Write-Host (L "Type 'YES' again (2/2): " "Nochmal 'JA' (2/2): ") -NoNewline -ForegroundColor Yellow
    $b = Read-Host
    if ($b -ne "JA" -and $b -ne "YES") { return $false }
  }
  Update-TTMode | Out-Null
  if ($TT.Mode -ne "fastboot") {
    Write-TTLog (L "Not in fastboot mode, aborting system flash." "Nicht im Fastboot-Modus, System-Flash abgebrochen.") "ERROR"
    return $false
  }
  Write-TTLog "Starting: fastboot flash system <gsi>" "WARNING"
  $o = Invoke-FastbootLogged @("flash","system",$Image)
  $v = Get-FlashVerdict $o
  Show-FlashVerdict $v "flash system"
  $ok = ($v.Verdict -eq "OK")
  if ($ok) {
    Write-Host (L "Next: fastboot reboot -> eRecovery (Vol-Up 3s) -> wipe data/factory reset -> first setup." "Weiter: fastboot reboot -> eRecovery (Vol-Up 3s) -> Wipe/Factory Reset -> Setup.") -ForegroundColor Green
  }
  return [bool]$ok
}

function Get-RomDownloads {
  # Registry ROMs with a verified direct download URL (never guessed links).
  $out = @()
  $reg = Get-CompatRegistry
  if ($reg -eq $null) { return $out }
  foreach ($r in $reg.roms) {
    $url = [string]$r.url
    if ([string]::IsNullOrWhiteSpace($url)) { continue }
    $out += @(@{ Label = (Get-RomEntryLabel $r); File = [string]$r.file; Url = $url; Status = [string]$r.status })
  }
  return $out
}

function Invoke-RomDownload {
  # Numbered working-ROM download -> data/roms -> auto-decompress (.gz) ->
  # validated ready file. Returns ready path or "".
  param([string]$Pick = "", [string]$Label = "", [switch]$ForceYes)
  $list = @(Get-RomDownloads)
  if ($list.Count -eq 0) {
    Write-Host (L "No downloadable ROMs in registry - drop the package into data/roms/ manually." "Keine ladbaren ROMs in Registry - Paket manuell nach data/roms/ legen.") -ForegroundColor Yellow
    return ""
  }
  Write-Host ""
  Write-Host (L "Working downloads (verified links):" "Working-Downloads (gepruefte Links):") -ForegroundColor Cyan
  for ($i = 0; $i -lt $list.Count; $i++) {
    Write-Host (" [" + ($i+1) + "] " + $list[$i].Label + "  (" + $list[$i].File + ", " + $list[$i].Status + ")") -ForegroundColor White
  }
  $sel = $null
  if ($Label -ne "") {
    $sel = @($list | Where-Object { $_.Label -eq $Label } | Select-Object -First 1)
    if ($sel.Count -gt 0) { $sel = $sel[0] } else { $sel = $null }
  }
  if ($sel -eq $null -and $Pick -match "^\d+$") {
    $idx = [int]$Pick - 1
    if ($idx -ge 0 -and $idx -lt $list.Count) { $sel = $list[$idx] }
  }
  if ($sel -eq $null) {
    Write-Host (L "Number + Enter (Enter=abort): " "Nummer + Enter (Enter=Abbruch): ") -NoNewline -ForegroundColor Yellow
    $s = Read-Host
    if ($s -match "^\d+$") {
      $idx = [int]$s - 1
      if ($idx -ge 0 -and $idx -lt $list.Count) { $sel = $list[$idx] }
    }
  }
  if ($sel -eq $null) { return "" }
  if (-not $ForceYes) {
    Write-Host ((L "Download ~1 GB from: " "Download ~1 GB von: ") + $sel.Url) -ForegroundColor Gray
    Write-Host (L "Start download? [Y/n]: " "Download starten? [J/n]: ") -NoNewline -ForegroundColor Cyan
    $a = Read-Host
    if ($a -ne "" -and $a -ne "Y" -and $a -ne "y" -and $a -ne "J" -and $a -ne "j") { return "" }
  }
  if (-not (Test-Path $TTRomDir)) { New-Item -ItemType Directory -Path $TTRomDir -Force | Out-Null }
  $fname = $sel.File
  if ([string]::IsNullOrWhiteSpace($fname)) { $fname = [System.IO.Path]::GetFileName(($sel.Url -split "/download")[0]) }
  $dst = Join-Path $TTRomDir $fname
  $dl = Invoke-FirmwareDownload -Url $sel.Url -OutFile $dst
  if (-not $dl.Ok) { return "" }
  $h = Get-FileHashInfo $dl.Path
  if ($h -ne $null) { $h.SHA256 | Out-File ($dl.Path + ".sha256") -Encoding ascii }
  $ready = $dl.Path
  if ($ready.ToLower().EndsWith(".gz")) {
    Write-Host (L "Decompressing (.gz) ..." "Dekomprimiere (.gz) ...") -ForegroundColor Cyan
    $raw = Expand-TTGzipImage $ready (Split-Path -Parent $ready)
    if ($raw -eq "") { Write-Host (L "Decompress failed." "Dekomprimieren fehlgeschlagen.") -ForegroundColor Red; return "" }
    $ready = $raw
  }
  $kind = Test-ImageKind $ready
  if ($kind -eq "system") {
    Write-Host (L "Ready: SYSTEM image (for Install ROM / flash system)." "Fertig: SYSTEM-Image (fuer ROM-Installation / flash system).") -ForegroundColor Green
  } elseif ($kind -eq "boot") {
    Write-Host (L "Ready: BOOT/RECOVERY image (for Magisk patch base via export)." "Fertig: BOOT/RECOVERY-Image (fuer Magisk-Patch-Basis via Export).") -ForegroundColor Green
  } else {
    Write-Host (L "Downloaded, but content unclear - validate before use." "Geladen, aber Inhalt unklar - vor Nutzung validieren.") -ForegroundColor Yellow
  }
  Write-Host ("Ready: " + $ready) -ForegroundColor White
  return $ready
}

function Get-TargetAndroidVersions {
  # Distinct Android versions with working registry images (+counts). Pure logic on registry.
  $reg = Get-CompatRegistry
  $map = @{}
  if ($reg -eq $null) { return @() }
  foreach ($r in $reg.roms) {
    $st = [string]$r.status
    if ($st -ne "working" -and $st -ne "working-slim" -and $st -ne "working-with-fixes") { continue }
    $a = 0
    try { $a = [int]$r.android } catch {}
    if ($a -le 0) { continue }
    if (-not $map.ContainsKey($a)) { $map[$a] = 0 }
    $map[$a]++
  }
  $out = @()
  foreach ($k in ($map.Keys | Sort-Object)) { $out += @(@{ Android = $k; Count = $map[$k] }) }
  return $out
}

function Get-ResolverSystems {
  # Working systems for one Android version (distinct labels). Pure logic on registry.
  param([int]$Android)
  $reg = Get-CompatRegistry
  $out = @()
  if ($reg -eq $null) { return $out }
  foreach ($r in $reg.roms) {
    $st = [string]$r.status
    if ($st -ne "working" -and $st -ne "working-slim" -and $st -ne "working-with-fixes") { continue }
    $a = 0
    try { $a = [int]$r.android } catch {}
    if ($a -ne $Android) { continue }
    $label = Get-RomEntryLabel $r
    if ($out.Label -notcontains $label) { $out += @(@{ Label = $label; Status = $st }) }
  }
  return $out
}

function Get-ResolverVariants {
  # Registry entries behind one system label (differing variant/build).
  param([string]$Label)
  $reg = Get-CompatRegistry
  $out = @()
  if ($reg -eq $null) { return $out }
  foreach ($r in $reg.roms) {
    $st = [string]$r.status
    if ($st -ne "working" -and $st -ne "working-slim" -and $st -ne "working-with-fixes") { continue }
    if ((Get-RomEntryLabel $r) -eq $Label) { $out += @($r) }
  }
  return $out
}

function Get-TargetConfig {
  # Full resolvable configuration for one registry entry: base + artifacts + root.
  param($Entry)
  $reg = Get-CompatRegistry
  $fwBase = ""
  $fwAdvisory = @()
  if ($reg -ne $null -and $reg.firmware -ne $null) {
    $fwBase = [string]$reg.firmware.required_base
    $fwAdvisory = @($reg.firmware.advisory)
  }
  $ra = @{ type = "recovery_ramdisk"; source = "stock_firmware" }
  if ($Entry -ne $null -and $Entry.root_artifact -ne $null) {
    if ([string]$Entry.root_artifact.type -ne "") { $ra.type = [string]$Entry.root_artifact.type }
    if ([string]$Entry.root_artifact.source -ne "") { $ra.source = [string]$Entry.root_artifact.source }
  }
  $isGsi = ($Entry -ne $null -and -not [string]::IsNullOrWhiteSpace([string]$Entry.gsi))
  return @{
    Device = $TT.ProfileId
    Android = $(if ($Entry -ne $null) { [string]$Entry.android } else { "" })
    System = $(if ($Entry -ne $null) { (Get-RomEntryLabel $Entry) } else { "" })
    IsGsi = $isGsi
    FirmwareBase = $fwBase
    FirmwareAdvisory = $fwAdvisory
    Vendor = $(if ($fwBase -ne "") { "Stock " + $fwBase + " vendor" } else { "" })
    RecoverySource = "RECOVERY_RAMDIS(K).img from UPDATE.APP ($fwBase)"
    RootMethod = "magisk-recovery-ramdisk"
    RootArtifact = $ra
    SystemFile = $(if ($Entry -ne $null) { [string]$Entry.file } else { "" })
    SystemUrl = $(if ($Entry -ne $null) { [string]$Entry.url } else { "" })
  }
}

function Select-TargetImage {
  # Multi-stage resolver: Android -> system -> variant -> full config.
  # Only Android versions with real registry images are listed (with counts).
  Show-TTHeader (L "Target system resolver (device -> Android -> system -> variant -> config)" "Zielsystem-Resolver (Geraet -> Android -> System -> Variante -> Config)")
  Write-Host ""
  Write-Host ((L "Device: Huawei " "Geraet: Huawei ") + $TT.ProfileId) -ForegroundColor Cyan
  $avs = @(Get-TargetAndroidVersions)
  if ($avs.Count -eq 0) {
    Write-Host (L "No working images in registry." "Keine working Images in Registry.") -ForegroundColor Red
    return $null
  }
  Write-Host ""
  Write-Host (L "Which Android version should be installed?" "Welche Android-Version soll installiert werden?") -ForegroundColor White
  for ($i = 0; $i -lt $avs.Count; $i++) {
    Write-Host (" [" + ($i+1) + "] Android " + $avs[$i].Android + "   (" + $avs[$i].Count + (L " images" " Images") + ")") -ForegroundColor White
  }
  Write-Host (L "[Esc] back" "[Esc] zurueck") -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  if ($k.Key -eq "Escape") { return $null }
  $idx = -1
  if ($k.KeyChar -match "[0-9]") { $idx = [int]$k.KeyChar.ToString() - 1 }
  if ($idx -lt 0 -or $idx -ge $avs.Count) { return $null }
  $android = $avs[$idx].Android
  $systems = @(Get-ResolverSystems -Android $android)
  Show-TTHeader ((L "Systems for Android " "Systeme fuer Android ") + $android)
  Write-Host ""
  $si = 0
  if ($systems.Count -eq 1) {
    Write-Host ((L "Only one: " "Nur eins: ") + $systems[0].Label) -ForegroundColor Gray
  } else {
    for ($i = 0; $i -lt $systems.Count; $i++) {
      Write-Host (" [" + ($i+1) + "] " + $systems[$i].Label + "  (" + $systems[$i].Status + ")") -ForegroundColor White
    }
    Write-Host (L "[Esc] back" "[Esc] zurueck") -ForegroundColor DarkGray
    $k2 = [Console]::ReadKey($true)
    if ($k2.Key -eq "Escape") { return $null }
    if ($k2.KeyChar -match "[0-9]") { $si = [int]$k2.KeyChar.ToString() - 1 }
    if ($si -lt 0 -or $si -ge $systems.Count) { return $null }
  }
  $label = $systems[$si].Label
  $variants = @(Get-ResolverVariants -Label $label)
  $entry = $variants[0]
  if ($variants.Count -gt 1) {
    Show-TTHeader ((L "Variant for " "Variante fuer ") + $label)
    Write-Host ""
    for ($i = 0; $i -lt $variants.Count; $i++) {
      $v = $variants[$i]
      $vd = [string]$v.variant; if ($vd -eq "") { $vd = [string]$v.build }
      if ($vd -eq "") { $vd = [string]$v.gsi }
      Write-Host (" [" + ($i+1) + "] " + $vd) -ForegroundColor White
    }
    Write-Host (L "[Esc] back" "[Esc] zurueck") -ForegroundColor DarkGray
    $k3 = [Console]::ReadKey($true)
    if ($k3.Key -eq "Escape") { return $null }
    $vi = -1
    if ($k3.KeyChar -match "[0-9]") { $vi = [int]$k3.KeyChar.ToString() - 1 }
    if ($vi -lt 0 -or $vi -ge $variants.Count) { return $null }
    $entry = $variants[$vi]
  }
  $cfg = Get-TargetConfig $entry
  Show-TTHeader (L "Resolved target configuration" "Aufgeloeste Ziel-Config")
  Write-Host ""
  Write-Host (" Device   : Huawei " + $cfg.Device) -ForegroundColor White
  Write-Host (" Android  : " + $cfg.Android) -ForegroundColor White
  Write-Host (" System   : " + $cfg.System) -ForegroundColor White
  Write-Host (" Base     : " + $cfg.FirmwareBase) -ForegroundColor White
  Write-Host (" Vendor   : " + $cfg.Vendor) -ForegroundColor White
  Write-Host (" Recovery : " + $cfg.RecoverySource) -ForegroundColor White
  Write-Host (" Root     : Magisk / " + $cfg.RootArtifact.type + " (" + $cfg.RootArtifact.source + ")") -ForegroundColor White
  if ($cfg.SystemFile -ne "") { Write-Host (" System   : " + $cfg.SystemFile) -ForegroundColor White }
  if ($cfg.SystemUrl -ne "") { Write-Host (" Download : " + $cfg.SystemUrl) -ForegroundColor Gray }
  else { Write-Host (L " Download : no verified direct link (manual package into data/roms/)." " Download : kein gepruefter Direktlink (Paket manuell nach data/roms/).") -ForegroundColor Yellow }
  return $cfg
}

function Find-LocalSystemImage {
  # Local ready system image for a target config: exact registry filename first,
  # then any *-arm64_*.img under data/. Returns path or "".
  param($Target)
  $want = ""
  if ($Target -ne $null) { $want = [string]$Target.SystemFile }
  $dirs = @($TTRomDir, (Join-Path $TTRoot "data"))
  if ($want -ne "") {
    foreach ($d in $dirs) {
      if (-not (Test-Path $d)) { continue }
      $hit = Get-ChildItem -Path $d -Recurse -File -Filter $want -ErrorAction SilentlyContinue | Select-Object -First 1
      if ($hit -ne $null) { return $hit.FullName }
    }
  }
  foreach ($d in $dirs) {
    if (-not (Test-Path $d)) { continue }
    $hit = Get-ChildItem -Path $d -Recurse -File -Filter "*-arm64_*.img" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($hit -ne $null) { return $hit.FullName }
  }
  return ""
}

function Screen-FlashSystem {
  Show-TTHeader (L "Install ROM / GSI system image (fully guided)" "ROM / GSI System-Image installieren (voll gefuehrt)")
  Write-Host ""
  foreach ($s in $WikiKnowledge.InstallRules) { Write-Host (" - " + $s) -ForegroundColor Gray }
  Write-Host ""
  $prof = $DeviceProfiles[$TT.ProfileId]
  if ($prof -ne $null -and $prof.GsiAdvice -ne "") { Write-Host ((L "Profile advice: " "Profil-Hinweis: ") + $prof.GsiAdvice) -ForegroundColor Cyan }
  $img = $TT.FlashSystemPreset
  $TT.FlashSystemPreset = ""
  if ([string]::IsNullOrWhiteSpace($img)) {
    Write-Host (L "GSI image path (*-arm64_*.img, unpacked - Enter = download working GSI): " "GSI-Image-Pfad (*-arm64_*.img, entpackt - Enter = Working-GSI laden): ") -NoNewline -ForegroundColor Yellow
    $img = Unquote-Path (Read-Host)
    if ([string]::IsNullOrWhiteSpace($img)) {
      $img = Invoke-RomDownload
    }
  } else {
    Write-Host ((L "Preset image from resolver: " "Preset-Image aus Resolver: ") + $img) -ForegroundColor Green
  }
    Write-TTLog (L "Invalid image path, aborting." "Image-Pfad ungueltig, Abbruch.") "ERROR"
    Pause-TT; return
  }
  Invoke-SystemFlash $img | Out-Null
  Pause-TT
}

function Screen-RootMethods {
  Show-TTHeader (L "Root methods (Magisk preferred + compatible alternatives)" "Root-Methoden (Magisk bevorzugt + Alternativen)")
  Write-Host ""
  $ordered = Get-PreferredRootMethod
  for ($i = 0; $i -lt $ordered.Count; $i++) {
    $m = $ordered[$i]
    $mark = " "
    if ($m.Id -eq $TT.RootMethod) { $mark = "*" }
    $pref = ""
    if ($m.Preferred) { $pref = " [PREFERRED]" }
    Write-Host (" [$mark] [" + ($i+1) + "] " + $m.Name + $pref) -ForegroundColor $(if ($m.Preferred) { "Green" } else { "White" })
    Write-Host ("       Needs: " + $m.Needs) -ForegroundColor Gray
    Write-Host ("       Works: " + $m.WorksOn) -ForegroundColor Gray
    Write-Host ("       Note:  " + $m.Notes) -ForegroundColor DarkGray
  }
  Write-Host ""
  Write-Host ((L "Current: " "Aktuell: ") + $TT.RootMethod) -ForegroundColor Cyan
  Write-Host (L "Number to select (Enter=keep): " "Nummer zum Waehlen (Enter=behalten): ") -NoNewline -ForegroundColor Yellow
  $s = Read-Host
  if ($s -match "^[1-9]$") {
    $idx = [int]$s - 1
    if ($idx -ge 0 -and $idx -lt $ordered.Count) {
      $TT.RootMethod = $ordered[$idx].Id
      Write-TTLog ("Root method selected: " + $TT.RootMethod) "SUCCESS"
    }
  }
  Pause-TT
}

function Invoke-TwrpFlash {
  param([string]$Image, [switch]$ForceYes)
  $chk = Test-RecoveryImageFile $Image
  Write-Host ""
  Write-Host (L "=== TWRP image check ===" "=== TWRP-Image-Pruefung ===") -ForegroundColor Cyan
  Write-Host ((L "Result: " "Ergebnis: ") + $chk.Verdict + " | Size=" + $chk.Hash.Size + " | " + $chk.HeaderHex) -ForegroundColor White
  $profOk = ($DeviceProfiles[$TT.ProfileId].Verified -eq $true)
  if ($chk.Verdict -ne "PASS" -or -not $profOk) {
    Write-Host "DO NOT FLASH" -ForegroundColor Red -BackgroundColor Black
    Write-TTLog (L "TWRP flash blocked (image FAIL or profile unverified)." "TWRP-Flash blockiert (Image FAIL oder Profil unverifiziert).") "ERROR"
    return $false
  }
  # Backup current slot first (Magisk slot lives here too).
  if ($TT.StockImage -ne "" -and (Test-Path $TT.StockImage)) { New-TTBackup $TT.StockImage | Out-Null }
  Write-Host ""
  Write-Host "WARNING" -ForegroundColor Red
  Write-Host (L "TWRP and Magisk-recovery SHARE the recovery_ramdisk slot." "TWRP und Magisk-Recovery TEILEN sich den recovery_ramdisk-Slot.") -ForegroundColor Yellow
  Write-Host (L "Flashing TWRP OVERWRITES a Magisk-patched slot (and vice versa)." "TWRP-Flash UEBERSCHREIBT einen Magisk-Slot (und umgekehrt).") -ForegroundColor Yellow
  Write-Host ((L "Backup: " "Backup: ") + $TT.BackupDir) -ForegroundColor White
  Write-Host (L "Boot TWRP afterwards: hold Vol-Up until loaded. NEVER wipe userdata in TWRP." "Danach TWRP booten: Vol-Up halten bis geladen. NIEMALS userdata in TWRP wipen.") -ForegroundColor Gray
  if (-not $ForceYes) {
    Write-Host (L "Type 'FLASH' to continue (1/2): " "Zum Fortfahren 'FLASHEN' tippen (1/2): ") -NoNewline -ForegroundColor Yellow
    $a = Read-Host
    if ($a -ne "FLASHEN" -and $a -ne "FLASH") { return $false }
    Write-Host (L "Type 'YES' again (2/2): " "Nochmal 'JA' (2/2): ") -NoNewline -ForegroundColor Yellow
    $b = Read-Host
    if ($b -ne "JA" -and $b -ne "YES") { return $false }
  }
  Update-TTMode | Out-Null
  if ($TT.Mode -ne "fastboot") {
    Write-TTLog (L "Not in fastboot mode, aborting TWRP flash." "Nicht im Fastboot-Modus, TWRP-Flash abgebrochen.") "ERROR"
    return $false
  }
  $part = $DeviceProfiles[$TT.ProfileId].TargetPartition
  Write-TTLog "Starting: fastboot flash $part <twrp>" "WARNING"
  $o = Invoke-FastbootLogged @("flash",$part,$Image)
  $v = Get-FlashVerdict $o
  Show-FlashVerdict $v ("flash " + $part)
  $ok = ($v.Verdict -eq "OK")
  if ($ok) { Write-Host (L "TWRP flash OK. Boot: hold Vol-Up." "TWRP-Flash OK. Boot: Vol-Up halten.") -ForegroundColor Green }
  return [bool]$ok
}

function Screen-Twrp {
  Show-TTHeader (L "TWRP path (guide + guided flash)" "TWRP-Pfad (Anleitung + gefuehrter Flash)")
  Write-Host ""
  Write-Host (L "Sources (device-exact builds only):" "Quellen (nur geraetegenaue Builds):") -ForegroundColor Cyan
  foreach ($s in $TwrpKnowledge.Sources) { Write-Host (" - " + $s) -ForegroundColor White }
  Write-Host ""
  Write-Host (L "Rules:" "Regeln:") -ForegroundColor Cyan
  foreach ($s in $TwrpKnowledge.Rules) { Write-Host (" - " + $s) -ForegroundColor Yellow }
  Write-Host ""
  Write-Host (L "TWRP image path (*twrp*.img, exact model build): " "TWRP-Image-Pfad (*twrp*.img, genauer Modell-Build): ") -NoNewline -ForegroundColor Yellow
  $img = Unquote-Path (Read-Host)
  if ([string]::IsNullOrWhiteSpace($img) -or -not (Test-Path $img)) {
    Write-TTLog (L "Invalid image path, aborting (guide shown above)." "Image-Pfad ungueltig, Abbruch (Anleitung oben).") "WARNING"
    Pause-TT; return
  }
  Invoke-TwrpFlash $img | Out-Null
  Pause-TT
}

function Screen-Compatibility {
  Show-TTHeader (L "Compatibility registry (researched ROM/firmware matrix)" "Kompatibilitaets-Registry (recherchierte ROM/Firmware-Matrix)")
  $reg = Get-CompatRegistry
  if ($reg -eq $null) {
    Write-Host (L "No registry for this profile. Submit device data first (device-support template)." "Keine Registry fuer dieses Profil. Erst Geraetedaten einreichen (device-support-Template).") -ForegroundColor Yellow
    Pause-TT; return
  }
  Write-Host ""
  Write-Host (L "Recommended (tested builds only):" "Empfohlen (nur getestete Builds):") -ForegroundColor Green
  foreach ($r in $reg.roms) {
    if ($r.status -like "working*") {
      $extra = ""
      if ($r.version) { $extra += " " + $r.version }
      if ($r.android) { $extra += " A" + $r.android }
      if ($r.build) { $extra += " build " + $r.build }
      Write-Host (" [+] " + $r.name + $extra + " [" + $r.status + "]") -ForegroundColor White
    }
  }
  Write-Host ""
  Write-Host (L "NOT recommended (researched):" "NICHT empfohlen (recherchiert):") -ForegroundColor Red
  foreach ($r in $reg.roms) {
    if ($r.status -notlike "working*") {
      Write-Host (" [X] " + $r.name + " [" + $r.status + "] - " + $r.reason) -ForegroundColor Yellow
    }
  }
  Write-Host ""
  if ($reg.firmware -ne $null) {
    Write-Host (L "Firmware base: " "Firmware-Basis: ") -NoNewline -ForegroundColor Cyan
    Write-Host ($reg.firmware.required_base + " | " + (($reg.firmware.advisory -join "; "))) -ForegroundColor White
  }
  if ($reg.magisk -ne $null) {
    Write-Host (L "Magisk: " "Magisk: ") -NoNewline -ForegroundColor Cyan
    Write-Host ($reg.magisk.method + ", on-device patch, forbidden: " + (($reg.magisk.forbidden -join ", "))) -ForegroundColor White
  }
  Pause-TT
}

function Invoke-DeveloperDump {
  # Developer mode only: raw device dumps for ROM research. Read-only.
  param([string]$Kind)
  if ($TTMode -ne "developer") {
    Write-Host (L "Developer mode required: re-run with -Mode developer." "Developer-Modus noetig: neu starten mit -Mode developer.") -ForegroundColor Yellow
    return $false
  }
  if ($TT.Mode -ne "android") { Update-TTMode | Out-Null }
  if ($TT.Mode -ne "android") { Write-TTLog (L "No android device for dump." "Kein Android-Geraet fuer Dump.") "ERROR"; return $false }
  $map = @{
    "dump-partitions" = @("shell","cat","/proc/partitions");
    "dump-properties" = @("shell","getprop");
    "dump-vendor"     = @("shell","ls -l /vendor/etc/ 2>&1; cat /vendor/build.prop 2>&1");
    "dump-logs"       = @("shell","logcat -d -t 200 2>&1")
  }
  if (-not $map.ContainsKey($Kind)) { return $false }
  $o = (Invoke-TTAdb $map[$Kind] -join "`n")
  $f = Join-Path $TTLogDir ($Kind + "-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".txt")
  $o | Out-File $f -Encoding utf8
  Write-TTLog ("Dump: " + $f) "SUCCESS"
  return $true
}

function Invoke-GuidedWipe {
  # Guided userdata wipe. Double-confirmed, never automatic. Erases user data
  # only (no system/boot touch -> no brick vector). eRecovery fallback if refused.
  param([switch]$ForceYes)
  Update-TTMode | Out-Null
  if ($TT.Mode -ne "fastboot") {
    Write-TTLog (L "Wipe needs fastboot mode. Reboot to fastboot first." "Wipe braucht Fastboot-Modus. Erst nach Fastboot booten.") "ERROR"
    return $false
  }
  Write-Host ""
  Write-Host "WARNING" -ForegroundColor Red
  Write-Host (L "This erases ALL user data (apps, photos, settings). System stays." "Das loescht ALLE Nutzerdaten (Apps, Fotos, Einstellungen). System bleibt.") -ForegroundColor Yellow
  if (-not $ForceYes) {
    Write-Host (L "Type 'WIPE' to continue (1/2): " "'WIPE' tippen zum Fortfahren (1/2): ") -NoNewline -ForegroundColor Yellow
    $a = Read-Host
    if ($a -ne "WIPE") { return $false }
    Write-Host (L "Type 'YES' again (2/2): " "Nochmal 'YES' (2/2): ") -NoNewline -ForegroundColor Yellow
    $b = Read-Host
    if ($b -ne "YES" -and $b -ne "JA") { return $false }
  } else {
    Write-TTLog "CLI --yes: explicit wipe consent documented." "WARNING"
  }
  $o = Invoke-FastbootLogged @("erase","userdata")
  $v = Get-FlashVerdict $o
  Show-FlashVerdict $v "erase userdata"
  if ($v.Verdict -eq "OK") {
    return $true
  }
  Write-Host (L "Device refused erase. Fallback (manual, safe): reboot to stock eRecovery (Vol-Up 3s) -> Wipe data / factory reset -> confirm on phone." "Geraet lehnt erase ab. Fallback (manuell, sicher): Stock-eRecovery booten (Vol-Up 3s) -> Wipe data / factory reset -> am Handy bestaetigen.") -ForegroundColor Yellow
  Write-TTLog "erase refused, eRecovery fallback shown." "WARNING"
  return $false
}

function Screen-Wipe {
  Show-TTHeader (L "Wipe userdata (guided, double-confirmed)" "Userdata wipen (gefuehrt, doppelt bestaetigt)")
  if (Invoke-GuidedWipe) {
    Write-Host (L "Wipe done. Reboot and set up the phone again." "Wipe fertig. Handy neu starten und neu einrichten.") -ForegroundColor Green
  }
  Pause-TT
}

function Screen-Reinstall {
  Show-TTHeader (L "Full reinstall (guided: choose ROM, optional wipe, flash, verify)" "Komplett-Reinstall (gefuehrt: ROM waehlen, optional Wipe, Flash, Verify)")
  Write-Host ""
  Write-Host (L "Already complete today: flash-system/TWRP/restore exist. This flow chains them: ROM choice -> backup reminder -> optional wipe -> flash -> reboot -> verify." "Teile existieren (flash-system/TWRP/restore). Dieser Flow verkettet: ROM-Wahl -> Backup-Hinweis -> optional Wipe -> Flash -> Reboot -> Verify.") -ForegroundColor Gray
  Write-Host ""
  Write-Host (L "[1] Custom ROM / GSI image  [2] Stock full firmware path  [Esc] back" "[1] Custom-ROM / GSI-Image  [2] Stock-Full-Firmware-Weg  [Esc] zurueck") -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  if ($k.KeyChar -eq "1") {
    Write-Host (L "Wipe userdata before flash? [W]=wipe first (double-confirmed) [Enter]=skip: " "Userdata vorher wipen? [W]=erst wipen (doppelt bestaetigt) [Enter]=ueberspringen: ") -NoNewline -ForegroundColor Yellow
    $w = [Console]::ReadKey($true)
    if ($w.KeyChar -eq "W" -or $w.KeyChar -eq "w") { Invoke-GuidedWipe | Out-Null }
    Screen-FlashSystem
  } elseif ($k.KeyChar -eq "2") {
    Show-TTHeader (L "Stock reinstall path (guided, honest scope)" "Stock-Reinstall-Weg (gefuehrt, ehrlicher Scope)")
    Write-Host ""
    Write-Host (L "This tool cannot unpack UPDATE.APP system images itself. Stock return runs via:" "Dieses Tool kann UPDATE.APP-System-Images nicht selbst entpacken. Stock-Rueckweg via:") -ForegroundColor White
    Write-Host (L "1. Full firmware for exact model+region (step 3 downloader or data/firmware/)." "1. Full-Firmware exakt fuer Modell+Region (Step-3-Downloader oder data/firmware/).") -ForegroundColor White
    Write-Host (L "2. Flash via HiSuite (official, recommended) or service flow." "2. Flashen via HiSuite (offiziell, empfohlen) oder Service-Flow.") -ForegroundColor White
    Write-Host (L "3. Stock eRecovery wipe + first setup (double-confirmed on phone)." "3. Stock-eRecovery-Wipe + Setup (doppelt am Handy bestaetigt).") -ForegroundColor White
    Write-Host ""
    Write-Host (L "If you already extracted SYSTEM.img from UPDATE.APP, use Install ROM with that file." "Falls SYSTEM.img schon extrahiert: per ROM-Installation mit dieser Datei flashen.") -ForegroundColor Cyan
    Pause-TT
  }
}

function Screen-Tools {
  while ($true) {
    $c = Show-TTMenu (L "Tools (read-only where possible)" "Tools (read-only wo moeglich)") @(
      "adb devices -l",
      (L "Reboot menu (bootloader/recovery/fastbootd/system)" "Reboot-Menue (bootloader/recovery/fastbootd/system)"),
      "fastboot devices + getvar (tolerant, Huawei FAILED ok)",
      "getprop full dump -> logs/",
      "adb kill-server/start-server",
      (L "Create diagnostic ZIP" "Diagnose-ZIP erzeugen"),
      (L "Mirror screen via scrcpy (optional)" "Bildschirm via scrcpy spiegeln (optional)"),
      (L "Post-flash validation report" "Post-Flash-Validierungsbericht"),
      (L "Back" "Zurueck")
    )
    if ($c -eq -1 -or $c -eq 8) { return }
    if ($c -eq 0) { Show-TTHeader "adb devices"; Write-Host ""; Write-Host ((Invoke-TTAdb @("devices","-l") -join "`n") ) -ForegroundColor White; Pause-TT }
    elseif ($c -eq 1) {
      $s = Show-TTMenu (L "Reboot target" "Reboot-Ziel") @("bootloader","recovery","fastbootd","system",(L "Cancel" "Abbrechen"))
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
      Write-Host (L "'waiting for device'/FAILED = driver/mode/Huawei behavior, not auto=locked." "'waiting for device'/FAILED = Treiber/Modus/Huawei-Eigenheit, nicht auto=locked.") -ForegroundColor Yellow
      Pause-TT
    }
    elseif ($c -eq 3) {
      $o = (Invoke-TTAdb @("shell","getprop") -join "`n")
      $f = Join-Path $TTLogDir ("getprop-full-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".txt")
      $o | Out-File $f -Encoding utf8
      Write-TTLog "Dump: $f" "SUCCESS"; Pause-TT
    }
    elseif ($c -eq 4) { Invoke-TTAdb @("kill-server") | Out-Null; Invoke-TTAdb @("start-server") | Out-Null; Write-TTLog (L "ADB server reset." "ADB-Server reset.") "SUCCESS"; Pause-TT }
    elseif ($c -eq 5) {
      $z = New-TTDiagnostic
      Write-Host ("ZIP: " + $z) -ForegroundColor White
      Pause-TT
    }
    elseif ($c -eq 6) {
      if ($TT.Scrcpy) {
        Write-TTLog (L "Starting scrcpy mirror (close window to continue) ..." "Starte scrcpy-Spiegel (Fenster schliessen zum Fortfahren) ...") "INFO"
        try { Start-Process -FilePath $TT.Scrcpy | Out-Null } catch { Write-TTLog $_.Exception.Message "ERROR" }
      } else {
        Write-Host (L "scrcpy not installed (optional). Get it: https://github.com/Genymobile/scrcpy" "scrcpy nicht installiert (optional). Bezug: https://github.com/Genymobile/scrcpy") -ForegroundColor Yellow
        Pause-TT
      }
    }
    elseif ($c -eq 7) {
      $rep = Invoke-TTValidate
      Write-ValidationReport $rep | Out-Null
      Pause-TT
    }
  }
}

function Screen-Bootkeys {
  Show-TTHeader (L "Huawei boot mechanism (from #2542, exact)" "Huawei Boot-Mechanismus (aus #2542, exakt)")
  Write-Host ""
  Write-Host (L "- Magisk boot: Vol-Up + Power until Huawei logo, then release (Magisk boot cheat)." "- Magisk-Boot: Vol-Up + Power bis Huawei-Logo, dann loslassen (Magisk boot cheat).") -ForegroundColor White
  Write-Host (L "- Without trick: stock boot (no root). Behavior NOT persistent." "- Ohne Trick: Stock-Boot (kein Root). Verhalten NICHT persistent.") -ForegroundColor White
  Write-Host (L "- Set persistent byte: warning screen -> eRecovery (Vol-Up 3s) -> confirm Wipe/Factory Reset+reboot" "- Persistent-Byte setzen: Warnscreen -> eRecovery (Vol-Up 3s) -> Wipe/Factory Reset bestaetigen+reboot") -ForegroundColor White
  Write-Host (L "  -> boots Magisk root (wipe is NOT executed)." "  -> bootet Magisk-Root (Wipe wird dabei NICHT ausgefuehrt).") -ForegroundColor White
  Write-Host (L "- Clear byte: remove /dload! Power off -> Vol-Up+Vol-Down+Power until logo" "- Byte loeschen: /dload entfernen! Power off -> Vol-Up+Vol-Down+Power bis Logo") -ForegroundColor White
  Write-Host (L "  -> EMUI 'OS Upgrade not successful' -> reboot -> clean." "  -> EMUI 'OS Upgrade not successful' -> Reboot -> clean.") -ForegroundColor White
  Write-Host (L "- /dload must NOT be on storage, else EMUI updater instead of recovery." "- /dload darf NICHT auf Speicher liegen, sonst EMUI-Updater statt Recovery.") -ForegroundColor Red
  Write-Host (L "- Never use Pixel/A-B guides." "- Keine Pixel-/A-B-Anleitung verwenden.") -ForegroundColor Yellow
  $bm = ""
  try {
    $st0 = Read-WorkflowState
    if ($st0 -ne $null -and $st0.last_root -ne $null -and $st0.last_root.boot_mode -ne $null) { $bm = [string]$st0.last_root.boot_mode }
  } catch {}
  if ($bm -ne "") { Write-Host ((L "Persisted boot mode: " "Persistierter Boot-Modus: ") + $bm) -ForegroundColor Cyan }
  Write-Host ""
  Write-Host (L "[1] Guide: SET persistent Magisk boot (discussion step 13)  [2] Guide: CLEAR it again (step 14)  [3] Verify persistence (normal reboot, then check)" "[1] Anleitung: persistenten Magisk-Boot SETZEN (Discussion-Schritt 13)  [2] Anleitung: wieder LOESCHEN (Schritt 14)  [3] Persistenz pruefen (normal rebooten, dann checken)") -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  if ($k.KeyChar -eq "1") {
    Show-TTHeader (L "SET persistent boot (manual on-device steps)" "Persistenten Boot SETZEN (manuelle Handy-Schritte)")
    Write-Host ""
    foreach ($l in @(
      (L "1. Boot the phone normally into the Magisk system once (Vol-Up + Power)." "1. Handy einmal normal ins Magisk-System booten (Vol-Up + Power)."),
      (L "2. At the yellow-text-on-black screen choose eRecovery (hold Vol-Up ~3s)." "2. Am Gelb-auf-Schwarz-Screen eRecovery waehlen (Vol-Up ~3s halten)."),
      (L "3. In eRecovery confirm Wipe data / Factory reset + reboot." "3. In eRecovery Wipe data / Factory reset bestaetigen + Reboot."),
      (L "4. The phone now boots the Magisk system on EVERY power-on (wipe is NOT executed)." "4. Handy bootet jetzt bei JEDEM Einschalten ins Magisk-System (Wipe wird NICHT ausgefuehrt).")
    )) { Write-Host (" " + $l) -ForegroundColor White }
    Write-Host ""
    Write-Host (L "Double confirmation (this changes every boot - reversible via CLEAR): type 'SET' then 'YES': " "Doppel-Bestaetigung (aendert jeden Boot - reversibel via CLEAR): 'SET' dann 'YES' tippen: ") -NoNewline -ForegroundColor Yellow
    $a = Read-Host
    if ($a -ne "SET") { Pause-TT; return }
    Write-Host (L "Type 'YES': " "'YES' tippen: ") -NoNewline -ForegroundColor Yellow
    $b = Read-Host
    if ($b -ne "YES" -and $b -ne "JA") { Pause-TT; return }
    Write-TTLog "User confirmed persistent-boot SET guide (manual steps shown)." "WARNING"
    Save-RootState "ROOTED" "persistent-pending"
    Write-Host (L "Do the 4 steps on the phone now, then use [3] to verify." "Jetzt die 4 Schritte am Handy ausfuehren, dann mit [3] verifizieren.") -ForegroundColor Green
  }
  elseif ($k.KeyChar -eq "2") {
    Show-TTHeader (L "CLEAR persistent boot (manual on-device steps)" "Persistenten Boot LOESCHEN (manuelle Handy-Schritte)")
    Write-Host ""
    foreach ($l in @(
      (L "1. Remove any /dload directory from phone storage (else EMUI updater starts)!" "1. /dload-Verzeichnis vom Speicher entfernen (sonst EMUI-Updater)!"),
      (L "2. Power off completely." "2. Komplett ausschalten."),
      (L "3. Hold Vol-Up + Vol-Down + Power until the Huawei logo, then release." "3. Vol-Up + Vol-Down + Power bis Huawei-Logo halten, dann loslassen."),
      (L "4. EMUI shows 'OS Upgrade not successful' -> reboot -> byte is clean, normal boots are unrooted again." "4. EMUI zeigt 'OS Upgrade not successful' -> Reboot -> Byte clean, normale Boots wieder ungerootet.")
    )) { Write-Host (" " + $l) -ForegroundColor White }
    Write-Host ""
    Write-Host (L "Confirm you did it on the phone [Y/n]: " "Am Handy erledigt bestaetigen [J/n]: ") -NoNewline -ForegroundColor Yellow
    $c = Read-Host
    if ($c -eq "" -or $c -eq "Y" -or $c -eq "y" -or $c -eq "J" -or $c -eq "j") {
      Save-RootState "UNKNOWN" "cheat"
      Write-TTLog "Persistent boot cleared by user." "SUCCESS"
    }
  }
  elseif ($k.KeyChar -eq "3") {
    Write-Host ""
    Write-Host (L "Reboot the phone NORMALLY now (no keys), wait for Android, then press Enter here." "Handy jetzt NORMAL rebooten (keine Tasten), Android abwarten, dann hier Enter.") -ForegroundColor Cyan
    Pause-TT
    Update-TTMode | Out-Null
    if ($TT.Mode -ne "android") {
      Write-Host (L "No Android/ADB - cannot verify yet." "Kein Android/ADB - noch nicht verifizierbar.") -ForegroundColor Yellow
      Pause-TT; return
    }
    $idOut = (Invoke-TTAdb @("shell","su -c id 2>&1") -join "").Trim()
    if ($idOut -match "uid=0") {
      Save-RootState "ROOTED" "persistent"
      Write-Host (L "PERSISTENT ROOT CONFIRMED (uid=0 without boot cheat)." "PERSISTENTER ROOT BESTAETIGT (uid=0 ohne Boot-Cheat).") -ForegroundColor Green
    } else {
      Save-RootState "NOT_ROOTED" "cheat"
      Write-Host (L "Not persistent: normal boot is unrooted (cheat still needed, or SET not active)." "Nicht persistent: normaler Boot ungerootet (Cheat weiter noetig oder SET nicht aktiv).") -ForegroundColor Yellow
    }
  }
  Pause-TT
}

function Screen-Logs {
  Show-TTHeader (L "Logs (logs/) + diagnostics" "Logs (logs/) + Diagnose")
  $files = Get-ChildItem $TTLogDir -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 15
  if (-not $files) { Write-Host (L "No logs." "Keine Logs.") -ForegroundColor Yellow; Pause-TT; return }
  $names = @()
  foreach ($f in $files) { $names += ($f.Name) }
  $names += (L "Open log folder" "Log-Ordner oeffnen"); $names += (L "Create diagnostic ZIP" "Diagnose-ZIP erzeugen"); $names += (L "Back" "Zurueck")
  $c = Show-TTMenu (L "Choose log" "Log waehlen") $names
  if ($c -eq -1 -or $c -eq ($names.Count-1)) { return }
  if ($c -eq ($names.Count-3)) { try { Start-Process explorer.exe $TTLogDir | Out-Null } catch {} ; return }
  if ($c -eq ($names.Count-2)) { $z = New-TTDiagnostic; Write-Host $z -ForegroundColor White; Pause-TT; return }
  $file = $files[$c].FullName
  Show-TTHeader ("Log: " + $files[$c].Name)
  Get-Content $file -TotalCount 200 | ForEach-Object { Write-Host $_ -ForegroundColor Gray }
  Pause-TT
}

function Select-RootTarget {
  # "Nur Root" needs an explicit target system too: keep current, stock, or other.
  # Returns $true to continue, $false to abort. Sets InstalledRom when chosen.
  Show-TTHeader (L "Root target: which system stays on the phone?" "Root-Ziel: welches System bleibt auf dem Handy?")
  Write-Host ""
  $curLabel = Get-RomLabel $TT.InstalledRom
  if ([string]::IsNullOrWhiteSpace($TT.InstalledRom)) { $curLabel = (L "(unknown - analyze first)" "(unbekannt - erst analysieren)") }
  Write-Host ("[1] " + (L "Current system: " "Aktuelles System: ") + $curLabel) -ForegroundColor White
  Write-Host ("[2] " + (L "Stock EMUI (choose version)" "Stock-EMUI (Version waehlen)")) -ForegroundColor White
  Write-Host ("[3] " + (L "Other system / ROM (choose Android -> ROM -> variant)" "Anderes System / ROM (Android -> ROM -> Variante waehlen)")) -ForegroundColor White
  Write-Host (L "[Esc] back" "[Esc] zurueck") -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  if ($k.Key -eq "Escape") { return $false }
  if ($k.KeyChar -eq "1") {
    if ([string]::IsNullOrWhiteSpace($TT.InstalledRom)) { Select-InstalledRom | Out-Null }
    if ([string]::IsNullOrWhiteSpace($TT.InstalledRom)) { return $false }
    $pb = Get-PatchBase
    Write-Host ""
    Write-Host ((L "Target kept: " "Ziel bleibt: ") + (Get-RomLabel $TT.InstalledRom) + " -> root artifact: " + $pb.Source) -ForegroundColor Green
    return $true
  }
  if ($k.KeyChar -eq "2") {
    Show-TTHeader (L "Which Stock EMUI version?" "Welche Stock-EMUI-Version?")
    Write-Host ""
    Write-Host "[1] Android 8 / EMUI 8" -ForegroundColor White
    Write-Host "[2] Android 9 / EMUI 9.0" -ForegroundColor White
    Write-Host "[3] Android 9 / EMUI 9.1" -ForegroundColor White
    Write-Host (L "[Esc] back" "[Esc] zurueck") -ForegroundColor DarkGray
    $k2 = [Console]::ReadKey($true)
    if ($k2.Key -eq "Escape") { return $false }
    $emui = ""
    if ($k2.KeyChar -eq "1") { $emui = "EMUI 8 (Android 8)" }
    elseif ($k2.KeyChar -eq "2") { $emui = "EMUI 9.0 (Android 9)" }
    elseif ($k2.KeyChar -eq "3") { $emui = "EMUI 9.1 (Android 9)" }
    else { return $false }
    Save-InstalledRom "stock"
    Write-Host ""
    Write-Host ((L "Target: Stock " "Ziel: Stock ") + $emui) -ForegroundColor Green
    $reg = Get-CompatRegistry
    if ($reg -ne $null -and $reg.firmware -ne $null) {
      Write-Host ((L "Required base: " "Benoetigte Basis: ") + [string]$reg.firmware.required_base) -ForegroundColor White
      foreach ($a in @($reg.firmware.advisory)) { Write-Host (" - " + $a) -ForegroundColor Gray }
      Write-Host (L "Firmware portals are gated (login/pack): place the full UPDATE.APP/ZIP into data/firmware/ or use the firmware downloader." "Firmware-Portale sind gated (Login/Paket): Full-UPDATE.APP/ZIP nach data/firmware/ legen oder Firmware-Downloader nutzen.") -ForegroundColor Yellow
    }
    return $true
  }
  if ($k.KeyChar -eq "3") {
    $tcfg = Select-TargetImage
    if ($tcfg -eq $null) { return $false }
    $id = "rom:" + $tcfg.System
    Save-InstalledRom $id
    Write-Host ""
    Write-Host ((L "Target kept for root (system is NOT replaced): " "Ziel fuer Root (System wird NICHT ersetzt): ") + $tcfg.System + " / Android " + $tcfg.Android) -ForegroundColor Green
    Write-Host ((L "Root artifact: " "Root-Artefakt: ") + $tcfg.RootArtifact.type + " (" + $tcfg.RootArtifact.source + ")") -ForegroundColor White
    return $true
  }
  return $false
}

function Start-TTWizard {
  # ROM-aware guided path: Detect -> Analyze (+ROM question) -> plain goal words
  # -> plan with visible SKIPs -> run. No fluff, no inapplicable steps.
  Load-InstalledRom | Out-Null
  Screen-Detect
  Screen-Analyze
  if ([string]::IsNullOrWhiteSpace($TT.InstalledRom)) { Select-InstalledRom | Out-Null }
  $romLabel = Get-RomLabel $TT.InstalledRom
  $pbSrc = (Get-PatchBase).Source
  $needsRomExport = ($pbSrc -eq "rom")
  Show-TTHeader (L "What do you want to do?" "Was willst du tun?")
  Write-Host ""
  Write-Host ((L "Your system: " "Dein System: ") + $romLabel) -ForegroundColor Cyan
  Write-Host ""
  Write-Host (L "[1] Root only (keep my system exactly as it is)" "[1] Nur Root (mein System bleibt exakt wie es ist)") -ForegroundColor White
  Write-Host (L "[2] Install a custom ROM / GSI" "[2] Custom-ROM / GSI installieren") -ForegroundColor White
  Write-Host (L "[3] Back to stock" "[3] Zurueck zu Stock") -ForegroundColor White
  Write-Host (L "[Esc] back" "[Esc] zurueck") -ForegroundColor DarkGray
  $k = [Console]::ReadKey($true)
  $goal = ""; $steps = @(); $skipped = @(); $autoFix = $false
  if ($k.KeyChar -eq "1") {
    $goal = (L "Root only" "Nur Root")
    $autoFix = $true
    if (-not (Select-RootTarget)) { return }
    $romLabel = Get-RomLabel $TT.InstalledRom
    $pbSrc = (Get-PatchBase).Source
    $needsRomExport = ($pbSrc -eq "rom")
    if ($needsRomExport) {
      $steps = @(
        @{ Screen = "Screen-ExportRecovery"; Label = (L "Get patch base from YOUR rom package" "Patch-Basis aus DEINEM ROM-Paket holen") },
        @{ Screen = "Screen-Patch"; Label = (L "Magisk patch (on your phone)" "Magisk-Patch (an deinem Handy)") },
        @{ Screen = "Screen-Backup"; Label = (L "Backup" "Backup") },
        @{ Screen = "Screen-Flash"; Label = (L "Flash + safety gate" "Flash + Safety-Gate") },
        @{ Screen = "Screen-RebootVerify"; Label = (L "Reboot + verify root" "Reboot + Root verify") }
      )
      $skipped = @(
        (L "stock firmware search/download - not needed, your base comes from " "Stock-Firmware-Suche/Download - nicht noetig, deine Basis kommt aus ") + $romLabel,
        (L "stock UPDATE.APP extract - not needed" "Stock-UPDATE.APP-Extrakt - nicht noetig")
      )
    } else {
      $steps = @(
        @{ Screen = "Screen-Firmware"; Label = (L "Stock firmware (find + download)" "Stock-Firmware (finden + laden)") },
        @{ Screen = "Screen-Extract"; Label = (L "Extract stock recovery image" "Stock-Recovery-Image extrahieren") },
        @{ Screen = "Screen-Patch"; Label = (L "Magisk patch (on your phone)" "Magisk-Patch (an deinem Handy)") },
        @{ Screen = "Screen-Backup"; Label = (L "Backup" "Backup") },
        @{ Screen = "Screen-Flash"; Label = (L "Flash + safety gate" "Flash + Safety-Gate") },
        @{ Screen = "Screen-RebootVerify"; Label = (L "Reboot + verify root" "Reboot + Root verify") }
      )
    }
  } elseif ($k.KeyChar -eq "2") {
    $goal = (L "Install custom ROM" "Custom-ROM installieren")
    $tcfg = Select-TargetImage
    if ($tcfg -eq $null) { return }
    $img = Find-LocalSystemImage $tcfg
    if ($img -ne "") {
      Write-Host ((L "Local image found: " "Lokales Image gefunden: ") + $img) -ForegroundColor Green
    } elseif ($tcfg.SystemUrl -ne "") {
      $img = Invoke-RomDownload -Label $tcfg.System
    }
    if ($img -eq "" -or -not (Test-Path $img)) {
      Write-Host (L "No image ready - manual install screen next." "Kein Image bereit - weiter mit manuellem Install-Screen.") -ForegroundColor Yellow
      $steps = @(
        @{ Screen = "Screen-Compatibility"; Label = (L "Check ROM compatibility" "ROM-Kompatibilitaet pruefen") },
        @{ Screen = "Screen-FlashSystem"; Label = (L "Install ROM image (manual path or download)" "ROM-Image installieren (manueller Pfad oder Download)") },
        @{ Screen = "Screen-RebootVerify"; Label = (L "Reboot + verify" "Reboot + Verify") }
      )
      $skipped = @((L "root/patch/flash - run this wizard again with [1] afterwards if you want root" "Root/Patch/Flash - danach Wizard erneut mit [1] starten falls Root gewuenscht"))
    } else {
      $TT.FlashSystemPreset = $img
      $steps = @(
        @{ Screen = "Screen-Compatibility"; Label = (L "Check ROM compatibility" "ROM-Kompatibilitaet pruefen") },
        @{ Screen = "Screen-FlashSystem"; Label = (L "Install ROM image (ready, no path needed)" "ROM-Image installieren (bereit, kein Pfad noetig)") },
        @{ Screen = "Screen-RebootVerify"; Label = (L "Reboot + verify" "Reboot + Verify") }
      )
      $skipped = @((L "root/patch/flash - run this wizard again with [1] afterwards if you want root" "Root/Patch/Flash - danach Wizard erneut mit [1] starten falls Root gewuenscht"))
    }
  } elseif ($k.KeyChar -eq "3") {
    $goal = (L "Back to stock" "Zurueck zu Stock")
    $steps = @(
      @{ Screen = "Screen-Firmware"; Label = (L "Stock firmware (find + download)" "Stock-Firmware (finden + laden)") },
      @{ Screen = "Screen-Reinstall"; Label = (L "Stock return guide" "Stock-Rueckweg-Anleitung") }
    )
    $skipped = @((L "Magisk patch/flash - stock return needs no root steps" "Magisk-Patch/Flash - Stock-Rueckweg braucht keine Root-Steps"))
  } else { return }
  Show-TTHeader ((L "Your path: " "Dein Weg: ") + $romLabel + " -> " + $goal)
  Write-Host ""
  if ($pbSrc -eq "stock-gsi") {
    Write-Host (L "GSI detected: recovery is untouched stock, so the normal stock steps below are correct." "GSI erkannt: Recovery ist unberuehrt Stock, also sind die Stock-Steps unten korrekt.") -ForegroundColor Green
    Write-Host ""
  }
  $n = 1
  foreach ($s in $steps) { Write-Host (" [" + $n + "] " + $s.Label) -ForegroundColor White; $n++ }
  foreach ($s in $skipped) { Write-Host (" [SKIP] " + $s) -ForegroundColor DarkGray }
  Write-Host ""
  Write-Host (L "Run now? [Y/n]: " "Jetzt starten? [J/n]: ") -NoNewline -ForegroundColor Cyan
  $a = Read-Host
  if ($a -ne "" -and $a -ne "Y" -and $a -ne "y" -and $a -ne "J" -and $a -ne "j") { return }
  $TT.SkipFixOffer = $autoFix
  foreach ($s in $steps) {
    try { & $s.Screen | Out-Null } catch { Write-TTLog (("Wizard step failed: " + $s.Label + " - " + $_.Exception.Message)) "ERROR"; $TT.SkipFixOffer = $false; return }
  }
  $TT.SkipFixOffer = $false
  if ($autoFix) {
    Write-Host ""
    Write-Host (L "Automatic: permanent APTouch fix attempt (non-destructive service.d + immediate stop). Skips cleanly without live root." "Automatisch: permanenter APTouch-Fix-Versuch (zerstoerungsfreies service.d + Sofort-Stopp). Ohne live Root sauber uebersprungen.") -ForegroundColor Cyan
    Update-TTMode | Out-Null
    if (Install-PersistFixes) {
      Write-Host (L "APTouch fix active now and on every rooted boot." "APTouch-Fix jetzt aktiv und bei jedem gerooteten Boot.") -ForegroundColor Green
    } else {
      Write-Host (L "Fix not installed (no live root or install failed) - offered again at every verified root." "Fix nicht installiert (kein live Root oder Install fehlgeschlagen) - wird bei jedem verifizierten Root erneut angeboten.") -ForegroundColor Yellow
    }
    Write-Host (L "Want every power-on rooted (no Vol-Up trick)? Boot tricks -> persistent boot. Conscious choice: it needs an eRecovery wipe." "Jeden Power-On gerootet (ohne Vol-Up-Trick)? Boot-Tricks -> persistenter Boot. Bewusste Entscheidung: braucht eRecovery-Wipe.") -ForegroundColor Gray
  }
  Write-TTLog ((L "Wizard path completed: " "Wizard-Weg fertig: ") + $goal) "SUCCESS"
}

# ============================================================ Orchestrator (P1): goals, planner, state, resume
$WorkflowGoals = @{
  "root"                   = @("reconnaissance","compatibility","firmware","extract","magisk_patch","backup","flash","reboot","root_verify","validate")
  "custom_rom"             = @("reconnaissance","compatibility","firmware","rom_validation","backup_if_required","flash_system","reboot","validate")
  "stock_rom"              = @("reconnaissance","firmware_selection","firmware_validation","artifact_extraction","backup","flash_plan","safety_gate","flash","reboot","validate")
  "root_custom_rom"        = @("reconnaissance","custom_rom_compatibility","firmware","rom_installation","boot","root_preparation","backup","root_flash","root_verify","validate")
  "root_stock_rom"         = @("reconnaissance","stock_firmware_validation","root_image_preparation","backup","root_flash","reboot","root_verify","validate")
  "root_custom_rom_recovery" = @("reconnaissance","rom_compatibility","recovery_compatibility","firmware","backup","rom_flash","recovery_flash","root_preparation","root_flash","boot","verify","validate")
  "root_stock_rom_recovery"  = @("reconnaissance","stock_firmware_validation","recovery_validation","backup","recovery_flash","root_preparation","root_flash","boot","verify","validate")
  "restore_original"       = @("reconnaissance","identify_original_artifact","validate_backup","rollback_plan","safety_gate","restore","reboot","validate")
  "full_reinstall"         = @("reconnaissance","compatibility","firmware","rom_validation","backup","wipe","flash_system","reboot","validate")
}

function Get-GoalSteps {
  # Pure, unit-testable: goal id -> ordered step list (empty = unknown goal).
  param([string]$Goal)
  $g = ([string]$Goal).ToLower().Trim()
  if ($WorkflowGoals.ContainsKey($g)) { return @($WorkflowGoals[$g]) }
  return @()
}

function Get-WorkflowStateFile {
  return (Join-Path $TTLogDir "workflow-state.json")
}

function Read-WorkflowState {
  $f = Get-WorkflowStateFile
  if (-not (Test-Path $f)) { return $null }
  try { return (Get-Content $f -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop) }
  catch { return $null }
}

function Write-WorkflowState {
  param($State)
  $State.updated = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
  ($State | ConvertTo-Json -Depth 6) | Out-File (Get-WorkflowStateFile) -Encoding utf8
}

function Save-RootState {
  # Persists last verified root state across runs (honest: re-verified on demand,
  # P10 recovery Magisk always needs the Vol-Up+Power boot for a rooted boot).
  param([string]$Root, [string]$BootMode = "")
  $f = Get-WorkflowStateFile
  $st = Read-WorkflowState
  if ($st -eq $null) { $st = @{ version = $TTVersion; goal = ""; steps = @() } }
  $lr = @{ state = $Root; timestamp = (Get-Date -Format "yyyy-MM-dd HH:mm:ss") }
  if ($BootMode -ne "") { $lr.boot_mode = $BootMode }
  elseif ($st.last_root -ne $null -and $st.last_root.boot_mode -ne $null) { $lr.boot_mode = [string]$st.last_root.boot_mode }
  $st | Add-Member -NotePropertyName "last_root" -NotePropertyValue $lr -Force
  $st.updated = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
  ($st | ConvertTo-Json -Depth 6) | Out-File $f -Encoding utf8
  Write-TTLog ("Root state persisted: " + $Root) "INFO"
}

$PersistScripts = @{
  "000-treblemanager-aptouch.sh" = "#!/system/bin/sh`n# trebleManager: stop aptouch (touchscreen edges) - runs every boot via Magisk service.d`nstop aptouch`n"
  "000-treblemanager-smartpa.sh" = "#!/system/bin/sh`n# trebleManager: smartpa speaker fix - runs every boot via Magisk service.d`nchown root:audio /dev/nxp_smartpa_dev`nchmod 0660 /dev/nxp_smartpa_dev`n"
}

function Install-PersistFixes {
  # Installs Magisk service.d boot scripts so aptouch/speaker fixes survive
  # reboots (no more manual adb after every boot). Needs live root (uid=0).
  $idOut = (Invoke-TTAdb @("shell","su -c id 2>&1") -join "").Trim()
  if ($idOut -notmatch "uid=0") {
    Write-TTLog (L "No live root (need uid=0). Boot rooted first (Vol-Up + Power), grant Magisk, retry." "Kein live Root (brauche uid=0). Erst gerootet booten (Vol-Up + Power), Magisk freigeben, erneut.") "ERROR"
    return $false
  }
  # Immediate relief now (non-destructive: stops the service, nothing is deleted).
  $stopOut = (Invoke-TTAdb @("shell","su -c 'stop aptouch' 2>&1") -join "`n").Trim()
  Write-TTLog ("Immediate stop aptouch: " + $stopOut) "INFO"
  $okAll = $true
  foreach ($name in $PersistScripts.Keys) {
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) $name
    $PersistScripts[$name] | Out-File -FilePath $tmp -Encoding ascii -NoNewline
    Invoke-TTAdb @("push",$tmp,"/sdcard/Download/" + $name) | Out-Null
    $cmd = "su -c 'cp /sdcard/Download/" + $name + " /data/adb/service.d/" + $name + " && chmod 755 /data/adb/service.d/" + $name + "' 2>&1"
    $o = (Invoke-TTAdb @("shell",$cmd) -join "`n").Trim()
    $chk = (Invoke-TTAdb @("shell","su -c 'ls -l /data/adb/service.d/" + $name + "' 2>&1") -join "`n").Trim()
    if ($chk -match $name.Replace(".","\.") -and $chk -match "rwx") {
      Write-TTLog ("Persisted: /data/adb/service.d/" + $name) "SUCCESS"
    } else {
      Write-TTLog (("FAILED: " + $name + " -- " + $o + " / " + $chk)) "ERROR"
      $okAll = $false
    }
  }
  if ($okAll) { Save-RootState "ROOTED+persisted-fixes" }
  return $okAll
}

function Screen-PersistFixes {
  Show-TTHeader (L "Persist root fixes (service.d, survives reboot)" "Root-Fixes persistieren (service.d, rebootfest)")
  Write-Host ""
  Write-Host (L "Honest scope: on P10, Magisk lives in recovery_ramdisk - every ROOTED boot needs Vol-Up + Power. There is no safe way around that." "Ehrlich: auf P10 lebt Magisk in recovery_ramdisk - jeder ROOT-Boot braucht Vol-Up + Power. Daran fuehrt kein sicherer Weg vorbei.") -ForegroundColor Yellow
  Write-Host (L "What persists instead: aptouch + speaker fixes as Magisk service.d boot scripts (no manual adb after reboot), plus the verified root state in the tool." "Was stattdessen persistiert: aptouch- + Speaker-Fixes als Magisk-service.d-Bootskripte (kein manuelles adb nach Reboot), plus verifizierter Root-Status im Tool.") -ForegroundColor White
  Write-Host ""
  Write-Host (L "Install now? Needs live root (uid=0). [Y/n]: " "Jetzt installieren? Braucht live Root (uid=0). [J/n]: ") -NoNewline -ForegroundColor Cyan
  $a = Read-Host
  if ($a -eq "" -or $a -eq "Y" -or $a -eq "y" -or $a -eq "J" -or $a -eq "j") {
    if (Install-PersistFixes) {
      Write-Host (L "Fixes will now apply on every (rooted) boot automatically." "Fixes greifen ab jetzt bei jedem (gerooteten) Boot automatisch.") -ForegroundColor Green
    }
  }
  Pause-TT
}

function New-WorkflowPlan {
  # Planner: goal + live recon -> ordered plan with per-step status. No execution.
  param([string]$Goal)
  $steps = Get-GoalSteps $Goal
  $st = Get-DeviceStates
  $plan = @()
  foreach ($s in $steps) {
    $gateStep = $s
    if ($gateStep -like "*flash*") { $gateStep = "flash" }
    elseif ($gateStep -like "*verify*" -or $gateStep -like "*validat*") { $gateStep = "verify" }
    elseif ($gateStep -like "*backup*") { $gateStep = "backup" }
    elseif ($gateStep -like "*recon*" -or $gateStep -like "*analy*") { $gateStep = "analyze" }
    else { $gateStep = "" }
    $status = "pending"
    if ($gateStep -ne "") {
      $g = Test-StepGate $gateStep
      if (-not $g.Pass) { $status = "blocked: " + ($g.Reasons -join "; ") }
    }
    $plan += (New-Object PSObject -Property @{ step = $s; status = $status })
  }
  return @{ goal = $Goal; device = $st; plan = $plan }
}

function Start-GoalWorkflow {
  # Executes a goal stepwise with persisted state; stops at first blocked/failed
  # step and offers diagnostic/restore/abort (failure engine, controlled states).
  param([string]$Goal)
  $steps = Get-GoalSteps $Goal
  if ($steps.Count -eq 0) { Write-TTLog (L "Unknown goal: " "Unbekanntes Ziel: ") + $Goal "ERROR"; return $false }
  $state = @{ version = $TTVersion; goal = $Goal; started = (Get-Date -Format "yyyy-MM-dd HH:mm:ss"); updated = ""; device = ""; steps = @() }
  foreach ($s in $steps) { $state.steps += @(@{ id = $s; status = "pending"; detail = "" }) }
  Write-WorkflowState $state
  $map = @{
    "reconnaissance" = "Screen-Analyze"; "compatibility" = "Screen-Compatibility";
    "firmware" = "Screen-Firmware"; "firmware_selection" = "Screen-Firmware";
    "extract" = "Screen-Extract"; "artifact_extraction" = "Screen-Extract";
    "magisk_patch" = "Screen-Patch"; "root_preparation" = "Screen-Patch"; "root_image_preparation" = "Screen-Patch";
    "backup" = "Screen-Backup"; "backup_if_required" = "Screen-Backup";
    "flash" = "Screen-Flash"; "root_flash" = "Screen-Flash"; "recovery_flash" = "Screen-Twrp";
    "rom_validation" = "Screen-Extract"; "rom_installation" = "Screen-FlashSystem"; "rom_flash" = "Screen-FlashSystem";
    "reboot" = "Screen-RebootVerify"; "boot" = "Screen-RebootVerify";
    "root_verify" = "Screen-RebootVerify"; "verify" = "Screen-RebootVerify";
    "validate" = "Screen-RebootVerify";
    "restore" = "Screen-Restore"; "identify_original_artifact" = "Screen-Restore"; "validate_backup" = "Screen-Restore";
    "custom_rom_compatibility" = "Screen-Compatibility"; "rom_compatibility" = "Screen-Compatibility";
    "recovery_compatibility" = "Screen-RootMethods"; "recovery_validation" = "Screen-Twrp";
    "stock_firmware_validation" = "Screen-Firmware"; "firmware_validation" = "Screen-Firmware";
    "flash_plan" = "Screen-Flash"; "safety_gate" = "Screen-Flash"; "rollback_plan" = "Screen-Restore";
    "wipe" = "Screen-Wipe"; "flash_system" = "Screen-FlashSystem"; "rom_export" = "Screen-ExportRecovery"
  }
  foreach ($entry in $state.steps) {
    $fn = $map[$entry.id]
    $entry.status = "active"
    Write-WorkflowState $state
    if ($fn -and (Get-Command $fn -ErrorAction SilentlyContinue)) {
      try { & $fn | Out-Null; $entry.status = "done"; $entry.detail = "screen completed" }
      catch { $entry.status = "failed"; $entry.detail = $_.Exception.Message }
    } else {
      $entry.status = "done"; $entry.detail = "informational/manual step"
    }
    Write-WorkflowState $state
    if ($entry.status -eq "failed" -or $entry.status -eq "blocked") {
      return (Invoke-FailureFlow $state $entry)
    }
  }
  $state.steps += @()
  Write-WorkflowState $state
  Write-TTLog ("Workflow '" + $Goal + "' COMPLETED.") "SUCCESS"
  return $true
}

function Invoke-FailureFlow {
  # Controlled failure states: BLOCKED/FAILED -> DIAGNOSTIC -> RESTORE_REQUIRED -> RESTORED/ABORTED.
  param($State, $Entry)
  Show-TTHeader (L "Step failed/blocked - controlled recovery" "Step fehlgeschlagen/blockiert - kontrollierte Recovery")
  Write-Host ""
  Write-Host (("Step: " + $Entry.id + " [" + $Entry.status + "] " + $Entry.detail)) -ForegroundColor Red
  Write-Host (L "[1] Create diagnostic ZIP  [2] Restore original  [3] Abort workflow" "[1] Diagnose-ZIP erzeugen  [2] Original restoren  [3] Workflow abbrechen") -ForegroundColor Yellow
  $k = [Console]::ReadKey($true)
  if ($k.KeyChar -eq "1") {
    $z = New-TTDiagnostic
    Write-Host ("ZIP: " + $z) -ForegroundColor White
    $Entry.status = "diagnostic"
  } elseif ($k.KeyChar -eq "2") {
    Screen-Restore
    $Entry.status = "restored"
  } else {
    $Entry.status = "aborted"
  }
  Write-WorkflowState $State
  Pause-TT
  return $false
}

function Screen-GoalSelect {
  $goals = @($WorkflowGoals.Keys | Sort-Object)
  $labels = @()
  foreach ($g in $goals) { $labels += ($g + " (" + (Get-GoalSteps $g).Count + " steps)") }
  $labels += (L "Back" "Zurueck")
  $c = Show-TTMenu (L "Select workflow goal (planner shows the plan first)" "Workflow-Ziel waehlen (Planner zeigt erst den Plan)") $labels
  if ($c -eq -1 -or $c -eq $goals.Count) { return }
  $goal = $goals[$c]
  $plan = New-WorkflowPlan $goal
  Show-TTHeader ((L "Plan for goal: " "Plan fuer Ziel: ") + $goal)
  Write-Host ""
  foreach ($p in $plan.plan) {
    $col = "Gray"
    if ($p.status -eq "pending") { $col = "White" } else { $col = "Yellow" }
    Write-Host (" - " + $p.step + " [" + $p.status + "]") -ForegroundColor $col
  }
  Write-Host ""
  Write-Host (L "Run this workflow now? [Y/n]: " "Workflow jetzt starten? [J/n]: ") -NoNewline -ForegroundColor Cyan
  $a = Read-Host
  if ($a -eq "" -or $a -eq "Y" -or $a -eq "y" -or $a -eq "J" -or $a -eq "j") {
    Start-GoalWorkflow $goal | Out-Null
  }
}

function Screen-Resume {
  $st = Read-WorkflowState
  if ($st -eq $null) {
    Write-TTLog (L "No saved workflow state to resume." "Kein gespeicherter Workflow zum Fortsetzen.") "WARNING"
    Pause-TT; return
  }
  Show-TTHeader ((L "Resume workflow: " "Workflow fortsetzen: ") + $st.goal)
  Write-Host ""
  foreach ($s in $st.steps) { Write-Host (" - " + $s.id + " [" + $s.status + "]") -ForegroundColor Gray }
  Write-Host ""
  Write-Host (L "Resume re-runs the workflow from its plan (completed screens simply run again). Continue? [Y/n]: " "Resume startet den Workflow aus seinem Plan neu (fertige Screens laufen einfach erneut). Weiter? [J/n]: ") -NoNewline -ForegroundColor Cyan
  $a = Read-Host
  if ($a -eq "" -or $a -eq "Y" -or $a -eq "y" -or $a -eq "J" -or $a -eq "j") {
    Start-GoalWorkflow $st.goal | Out-Null
  }
}

function Start-TTTui {
  Invoke-TTSelfElevate
  # Guided preflight: resolve with the user until prerequisites are met (or abort).
  for ($try = 0; $try -lt 3; $try++) {
    $pre = Invoke-Preflight
    if ($pre.Go) { break }
    Show-PreflightBlocked $pre
    $pre = Invoke-Preflight
    if ($pre.Go) { break }
    if ($try -ge 2) { return }
  }
  if (-not $pre.Go) { return }
  Write-Host ""
  Write-Host (L "PREFLIGHT READY (all green):" "PREFLIGHT READY (alles gruen):") -ForegroundColor Green
  Write-Host (" ADB: " + $pre.States.AdbState) -ForegroundColor $(if ($pre.States.AdbState -like "*READY*") { "Green" } else { "Yellow" })
  Write-Host (" Fastboot: " + $pre.States.FastbootState) -ForegroundColor $(if ($pre.States.FastbootState -like "*READY*") { "Green" } else { "Yellow" })
  Write-Host (" Admin: " + $(if (Test-TTAdmin) { "YES" } else { "NO (fastboot drivers may need it)" })) -ForegroundColor $(if (Test-TTAdmin) { "Green" } else { "Yellow" })
  if (-not (Select-TargetDevice $pre.States)) { return }
  Find-TTTools
  Update-TTMode | Out-Null
  # Defaults for target device (example values from order, overridable by analysis)
  if ($TT.ProfileId -eq "") { $TT.ProfileId = "VTR-L29" }
  Load-InstalledRom | Out-Null
  if (-not $NoElevateCheck -and -not (Test-TTAdmin)) {
    Write-TTLog (L "No admin. Fastboot/USB drivers may need admin (BAT asks for elevation, menu=admin restart)." "Kein Admin. Fastboot/USB-Treiber brauchen ggf. Admin (BAT fragt Elevation, Menue=Admin-Neustart).") "WARNING"
  }
  while ($true) {
    $c = Show-TTMenu (L "Main menu - what do you want to do?" "Hauptmenue - was willst du tun?") @(
      (L "Guided run (asks system + goal, runs automatically)" "Gefuehrter Lauf (fragt System + Ziel, laeuft automatisch)"),
      (L "Check device (status, detect, analyze, logs)" "Geraet pruefen (Status, Detect, Analyse, Logs)"),
      (L "Single steps (individual screens)" "Einzel-Steps (einzelne Screens)"),
      (L "Workflows (goals, resume, reinstall)" "Workflows (Ziele, Resume, Reinstall)"),
      (L "Settings (my system, admin restart)" "Einstellungen (mein System, Admin-Neustart)"),
      (L "Exit" "Beenden")
    ) (L "Pick one - each path guides you step by step" "Waehle eins - jeder Weg fuehrt dich Step by Step")
    if ($c -eq -1 -or $c -eq 5) { Write-TTLog ((L "Exiting. Log: " "Beendet. Log: ") + $TT.Log) "SUCCESS"; break }
    if ($c -eq 0) { Start-TTWizard }
    elseif ($c -eq 1) { Show-TTCheckMenu }
    elseif ($c -eq 2) { Show-TTStepsMenu }
    elseif ($c -eq 3) { Show-TTWorkflowMenu }
    elseif ($c -eq 4) { Show-TTSettingsMenu }
  }
}

function Show-TTCheckMenu {
  while ($true) {
    $c = Show-TTMenu (L "Check device (read-only)" "Geraet pruefen (read-only)") @(
      (L "Status overview" "Status-Uebersicht"),
      (L "Detect device" "Device erkennen"),
      (L "Analyze (OS/partitions/boot chain)" "Analyse (OS/Partitionen/Bootchain)"),
      (L "Logs + diagnostic ZIP" "Logs + Diagnose-ZIP"),
      (L "Back" "Zurueck")
    ) ""
    if ($c -eq -1 -or $c -eq 4) { return }
    if ($c -eq 0) { Show-TTStatus }
    elseif ($c -eq 1) { Screen-Detect }
    elseif ($c -eq 2) { Screen-Analyze }
    elseif ($c -eq 3) { Screen-Logs }
  }
}

function Show-TTStepsMenu {
  while ($true) {
    $c = Show-TTMenu (L "Single steps (extras)" "Einzel-Steps (Extras)") @(
      (L "Firmware (find + download)" "Firmware (finden + laden)"),
      (L "Extract stock recovery" "Stock-Recovery extrahieren"),
      (L "Recovery export (custom ROMs)" "Recovery-Export (Custom-ROMs)"),
      (L "Magisk patch" "Magisk-Patch"),
      (L "Backup" "Backup"),
      (L "Flash (safety gate)" "Flash (Safety-Gate)"),
      (L "Install ROM / GSI" "ROM / GSI installieren"),
      (L "TWRP path" "TWRP-Pfad"),
      (L "Root methods" "Root-Methoden"),
      (L "Compatibility registry" "Kompatibilitaets-Registry"),
      (L "Persist root fixes" "Root-Fixes persistieren"),
      (L "Unlock guide" "Unlock-Anleitung"),
      (L "Kernels + fixes" "Kernel + Fixes"),
      (L "Boot tricks" "Boot-Tricks"),
      (L "Wipe userdata" "Userdata wipen"),
      (L "Restore / Unroot" "Restore / Unroot"),
      (L "Reboot + verify" "Reboot + Verify"),
      (L "Tools" "Tools"),
      (L "Back" "Zurueck")
    ) ""
    if ($c -eq -1 -or $c -eq 18) { return }
    if ($c -eq 0) { Screen-Firmware }
    elseif ($c -eq 1) { Screen-Extract }
    elseif ($c -eq 2) { Screen-ExportRecovery }
    elseif ($c -eq 3) { Screen-Patch }
    elseif ($c -eq 4) { Screen-Backup }
    elseif ($c -eq 5) { Screen-Flash }
    elseif ($c -eq 6) { Screen-FlashSystem }
    elseif ($c -eq 7) { Screen-Twrp }
    elseif ($c -eq 8) { Screen-RootMethods }
    elseif ($c -eq 9) { Screen-Compatibility }
    elseif ($c -eq 10) { Screen-PersistFixes }
    elseif ($c -eq 11) { Screen-Unlock }
    elseif ($c -eq 12) { Screen-KernelFixes }
    elseif ($c -eq 13) { Screen-Bootkeys }
    elseif ($c -eq 14) { Screen-Wipe }
    elseif ($c -eq 15) { Screen-Restore }
    elseif ($c -eq 16) { Screen-RebootVerify }
    elseif ($c -eq 17) { Screen-Tools }
  }
}

function Show-TTWorkflowMenu {
  while ($true) {
    $c = Show-TTMenu (L "Workflows (planner + resume)" "Workflows (Planner + Resume)") @(
      (L "Workflow goals" "Workflow-Ziele"),
      (L "Resume saved workflow" "Gespeicherten Workflow fortsetzen"),
      (L "Full reinstall" "Komplett-Reinstall"),
      (L "Back" "Zurueck")
    ) ""
    if ($c -eq -1 -or $c -eq 3) { return }
    if ($c -eq 0) { Screen-GoalSelect }
    elseif ($c -eq 1) { Screen-Resume }
    elseif ($c -eq 2) { Screen-Reinstall }
  }
}

function Show-TTSettingsMenu {
  while ($true) {
    $c = Show-TTMenu (L "Settings" "Einstellungen") @(
      (L "My system (Stock / custom ROM)" "Mein System (Stock / Custom-ROM)"),
      (L "Admin restart" "Admin-Neustart"),
      (L "Back" "Zurueck")
    ) ((L "Phone runs: " "Handy laeuft mit: ") + (Get-RomLabel $TT.InstalledRom))
    if ($c -eq -1 -or $c -eq 2) { return }
    if ($c -eq 0) { Select-InstalledRom | Out-Null; Pause-TT }
    elseif ($c -eq 1) {
      try {
        $exe = (Get-Process -Id $PID).Path
        $sp = $MyInvocation.MyCommand.Path
        if ([string]::IsNullOrEmpty($sp)) { $sp = $PSCommandPath }
        Start-Process -FilePath $exe -ArgumentList ("-NoProfile -ExecutionPolicy Bypass -File `"$sp`"") -Verb RunAs | Out-Null
        exit 0
      } catch { Write-TTLog ((L "Elevation cancelled: " "Elevation abgebrochen: ") + $_.Exception.Message) "ERROR"; Pause-TT }
    }
  }
}

# ============================================================ CLI
function Show-TTHelp {
  Write-Host "Huawei P10 Root Manager v$TTVersion" -ForegroundColor Cyan
  Write-Host "Usage: Treble-Toolkit.ps1 [detect|devices|analyze|firmware|download|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|persist|validate|verify|restore|wipe|reinstall|rom|download-rom|diagnostic|dump-partitions|dump-properties|dump-vendor|dump-logs|preflight|recon|status|workflow|resume|root|wizard|help] [--goal <id>] [--mode safe|unattended|developer] [--json] [--yes] [--image <path>] [--firmware-file <url|path>] [--anonymize] [--no-reboot]" -ForegroundColor White
  Write-Host (L "No args: TUI. Download/flash/restore need explicit confirmation (--yes = documented consent)." "Ohne Args: TUI. Download/Flash/Restore brauchen explizite Bestaetigung (--yes = dokumentierte Zustimmung).") -ForegroundColor Gray
}

if ($Help) { Show-TTHelp; exit 0 }

# Tolerate --json/--yes style args (PS binding + manual)
$wantVal = ""
foreach ($a in $args) {
  if ($a -eq "--json") { $Json = $true }
  elseif ($a -eq "--yes") { $Yes = $true }
  elseif ($a -eq "--anonymize") { $Anonymize = $true }
  elseif ($a -eq "--no-reboot") { $NoReboot = $true }
  elseif ($a -eq "--goal" -or $a -eq "--image" -or $a -eq "--firmware-file" -or $a -eq "--mode") { $wantVal = $a }
  elseif ($wantVal -eq "--goal") { $Goal = $a; $wantVal = "" }
  elseif ($wantVal -eq "--image") { $Image = $a; $wantVal = "" }
  elseif ($wantVal -eq "--firmware-file") { $FirmwareFile = $a; $wantVal = "" }
  elseif ($wantVal -eq "--mode") { $Mode = $a; $TTMode = Resolve-RunMode $Mode; $wantVal = "" }
}

$cmd = $Command.ToLower().Trim()
if ($cmd -eq "" -and $args.Count -gt 0 -and -not $args[0].StartsWith("-")) { $cmd = $args[0].ToLower().Trim() }
if ($cmd.StartsWith("--")) { $cmd = "" }

if ($cmd -eq "") {
  Start-TTTui
  exit 0
}

Invoke-TTSelfElevate
Find-TTTools
Update-TTMode | Out-Null
if ($TT.ProfileId -eq "") { $TT.ProfileId = "VTR-L29" }

 if ($cmd -eq "help") { Show-TTHelp; exit 0 }
elseif ($cmd -eq "preflight") {
  $pre = Invoke-Preflight
  if ($Json) { (@{ go = $pre.Go; blocks = $pre.Blocks; adb = $pre.States.AdbState; fastboot = $pre.States.FastbootState } | ConvertTo-Json -Depth 3) | Write-Host }
  else {
    if ($pre.Go) { Write-Host "PREFLIGHT READY" }
    else { Write-Host "PREFLIGHT BLOCKED:"; foreach ($b in $pre.Blocks) { Write-Host (" - " + $b) } }
    Write-Host ("ADB: " + $pre.States.AdbState + " | Fastboot: " + $pre.States.FastbootState)
  }
  if (-not $pre.Go) { exit 1 }
}
elseif ($cmd -eq "recon") {
  $pre = Invoke-Preflight
  if (-not $pre.Go) { Write-Host "PREFLIGHT BLOCKED"; exit 1 }
  if ($TT.Mode -eq "" -or $TT.Mode -eq "none") { Update-TTMode | Out-Null }
  if ($TT.Mode -eq "android") { Invoke-TTAndroidAnalysis | Out-Null }
  if ($TT.Mode -eq "fastboot") { Invoke-TTFastbootAnalysis | Out-Null }
  $o = @{ mode = $TT.Mode; os = $TT.OS; profile = $TT.ProfileId; storage = $TT.Storage; props = $TT.Props }
  if ($Json) { ($o | ConvertTo-Json -Depth 6) | Write-Host }
  else { Write-Host ("mode=" + $TT.Mode + " os=" + $TT.OS.Kind + " profile=" + $TT.ProfileId + " storage=" + $TT.Storage) }
}
elseif ($cmd -eq "status") {
  Load-InstalledRom | Out-Null
  $pre = Invoke-Preflight
  $saved = Read-WorkflowState
  $full = @{ preflight_go = $pre.Go; blocks = $pre.Blocks; adb = $pre.States.AdbState; fastboot = $pre.States.FastbootState; device_mode = $TT.Mode; installed_rom = $TT.InstalledRom; run_mode = $TTMode; goal = $(if ($saved) { $saved.goal } else { "" }); saved_steps = $(if ($saved) { $saved.steps } else { @() }); log = $TT.Log }
  ($full | ConvertTo-Json -Depth 6) | Write-Host
}
elseif ($cmd -eq "rom") {
  Load-InstalledRom | Out-Null
  $sub = ""
  if ($args.Count -gt 0) { $sub = $args[0].ToLower().Trim() }
  if ($sub -eq "list") {
    $opts = Get-RomOptions
    for ($i = 0; $i -lt $opts.Count; $i++) { Write-Host (($i+1).ToString() + ". " + $opts[$i].Label + " [" + $opts[$i].Id + "]") }
  } elseif ($sub -eq "set" -and $args.Count -gt 1) {
    $v = $args[1].Trim()
    if ($v -match "^\d+$") {
      $opts = Get-RomOptions
      $idx = [int]$v - 1
      if ($idx -ge 0 -and $idx -lt $opts.Count) { $v = $opts[$idx].Id } else { Write-Host "Unknown number."; exit 3 }
    }
    Save-InstalledRom $v
    Write-Host ("Phone runs: " + (Get-RomLabel $TT.InstalledRom))
  } elseif ($sub -eq "clear") {
    Save-InstalledRom ""
    Write-Host "Installed ROM cleared."
  } else {
    if ($Json) { (@{ installed_rom = $TT.InstalledRom; label = (Get-RomLabel $TT.InstalledRom) } | ConvertTo-Json) | Write-Host }
    else { Write-Host ("Phone runs: " + (Get-RomLabel $TT.InstalledRom) + " [" + $TT.InstalledRom + "]") }
  }
}
elseif ($cmd -eq "workflow") {
  $g = $Goal
  if ($g -eq "" -and $args.Count -gt 0) { $g = $args[0] }
  $steps = Get-GoalSteps $g
  if ($steps.Count -eq 0) { Write-Host ("Unknown goal. Known: " + (($WorkflowGoals.Keys | Sort-Object) -join ", ")); exit 1 }
  $plan = New-WorkflowPlan $g
  if ($Json) { ($plan | ConvertTo-Json -Depth 6) | Write-Host }
  else { Write-Host ("Goal: " + $g); foreach ($p in $plan.plan) { Write-Host (" - " + $p.step + " [" + $p.status + "]") } }
}
elseif ($cmd -eq "resume") {
  $st = Read-WorkflowState
  if ($st -eq $null) { Write-Host "No saved workflow state."; exit 1 }
  if (-not $Yes) { Write-Host ("Resume goal '" + $st.goal + "'? Re-run with --yes to execute."); exit 4 }
  if (-not (Start-GoalWorkflow $st.goal)) { exit 1 }
}
elseif ($cmd -eq "root") {
  if (-not $Yes) { Write-Host "Root goal needs --yes (gates still enforced). Show plan: workflow --goal root [--json]."; exit 4 }
  if (-not (Start-GoalWorkflow "root")) { exit 1 }
}
elseif ($cmd -eq "devices") {
  $list = @()
  foreach ($k in ($DeviceProfiles.Keys | Sort-Object)) {
    $p = $DeviceProfiles[$k]
    $list += @{ id = $p.Id; marketing = $p.Marketing; arch = $p.Arch; soc = $p.SoC; verified = [bool]$p.Verified; variant = $p.Variant; target = $p.TargetPartition }
  }
  if ($Json) { ($list | ConvertTo-Json -Depth 3) | Write-Host }
  else { foreach ($d in $list) { Write-Host (" - " + $d.id + " (" + $d.marketing + ", " + $d.arch + ") verified=" + $d.verified + " target=" + $d.target) } }
}
elseif ($cmd -eq "detect") {
  $o = @{ mode = $TT.Mode; adb = $TT.AdbSerial; fastboot = $TT.FbSerial; adb_path = $TT.Adb; fastboot_path = $TT.Fastboot }
  if ($Json) { ($o | ConvertTo-Json -Depth 3) | Write-Host } else { ($o.GetEnumerator() | ForEach-Object { "$($_.Key): $($_.Value)" }) | ForEach-Object { Write-Host $_ } }
}
elseif ($cmd -eq "analyze") {
  if ($TT.Mode -eq "android") { Invoke-TTAndroidAnalysis | Out-Null }
  if ($TT.Mode -eq "fastboot") { Invoke-TTFastbootAnalysis | Out-Null }
  $o = @{ mode = $TT.Mode; os = $TT.OS; props = $TT.Props; byname = ($TT.ByName | ForEach-Object { "$($_.Name) -> $($_.Target)" }); fastboot = $TT.FbVars }
  if ($Json) { ($o | ConvertTo-Json -Depth 6) | Write-Host }
  else {
    Write-Host ("OS: " + $TT.OS.Kind + " | " + $TT.OS.Detail)
    Write-Host (L "Partitions:" "Partitionen:"); foreach ($n in $TT.ByName) { Write-Host (" " + $n.Name + " -> " + $n.Target) }
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
  # Read-only: analyze UPDATE.APP + validate existing RECOVERY images. No flash.
  $appPath = $FirmwareFile
  if ($appPath -eq "" -and $Image -ne "") { $appPath = $Image }
  if ($appPath -eq "") {
    $apps = Get-ChildItem -Path $TTFirmDir -Filter "*.APP" -File -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($apps) { $appPath = $apps.FullName }
  }
  $appInfo = $null
  if ($appPath -ne "" -and (Test-Path $appPath)) { $appInfo = Invoke-TTUpdateAppAnalysis $appPath }
  else { $appInfo = @{ Exists = $false; Size = 0; HeaderHex = ""; Note = (L "No UPDATE.APP found. Place full firmware in data/firmware/ (no auto-download from dubious sources)." "Keine UPDATE.APP gefunden. Lege Full-Firmware nach data/firmware/ (kein Auto-Download aus dubiosen Quellen).") } }
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
    if ($validated.Count -eq 0) { Write-Host (L "No RECOVERY_RAMDIS(K).img in data/firmware/ - obtain it first." "Keine RECOVERY_RAMDIS(K).img in data/firmware/ - erst beschaffen.") }
    foreach ($v in $validated) { Write-Host (" - " + $v.path + " [" + $v.verdict + "] SHA256=" + $v.sha256) }
  }
  if ($validated.Count -eq 0) { exit 3 }
}
elseif ($cmd -eq "export") {
  # Export recovery/boot images from custom ROM packages. Read-only except data/recovery/.
  $rom = $Image
  if ($rom -eq "" -and $FirmwareFile -ne "") { $rom = $FirmwareFile }
  if ($rom -eq "") {
    $found = @()
    foreach ($ext in @("*.zip","*.img","*.tar","*.tar.gz","*.tgz","*.gz","*.xz")) {
      $hits = Get-ChildItem -Path $TTRomDir -Filter $ext -File -ErrorAction SilentlyContinue
      foreach ($h in $hits) { $found += $h.FullName }
    }
    if ($found.Count -gt 0) { $rom = $found[0] }
  }
  if ($rom -eq "" -or -not (Test-Path $rom)) { Write-Host (L "No ROM package found. Place .img/.img.gz/.img.xz/.zip/.tar.gz in data/roms/ or pass --image <path>." "Kein ROM-Paket gefunden. .img/.img.gz/.img.xz/.zip/.tar.gz nach data/roms/ legen oder --image <Pfad> nutzen.") ; exit 3 }
  $e = Export-RecoveryFromRom $rom
  if ($Json) { ($e | ConvertTo-Json -Depth 5) | Write-Host }
  else { foreach ($n in $e.Notes) { Write-Host (" - " + $n) }; foreach ($f in $e.Files) { Write-Host (" [+] " + $f) } }
  if (-not $e.Ok) { exit 3 }
}
elseif ($cmd -eq "patch") {
  # Read-only prep: validate stock + to-patch staging + instructions. NO fake patch.
  $stock = $Image
  if ($stock -eq "" -and $TT.StockImage -ne "") { $stock = $TT.StockImage }
  if ($stock -eq "") {
    $found = Find-TTRecoveryImage
    if ($found.Count -gt 0) { $stock = $found[0] }
  }
  if ($stock -eq "" -or -not (Test-Path $stock)) { Write-Host (L "No stock image found. Run 'extract' first (UPDATE.APP -> data/firmware/)." "Kein Stock-Image gefunden. Erst 'extract' (UPDATE.APP -> data/firmware/).") ; exit 3 }
  $chk = Test-RecoveryImageFile $stock
  if ($chk.Verdict -ne "PASS") { Write-Host ("Stock image FAIL: " + ($chk.Notes -join " | ")) ; exit 3 }
  $TT.StockImage = $stock; $TT.StockHash = $chk.Hash
  $apk = Find-TTMagiskApk
  $mi = $null
  if ($apk -ne "") { $mi = Get-TTMagiskInfo $apk }
  $staged = Prepare-TTMagiskPatch $stock
  $o = @{ stock = $stock; sha256 = $chk.Hash.SHA256; magisk_apk = $apk; magisk = $mi; staged = $staged; next = "On device: Magisk -> Install -> Select and Patch a File -> adb pull /sdcard/Download/magisk_patched-*.img data/magisk/" }
  if ($Json) { ($o | ConvertTo-Json -Depth 5) | Write-Host }
  else { Write-Host ("Stock OK. Staged: " + $staged); Write-Host (L "Next step: patch on device, then 'Register patched file' in TUI." "Naechster Schritt: on-device patchen, dann in TUI 'Gepatchte Datei registrieren'.") }
}
elseif ($cmd -eq "backup") {
  $stock = $Image
  if ($stock -eq "" -and $TT.StockImage -ne "") { $stock = $TT.StockImage }
  if ($stock -eq "") {
    $found = Find-TTRecoveryImage
    if ($found.Count -gt 0) { $stock = $found[0] }
  }
  if ($stock -eq "" -or -not (Test-Path $stock)) { Write-Host (L "No stock image for backup. Run 'extract' first." "Kein Stock-Image fuer Backup. Erst 'extract'.") ; exit 3 }
  $chk = Test-RecoveryImageFile $stock
  if ($chk.Verdict -ne "PASS") { Write-Host (L "Stock image not verifiable - backup will NOT be faked." "Stock-Image nicht verifizierbar - Backup wird NICHT vorgetaeuscht.") ; exit 3 }
  $TT.StockImage = $stock; $TT.StockHash = $chk.Hash
  $r = New-TTBackup $stock
  if ($Json) { ($r | ConvertTo-Json -Depth 3) | Write-Host } else { Write-Host ("Backup: " + $r.Dir + " OK=" + $r.Ok) }
  if (-not $r.Ok) { exit 3 }
}
elseif ($cmd -eq "download") {
  # Download stock firmware (original). Without --yes only show candidate, no traffic.
  $url = $FirmwareFile
  if ($url -eq "" -and $Image -ne "") { $url = $Image }
  if ($url -eq "") { Write-Host "URL missing. Example: Treble-Toolkit.ps1 download --firmware-file <https-URL-to-full-ZIP> [--yes]"; exit 4 }
  $chk = Test-FirmwareUrl $url
  if (-not $chk.Ok) { Write-Host ((L "URL rejected: " "URL abgelehnt: ") + $chk.Reason); exit 4 }
  $fname = "stock-firmware-" + $TT.ProfileId + "-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".zip"
  $out = Join-Path $TTFirmDir $fname
  if (-not $Yes) {
    $o = @{ url = $url; host = $chk.Host; out = $out; note = (L "Confirmation missing. Download with --yes (2-4 GB)." "Bestaetigung fehlt. Mit --yes herunterladen (2-4 GB).") }
    if ($Json) { ($o | ConvertTo-Json -Depth 3) | Write-Host } else { Write-Host ((L "Ready (not loaded): " "Bereit (nicht geladen): ") + $url + " -> " + $out + (L " | Confirm with --yes." " | Mit --yes bestaetigen.")) }
    exit 4
  }
  $res = Invoke-FirmwareDownload $url $out
  if (-not $res.Ok) { exit 1 }
  $ver = Test-DownloadedFirmware $out
  $o = @{ path = $out; sha256 = $ver.SHA256; ok = $ver.Ok; notes = $ver.Notes }
  if ($Json) { ($o | ConvertTo-Json -Depth 4) | Write-Host } else { Write-Host ((L "Done: " "Fertig: ") + $out + " OK=" + $ver.Ok) }
  if (-not $ver.Ok) { exit 3 }
}
elseif ($cmd -eq "download-rom") {
  # Download working custom ROM/GSI from verified registry links (numbered).
  $pick = ""
  if ($args.Count -gt 0 -and $args[0] -notmatch "^-") { $pick = $args[0] }
  $ready = Invoke-RomDownload -Pick $pick -ForceYes:$Yes
  if ($ready -eq "") { exit 1 }
  $o = @{ ready = $ready; kind = (Test-ImageKind $ready) }
  if ($Json) { ($o | ConvertTo-Json -Depth 3) | Write-Host } else { Write-Host ((L "Ready: " "Fertig: ") + $ready) }
}
elseif ($cmd -eq "flash") {
  if ($Image -ne "") { $TT.PatchedImage = $Image; $TT.PatchedHash = (Test-RecoveryImageFile $Image).Hash }
  $ok = Invoke-TTSafeFlash -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "flash-system") {
  if ($Image -eq "" -or -not (Test-Path $Image)) { Write-Host (L "GSI image path missing. Use --image <path-to-gsi.img>." "GSI-Image-Pfad fehlt. Nutze --image <Pfad-zu-gsi.img>."); exit 3 }
  $ok = Invoke-SystemFlash $Image -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "twrp") {
  if ($Image -eq "" -or -not (Test-Path $Image)) { Write-Host (L "TWRP image path missing. Use --image <path-to-twrp.img>." "TWRP-Image-Pfad fehlt. Nutze --image <Pfad-zu-twrp.img>."); exit 3 }
  $ok = Invoke-TwrpFlash $Image -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "root-methods") {
  $ordered = Get-PreferredRootMethod
  if ($Json) { ($ordered | ConvertTo-Json -Depth 3) | Write-Host }
  else { foreach ($m in $ordered) { Write-Host (" - " + $m.Id + ": " + $m.Name + $(if ($m.Preferred) { " [PREFERRED]" } else { "" })) } }
}
elseif ($cmd -eq "compat") {
  $reg = Get-CompatRegistry
  if ($reg -eq $null) { Write-Host (L "No registry for this profile." "Keine Registry fuer dieses Profil."); exit 3 }
  if ($Json) { ($reg | ConvertTo-Json -Depth 6) | Write-Host }
  else {
    Write-Host ("Profile: " + $TT.ProfileId + " (variant " + $reg.device.variant + ")")
    Write-Host "Recommended:"
    foreach ($r in $reg.roms) { if ($r.status -like "working*") { Write-Host (" [+] " + $r.name + " [" + $r.status + "]") } }
    Write-Host "Not recommended:"
    foreach ($r in $reg.roms) { if ($r.status -notlike "working*") { Write-Host (" [X] " + $r.name + " [" + $r.status + "] - " + $r.reason) } }
  }
}
elseif ($cmd -eq "verify") {
  $r = Invoke-TTRootVerification -NoReboot:$NoReboot
  Save-RootState $r.Root
  if ($Json) { ($r | ConvertTo-Json -Depth 4) | Write-Host }
  if ($r.Root -ne "ROOTED") { exit 2 }
}
elseif ($cmd -eq "persist") {
  if (-not $Yes) { Write-Host (L "Needs live root + --yes (writes service.d scripts)." "Braucht live Root + --yes (schreibt service.d-Skripte)."); exit 4 }
  if (-not (Install-PersistFixes)) { exit 1 }
}
elseif ($cmd -eq "validate") {
  $rep = Invoke-TTValidate
  if ($Json) { ($rep | ConvertTo-Json -Depth 5) | Write-Host }
  else { Write-ValidationReport $rep | Out-Null }
  if (-not $rep.ok) { exit 2 }
}
elseif ($cmd -eq "dump-partitions" -or $cmd -eq "dump-properties" -or $cmd -eq "dump-vendor" -or $cmd -eq "dump-logs") {
  if (-not (Invoke-DeveloperDump $cmd)) { exit 1 }
}
elseif ($cmd -eq "restore") {
  $ok = Invoke-TTRestoreFlow -BackupPick $Image -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "wipe") {
  if (-not $Yes) { Write-Host (L "Wipe needs --yes (erases all user data)." "Wipe braucht --yes (loescht alle Nutzerdaten)."); exit 4 }
  if (-not (Invoke-GuidedWipe -ForceYes)) { exit 1 }
}
elseif ($cmd -eq "reinstall") {
  if (-not $Yes) { Write-Host (L "Reinstall needs --yes (flashes system, optional wipe)." "Reinstall braucht --yes (flasht System, optional Wipe)."); exit 4 }
  if ($Image -eq "" -or -not (Test-Path $Image)) { Write-Host (L "Reinstall needs --image <gsi-or-system.img> (custom), stock path is guided in TUI." "Reinstall braucht --image <gsi-oder-system.img> (custom), Stock-Weg gefuehrt in TUI."); exit 3 }
  $ok = Invoke-SystemFlash $Image -ForceYes:$Yes
  if (-not $ok) { exit 1 }
}
elseif ($cmd -eq "wizard") { Start-TTTui }
else {
  Write-Host ((L "Unknown command: " "Unbekanntes Kommando: ") + $cmd) -ForegroundColor Yellow
  Show-TTHelp
  exit 1
}
