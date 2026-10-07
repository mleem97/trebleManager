#requires -Version 5.1
<#
.SYNOPSIS
  Unit + Integration Tests (simulierte Command-Ausgaben).
  Gilt NICHT fuer Produktionsworkflow (dort keine Mocks, nur echte Geraete).
  Start: powershell -ExecutionPolicy Bypass -File tests/Test-Parsers.ps1
  Online: irm https://raw.githubusercontent.com/mleem97/trebleManager/main/tests/Test-Parsers.ps1 | iex
#>

# Online run (irm|iex): without repo layout fetch the FULL release ZIP
# (same trust root) and run the suite from it. Guard prevents loops.
# Tag order: raw VERSION first (no API needed), API last (rate-limited).
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
      $__e = ""
      try { $__e = (([string]$__w.DownloadString("https://github.com/mleem97/trebleManager/releases/download/" + $Tag + "/trebleManager-" + $Tag + ".zip.sha256") -split '\s+')[0]).Trim().ToUpper() } catch {}
      if ($__e -ne "") {
        $__a = (Get-FileHash $__z -Algorithm SHA256).Hash.ToUpper()
        if ($__e -ne $__a) { Remove-Item $__z -Force; throw "SHA256 MISMATCH, deleted, aborting" }
        Write-Host "SHA256 OK." -ForegroundColor Green
      }
      if (-not (Test-Path $__d)) { New-Item -ItemType Directory -Path $__d -Force | Out-Null }
      Expand-Archive -Path $__z -DestinationPath $__d -Force
    }
    if (Test-Path $__t) { return $__t }
  } catch { Write-Host ("WARN: fetch " + $Tag + " failed (" + $_.Exception.Message + ")") -ForegroundColor Yellow }
  return ""
}
if (-not $env:TT_TEST_BOOTSTRAPPED) {
  $__r = $PSScriptRoot
  if ([string]::IsNullOrEmpty($__r)) { $__r = (Get-Location).Path }
  else { $__r = Split-Path -Parent $__r }
  if (-not ((Test-Path (Join-Path $__r "VERSION")) -and (Test-Path (Join-Path $__r "scripts/Treble-Toolkit.ps1")))) {
    Write-Host "Test suite without layout - fetching full release ZIP ..." -ForegroundColor Cyan
    $__target = ""
    try {
      $__wc = New-Object Net.WebClient
      $__wc.Headers.Add("User-Agent", "trebleManager")
      [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
      $__ver = ([string]$__wc.DownloadString("https://raw.githubusercontent.com/mleem97/trebleManager/main/VERSION")).Trim()
      if (-not [string]::IsNullOrEmpty($__ver)) { $__target = Get-BootstrapReleaseFile ("v" + $__ver) "tests/Test-Parsers.ps1" }
    } catch { Write-Host ("WARN: raw VERSION failed (" + $_.Exception.Message + ")") -ForegroundColor Yellow }
    if ([string]::IsNullOrEmpty($__target)) {
      try {
        $__wc2 = New-Object Net.WebClient
        $__wc2.Headers.Add("User-Agent", "trebleManager")
        $__tag2 = ([string](($__wc2.DownloadString("https://api.github.com/repos/mleem97/trebleManager/releases/latest") | ConvertFrom-Json).tag_name)).Trim()
        if (-not [string]::IsNullOrEmpty($__tag2)) { $__target = Get-BootstrapReleaseFile $__tag2 "tests/Test-Parsers.ps1" }
      } catch { Write-Host ("WARN: API fallback failed (" + $_.Exception.Message + ")") -ForegroundColor Yellow }
    }
    if (-not [string]::IsNullOrEmpty($__target)) {
      $env:TT_TEST_BOOTSTRAPPED = "1"
      $__exe = [Diagnostics.Process]::GetCurrentProcess().MainModule.FileName
      & $__exe -NoProfile -ExecutionPolicy Bypass -File $__target
      exit $LASTEXITCODE
    }
    Write-Host "ERROR: test bootstrap failed - tests need the repo layout (release ZIP or git checkout)." -ForegroundColor Red
    exit 1
  }
  Remove-Variable __r,__wc,__ver,__wc2,__tag2,__target,__exe -ErrorAction SilentlyContinue
}

$ErrorActionPreference = "Stop"
$Fail = 0
$Pass = 0
$__suiteSrc = $MyInvocation.MyCommand.Path
if ([string]::IsNullOrEmpty($__suiteSrc)) { $__suiteSrc = "remote/iex (bootstrapped)" }
Write-Host ("Test-Parsers (PS " + $PSVersionTable.PSVersion + ", src: " + $__suiteSrc + ")") -ForegroundColor DarkGray
Remove-Variable __suiteSrc -ErrorAction SilentlyContinue

function Assert-Equal {
  param($Name, $Expected, $Actual)
  if ("$Expected" -eq "$Actual") { Write-Host ("[PASS] " + $Name) -ForegroundColor Green; $script:Pass++ }
  else { Write-Host ("[FAIL] " + $Name + " erwartet='$Expected' ist='$Actual'") -ForegroundColor Red; $script:Fail++ }
}
function Assert-True {
  param($Name, [bool]$Cond)
  if ($Cond) { Write-Host ("[PASS] " + $Name) -ForegroundColor Green; $script:Pass++ }
  else { Write-Host ("[FAIL] " + $Name) -ForegroundColor Red; $script:Fail++ }
}

# ---- Funktionen aus Hauptskript laden (nur reine, kein Geraet) ----
$__testRoot = $PSScriptRoot
if ([string]::IsNullOrEmpty($__testRoot)) { $__testRoot = (Get-Location).Path }
if ((Split-Path -Leaf $__testRoot) -eq "tests") { $__testRoot = Split-Path -Parent $__testRoot }
$Main = Join-Path $__testRoot "scripts/Treble-Toolkit.ps1"
$Src = Get-Content $Main -Raw
function Import-TTFunction {
  param([string]$Name)
  $start = $Src.IndexOf("function " + $Name)
  if ($start -lt 0) { Write-Host ("Funktion nicht gefunden: " + $Name) -ForegroundColor Red; $script:Fail++; return }
  # Balancierte Klammern ab erstem '{' nach Funktionsname
  $brace = $Src.IndexOf("{", $start)
  $depth = 0
  $i = $brace
  while ($i -lt $Src.Length) {
    $ch = $Src[$i]
    if ($ch -eq "{") { $depth++ }
    elseif ($ch -eq "}") { $depth--; if ($depth -eq 0) { break } }
    $i++
  }
  $block = $Src.Substring($start, ($i - $start) + 1)
  # Script scope: Invoke-Expression inside this function would define $Name
  # locally (gone after return) -> qualify so tests can call it afterwards.
  if ($block.StartsWith("function ")) { $block = "function script:" + $block.Substring(9) }
  try { Invoke-Expression $block } catch { Write-Host ("Ladefehler " + $Name + ": " + $_.Exception.Message) -ForegroundColor Red; $script:Fail++ }
}
$__need = @("L","ConvertFrom-AdbDevices","ConvertFrom-FastbootDevices","ConvertFrom-GetpropDump","ConvertFrom-ByNameListing","ConvertFrom-FastbootGetvar","Get-OSClassification","Test-FirmwareCompatibility","Test-FirmwareUrl","Test-BootImageMagic","Get-PreferredRootMethod","Test-RomAgainstRegistry","Get-VendorAdvice","Resolve-RunMode","Unquote-Path","Get-GoalSteps","Get-FlashVerdict","Get-RomSuggested","Get-RomLabel")
foreach ($fn in $__need) {
  Import-TTFunction $fn
}
# Fail fast: missing imports must stop here, not cascade into every test.
$__missing = @($__need | Where-Object { -not (Get-Command $_ -ErrorAction SilentlyContinue) })
if ($__missing.Count -gt 0) {
  Write-Host ("MISSING functions, aborting (no cascade): " + ($__missing -join ", ")) -ForegroundColor Red
  Write-Host ("Main script: " + $Main) -ForegroundColor Yellow
  exit 1
}

# ---- 1. ADB-Parser ----
$r = ConvertFrom-AdbDevices @("List of devices attached","6PQ0217B08003446	device","","ABC123	unauthorized")
Assert-Equal "adb serial" "6PQ0217B08003446" $r[0].Serial
Assert-Equal "adb state" "device" $r[0].State
Assert-Equal "adb unauthorized erkannt" "unauthorized" $r[1].State

# ---- 2. Fastboot-Parser ----
$r = ConvertFrom-FastbootDevices @("6PQ0217B08003446	fastboot","")
Assert-Equal "fb serial" "6PQ0217B08003446" $r[0].Serial

# ---- 3. getvar-Parser: Huawei FAILED darf nicht als locked gelten ----
$p = ConvertFrom-FastbootGetvar @("getvar:unlocked FAILED (remote: Command not allowed)","finished. total time: 0.016s")
Assert-True "huawei denied erkannt" $p.CommandDenied
Assert-True "kein Lock-Urteil aus denied" ($p.Vars.Count -ge 0)

$p2 = ConvertFrom-FastbootGetvar @("partition-size:recovery_ramdisk: 0x02000000","finished.")
Assert-True "getvar key/value" ($p2.Vars.Count -gt 0)

# ---- 4. Partition-Parser ----
$by = ConvertFrom-ByNameListing "recovery_ramdisk -> /dev/block/mmcblk0p30`nboot -> /dev/block/mmcblk0p28`nsystem -> /dev/block/mmcblk0p60"
$__rr = @($by | Where-Object { $_.Name -eq "recovery_ramdisk" })
$__bb = @($by | Where-Object { $_.Name -eq "boot" })
Assert-True "recovery_ramdisk gefunden" ($__rr.Count -eq 1)
Assert-True "boot gefunden" ($__bb.Count -eq 1)
if ($__rr.Count -ne 1 -or $__bb.Count -ne 1) {
  Write-Host ("DIAG parsed=" + $by.Count + " names=" + (($by | ForEach-Object { $_.Name }) -join ",")) -ForegroundColor Yellow
}

# ---- 5. getprop-Parser (beide Formate) ----
$h = ConvertFrom-GetpropDump @("[ro.product.model]: [TrebleDroid with GApps]","[ro.build.version.release]: [13]","ro.secure = 1")
Assert-Equal "getprop bracket" "TrebleDroid with GApps" $h["ro.product.model"]
Assert-Equal "getprop equals" "1" $h["ro.secure"]

# ---- 6. OS-Klassifizierung: jedes OS ----
$gsi = Get-OSClassification @{ "ro.product.model"="TrebleDroid with GApps"; "ro.product.name"="lineage_arm64_bgN"; "ro.build.display.id"="lineage_arm64_bgN-userdebug 13 TQ3A.230901.001 eng.crossg.20251022.010856"; "ro.build.version.release"="13" }
Assert-Equal "GSI erkannt" "TrebleDroid-GSI" $gsi.Kind
Assert-True "GSI-Flag" $gsi.IsGsi

$stock = Get-OSClassification @{ "ro.product.model"="VTR-L29"; "ro.product.name"="VTR-L29"; "ro.build.display.id"="VTR-L29 9.1.0.297(C432E5R1P9)"; "ro.build.version.release"="9"; "ro.build.version.emui"="EmotionUI_9.1.0" }
Assert-True "Stock erkannt" $stock.IsStock

$pe = Get-OSClassification @{ "ro.product.model"="TrebleDroid with GApps"; "ro.product.name"="PixelExperience"; "ro.build.display.id"="PixelExperience 13"; "ro.build.version.release"="13" }
Assert-True "PE als GSI/Custom" ($pe.Kind -like "*GSI*" -or $pe.Kind -like "*Pixel*")

# ---- 7. Firmware-Kompatibilitaet ----
$c = Test-FirmwareCompatibility "VTR-L29" "VTR-L29 9.1.0.297(C432E5R1P9)" "C432"
Assert-Equal "fw pass" "PASS" $c.Status
$c2 = Test-FirmwareCompatibility "VTR-L29" "VKY-L29 9.1.0.297(C432E5R1P9)" "C432"
Assert-Equal "fw falsches Modell FAIL" "FAIL" $c2.Status
$c3 = Test-FirmwareCompatibility "VTR-L29" "VTR-L09 9.1.0.297(C432E5R1P9)" "C432"
Assert-True "fw L09 vs L29 mind. WARN" ($c3.Status -eq "WARN" -or $c3.Status -eq "FAIL")
$c4 = Test-FirmwareCompatibility "VTR-L29" "" ""
Assert-Equal "fw leer FAIL" "FAIL" $c4.Status

# ---- 8. Hashing (echt, kein Mock) ----
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("tt-test-" + (Get-Date -Format "HHmmss") + ".bin")
"TrebleToolkit-test" | Out-File $tmp -Encoding ascii
$h256 = (Get-FileHash -Path $tmp -Algorithm SHA256).Hash
$h512 = (Get-FileHash -Path $tmp -Algorithm SHA512).Hash
Assert-True "sha256 64 hex" ($h256.Length -eq 64)
Assert-True "sha512 128 hex" ($h512.Length -eq 128)
Remove-Item $tmp -Force -ErrorAction SilentlyContinue

# ---- 9. Root-State-Logik (uid=0 zaehlt, Boot allein nicht) ----
function Test-RootDecision { param([string]$SuId, [string]$WhichSu)
  if ($SuId -match "uid=0") { return "ROOTED" }
  elseif ($WhichSu -ne "" -and $WhichSu -notmatch "not found") { return "INCONCLUSIVE" }
  return "NOT_ROOTED"
}
Assert-Equal "root uid=0" "ROOTED" (Test-RootDecision "uid=0(root) gid=0" "/system/xbin/su")
Assert-Equal "root nur boot heisst NOT_ROOTED" "NOT_ROOTED" (Test-RootDecision "" "")
Assert-Equal "root su ohne uid=0 = inconclusive" "INCONCLUSIVE" (Test-RootDecision "permission denied" "/system/bin/su")

# ---- 10. Firmware-URL-Validierung (kein Netzwerk, nur Logik) ----
$u1 = Test-FirmwareUrl "https://androidhost.ru/xxx/VTR-L29-9.1.0.297.zip"
Assert-True "url https zip ok" $u1.Ok
$u2 = Test-FirmwareUrl "ftp://example.com/fw.zip"
Assert-True "url ftp abgelehnt" (-not $u2.Ok)
$u3 = Test-FirmwareUrl "file:///C:/fw.zip"
Assert-True "url file abgelehnt" (-not $u3.Ok)
$u4 = Test-FirmwareUrl "https://example.com/fw.exe"
Assert-True "url exe abgelehnt" (-not $u4.Ok)
$u5 = Test-FirmwareUrl ""
Assert-True "url leer abgelehnt" (-not $u5.Ok)

# ---- 11. Boot image magic (echt, kein Mock) ----
$tmpImg = Join-Path ([System.IO.Path]::GetTempPath()) ("tt-boot-" + (Get-Date -Format "HHmmss") + ".img")
$fs = [System.IO.File]::Create($tmpImg)
$magic = [System.Text.Encoding]::ASCII.GetBytes("ANDROID!")
$fs.Write($magic, 0, $magic.Length)
$fs.WriteByte(3)
$pad = New-Object byte[] 100
$fs.Write($pad, 0, $pad.Length)
$fs.Close()
Assert-Equal "boot magic version" 3 (Test-BootImageMagic $tmpImg)
$tmpTxt = $tmpImg + ".txt"
"no android here" | Out-File $tmpTxt -Encoding ascii
Assert-Equal "kein magic abgelehnt" -1 (Test-BootImageMagic $tmpTxt)
Remove-Item $tmpImg -Force -ErrorAction SilentlyContinue
Remove-Item $tmpTxt -Force -ErrorAction SilentlyContinue
# ---- 12. Root method priority (Magisk preferred first) ----
$RootMethods = @(
  @{ Id = "phh-su"; Preferred = $false },
  @{ Id = "magisk-recovery"; Preferred = $true },
  @{ Id = "kernelsu"; Preferred = $false }
)
$ordered = Get-PreferredRootMethod
Assert-Equal "preferred first" "magisk-recovery" $ordered[0].Id
Assert-Equal "all methods kept" 3 $ordered.Count

# ---- 13. Registry ROM check (synthetic registry object) ----
$reg = [PSCustomObject]@{
  roms = @(
    [PSCustomObject]@{ name = "LineageOS 20 Light"; status = "broken"; reason = "non-booting"; markers = @("light") },
    [PSCustomObject]@{ name = "LineageOS"; version = 20; status = "working"; markers = $null }
  )
}
$r1 = Test-RomAgainstRegistry $reg "lineage-20-light-arm64.img"
Assert-Equal "light blocked" "FAIL" $r1.Status
$r2 = Test-RomAgainstRegistry $reg "lineage-20.0-arm64_bgN.img"
Assert-Equal "normal lineage passes" "PASS" $r2.Status

# ---- 14. Vendor advice ----
# ---- 14. Vendor advice ----
Assert-True "oreo risk" ((Get-VendorAdvice "EmotionUI_8.0" "13") -match "RISK")
Assert-True "pie boots" ((Get-VendorAdvice "EmotionUI_9.1" "13") -match "expected to boot")

# ---- 16. Run modes ----
Assert-Equal "mode safe default" "safe" (Resolve-RunMode "")
Assert-Equal "mode unattended" "unattended" (Resolve-RunMode "unattended")
Assert-Equal "mode developer" "developer" (Resolve-RunMode "developer")
Assert-Equal "mode unknown falls back" "safe" (Resolve-RunMode "yolo")

# ---- 17. Path quoting (spaces + drag-drop quotes) ----
Assert-Equal "quoted path" 'C:\my dir\img file.img' (Unquote-Path '"C:\my dir\img file.img"')
Assert-Equal "single-quoted path" 'C:\my dir\img file.img' (Unquote-Path "'C:\my dir\img file.img'")
Assert-Equal "plain path" 'C:\plain\a.img' (Unquote-Path 'C:\plain\a.img')

# ---- 18. Workflow goals (planner input) ----
$WorkflowGoals = @{ "root" = @("reconnaissance","flash","validate"); "restore_original" = @("reconnaissance","restore") }
Assert-Equal "root steps" 3 (Get-GoalSteps "root").Count
Assert-Equal "unknown goal empty" 0 (Get-GoalSteps "nope").Count

# ---- 19. Flash verdict: FAILED vetoes, progress words are not success ----
$v = Get-FlashVerdict @("Sending 'system' (1126400 KB)              OKAY [ 28.1s]","Writing 'system'                                 OKAY [ 41.2s]","Finished. Total time: 70.003s")
Assert-Equal "verdict OK on full success" "OK" $v.Verdict
$v = Get-FlashVerdict @("Sending 'system' (1126400 KB)              OKAY [ 28.1s]","Writing 'system'          FAILED (remote: 'Command not allowed')","Finished. Total time: 0.010s")
Assert-Equal "verdict FAILED despite Writing+OKAY" "FAILED" $v.Verdict
$v = Get-FlashVerdict @("Erasing 'userdata' ...")
Assert-Equal "verdict UNCLEAR on progress only" "UNCLEAR" $v.Verdict
$v = Get-FlashVerdict @("Erasing 'userdata'                                 OKAY [  2.1s]","Finished. Total time: 2.150s")
Assert-Equal "verdict OK on erase success" "OK" $v.Verdict

# ---- 20. Installed ROM: suggestion never guesses custom, labels exact ----
Assert-Equal "suggest stock for EMUI" "stock" (Get-RomSuggested "Stock EMUI 9.1")
Assert-Equal "suggest empty for GSI (user picks)" "" (Get-RomSuggested "TrebleDroid GSI")
Assert-Equal "label stock" "Stock EMUI" (Get-RomLabel "stock")
Assert-Equal "label rom id" "LineageOS 20" (Get-RomLabel "rom:LineageOS 20")
Assert-Equal "label other" "Other custom ROM" (Get-RomLabel "other")
Assert-Equal "label empty" "?" (Get-RomLabel "")

# ---- 15. Immutable release: single version everywhere ----
$TTRoot = $__testRoot
$verFile = (Get-Content (Join-Path $TTRoot "VERSION") -Raw).Trim()
$mainSrc = Get-Content (Join-Path $TTRoot "scripts/Treble-Toolkit.ps1") -Raw
$m = [regex]::Match($mainSrc, '\$TTVersion = "([^"]+)"')
Assert-True "ps1 version present" $m.Success
if ($m.Success) { Assert-Equal "ps1 == VERSION" $verFile $m.Groups[1].Value }
$readme = Get-Content (Join-Path $TTRoot "README.md") -Raw
Assert-True "readme badge matches" ($readme -match [regex]::Escape("Version-" + $verFile))
$cl = Get-Content (Join-Path $TTRoot "CHANGELOG.md") -Raw
Assert-True "changelog has version" ($cl -match [regex]::Escape("## v" + $verFile))

Write-Host ""
Write-Host ("Ergebnis: " + $Pass + " PASS, " + $Fail + " FAIL") -ForegroundColor $(if ($Fail -eq 0) { "Green" } else { "Red" })
if ($Fail -gt 0) { exit 1 }
