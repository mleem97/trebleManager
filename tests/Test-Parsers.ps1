#requires -Version 5.1
<#
.SYNOPSIS
  Unit + Integration Tests (simulierte Command-Ausgaben).
  Gilt NICHT fuer Produktionsworkflow (dort keine Mocks, nur echte Geraete).
  Start: powershell -ExecutionPolicy Bypass -File tests/Test-Parsers.ps1
#>
$ErrorActionPreference = "Stop"
$Fail = 0
$Pass = 0

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
$Main = Join-Path (Split-Path -Parent $PSScriptRoot) "scripts/Treble-Toolkit.ps1"
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
  try { Invoke-Expression $block } catch { Write-Host ("Ladefehler " + $Name + ": " + $_.Exception.Message) -ForegroundColor Red; $script:Fail++ }
}
foreach ($fn in @("ConvertFrom-AdbDevices","ConvertFrom-FastbootDevices","ConvertFrom-GetpropDump","ConvertFrom-ByNameListing","ConvertFrom-FastbootGetvar","Get-OSClassification","Test-FirmwareCompatibility","Test-FirmwareUrl","Test-BootImageMagic","Get-PreferredRootMethod")) {
  Import-TTFunction $fn
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
Assert-True "recovery_ramdisk gefunden" (($by | Where-Object { $_.Name -eq "recovery_ramdisk" }).Count -eq 1)
Assert-True "boot gefunden" (($by | Where-Object { $_.Name -eq "boot" }).Count -eq 1)

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

Write-Host ""
Write-Host ("Ergebnis: " + $Pass + " PASS, " + $Fail + " FAIL") -ForegroundColor $(if ($Fail -eq 0) { "Green" } else { "Red" })
if ($Fail -gt 0) { exit 1 }
