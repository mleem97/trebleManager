<#
.SYNOPSIS
  Online installer + launcher for trebleManager (iex-safe, zero files needed).

.DESCRIPTION
  One-liner usage (paste into PowerShell, Enter):
    irm https://raw.githubusercontent.com/mleem97/trebleManager/main/Install-Online.ps1 | iex

  Resolves the latest release (raw VERSION first, GitHub API fallback),
  downloads the full release ZIP (SHA-256 verified), extracts it to
  %LOCALAPPDATA%\trebleManager\<tag> (cache: re-runs reuse it), then
  launches the central starter Start-TrebleToolkit.bat (admin, setup, TUI).

  Never uses $MyInvocation/$PSScriptRoot (both are empty under iex).
  Nothing exits silently: every failure stops with a readable message.

.PARAMETER Gui
  Install + start the gsi-root GUI instead (prebuilt Windows binary).

.PARAMETER Tag
  Pin a version (e.g. -Tag v2.21.0). Default: latest.
#>
param([switch]$Gui, [string]$Tag = "")

$ErrorActionPreference = "Stop"
$Repo = "mleem97/trebleManager"
$Base = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "trebleManager"
if (-not (Test-Path $Base)) { New-Item -ItemType Directory -Path $Base -Force | Out-Null }

function Write-Step([string]$Msg) { Write-Host ("==> " + $Msg) -ForegroundColor Cyan }

try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}

function Get-LatestTag {
  # raw VERSION first (no API rate limit), GitHub API (with UA) as fallback.
  try {
    $wc = New-Object Net.WebClient
    $wc.Headers.Add("User-Agent", "trebleManager")
    $v = ([string]$wc.DownloadString("https://raw.githubusercontent.com/$Repo/main/VERSION")).Trim()
    if ($v -ne "") { return "v" + $v }
  } catch { Write-Host ("WARN: raw VERSION failed (" + $_.Exception.Message + ")") -ForegroundColor Yellow }
  try {
    $wc2 = New-Object Net.WebClient
    $wc2.Headers.Add("User-Agent", "trebleManager")
    $j = $wc2.DownloadString("https://api.github.com/repos/$Repo/releases/latest") | ConvertFrom-Json
    return [string]$j.tag_name
  } catch { throw ("version resolve failed: " + $_.Exception.Message) }
}

function Get-VerifiedFile([string]$Url, [string]$Dest) {
  Write-Step ("download: " + $Url)
  $wc = New-Object Net.WebClient
  $wc.Headers.Add("User-Agent", "trebleManager")
  $wc.DownloadFile($Url, $Dest)
  try {
    $shaUrl = $Url + ".sha256"
    $exp = (([string]$wc.DownloadString($shaUrl) -split '\s+')[0]).Trim().ToUpper()
    $act = (Get-FileHash $Dest -Algorithm SHA256).Hash.ToUpper()
    if ($exp -ne "" -and $exp -ne $act) { Remove-Item $Dest -Force; throw "SHA256 MISMATCH, deleted, aborting" }
    Write-Host "SHA256 OK." -ForegroundColor Green
  } catch {
    if ($_.Exception.Message -match "MISMATCH") { throw }
    Write-Host ("WARN: hash check skipped (" + $_.Exception.Message + ")") -ForegroundColor Yellow
  }
  return $Dest
}

if ([string]::IsNullOrWhiteSpace($Tag)) { $Tag = Get-LatestTag }
if ([string]::IsNullOrWhiteSpace($Tag)) { throw "empty version response" }
Write-Step ("version: " + $Tag)

if ($Gui) {
  $arch = (Get-CimInstance Win32_Processor | Select-Object -First 1).Architecture
  # 9 = x64, 12 = ARM64 (WMI values)
  if ($arch -ne 9) { throw ("no prebuilt GUI binary for this CPU (arch=" + $arch + "). Fallback: run without -Gui (TUI).") }
  $gbinDir = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "gsi-root\bin"
  $gbin = Join-Path $gbinDir "gsi-root.exe"
  if (-not (Test-Path $gbin)) {
    $gzName = "gsi-root-windows-x86_64.zip"
    $gzUrl = "https://github.com/$Repo/releases/download/$Tag/$gzName"
    $gz = Join-Path $Base $gzName
    Get-VerifiedFile $gzUrl $gz | Out-Null
    if (-not (Test-Path $gbinDir)) { New-Item -ItemType Directory -Path $gbinDir -Force | Out-Null }
    Write-Step ("installing to " + $gbinDir)
    Expand-Archive -Path $gz -DestinationPath $gbinDir -Force
    if (-not (Test-Path $gbin)) { throw "gsi-root.exe missing after install" }
    $p = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($p -split ";" ) -notcontains $gbinDir) {
      [Environment]::SetEnvironmentVariable("Path", ($p + ";" + $gbinDir), "User")
      Write-Host "PATH updated (new terminals)." -ForegroundColor Green
    } else { Write-Host "PATH already set." -ForegroundColor Gray }
  } else { Write-Host ("already installed: " + $gbin) -ForegroundColor Gray }
  Write-Step ("launching GUI: " + $gbin)
  Start-Process -FilePath $gbin
  return
}

$dest = Join-Path $Base $Tag
$starter = Join-Path $dest "Start-TrebleToolkit.bat"
if (-not (Test-Path $starter)) {
  $zipName = "trebleManager-$Tag.zip"
  $zipUrl = "https://github.com/$Repo/releases/download/$Tag/$zipName"
  $zip = Join-Path $Base $zipName
  Get-VerifiedFile $zipUrl $zip | Out-Null
  if (-not (Test-Path $dest)) { New-Item -ItemType Directory -Path $dest -Force | Out-Null }
  Write-Step ("extracting to " + $dest)
  Expand-Archive -Path $zip -DestinationPath $dest -Force
}
if (-not (Test-Path $starter)) { throw ("central starter missing in package: " + $starter) }
Write-Step ("launching central starter (admin, setup, TUI): " + $starter)
& $starter
