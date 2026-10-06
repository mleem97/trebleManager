#!/usr/bin/env bash
# trebleManager - Linux bash port (Windows PowerShell TUI parity)
# Huawei P10 (VTR-L29 and others): detect, analyze, firmware, download,
# extract, export, patch, backup, flash, verify, restore, diagnostic.
# No extra dependencies: only adb, fastboot, and coreutils. Optional:
# unzip or python3 (zip listing), curl or wget (download), zip or python3 (diagnostic).
# Repo language: English. TUI German if $LANG starts with de.
set -u

TTVERSION="2.6.0"
# Run modes: safe (confirm everything), unattended (--yes auto-confirms, gates
# still enforced), developer (unlocks dump-* commands).
RUNMODE_REQ=""
resolve_mode() { # pure: safe|unattended|developer -> itself, else safe
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in unattended|developer) printf '%s' "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" ;; *) printf 'safe' ;; esac
}
TTLANG="en"
case "${LANG:-en}" in de*) TTLANG="de" ;; esac

# ---------------------------------------------------------------- paths/state
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOL_ROOT="$(dirname "$SCRIPT_DIR")"
if [ "$(basename "$TOOL_ROOT")" = "scripts" ]; then TOOL_ROOT="$(dirname "$TOOL_ROOT")"; fi
# Remote-run fallback: current directory layout
if [ ! -d "$TOOL_ROOT/data" ]; then TOOL_ROOT="$(pwd)"; fi
LOG_DIR="$TOOL_ROOT/logs"
DATA_DIR="$TOOL_ROOT/data"
FIRM_DIR="$DATA_DIR/firmware"
MAG_DIR="$DATA_DIR/magisk"
TOOL_DIR="$DATA_DIR/tools"
ROM_DIR="$DATA_DIR/roms"
REC_DIR="$DATA_DIR/recovery"
BACK_DIR="$TOOL_ROOT/backups"
mkdir -p "$LOG_DIR" "$FIRM_DIR" "$MAG_DIR" "$TOOL_DIR" "$ROM_DIR" "$REC_DIR" "$BACK_DIR" 2>/dev/null
STAMP="$(date +%Y%m%d-%H%M%S)"
TTLOG="$LOG_DIR/toolkit-$STAMP.log"

PROFILE_ID="VTR-L29"
MODE="none"
ADB_SERIAL=""
FB_SERIAL=""
ADB_BIN=""
FB_BIN=""
OS_KIND="?"
OS_DETAIL=""
FW_BASELINE=""
FW_STATUS=""
STOCK_IMAGE=""
STOCK_SHA=""
PATCHED_IMAGE=""
PATCHED_SHA=""
BACKUP_DIR=""

# ---------------------------------------------------------------- i18n + log
L() { if [ "$TTLANG" = "de" ]; then printf '%s' "$2"; else printf '%s' "$1"; fi; }
C_RED='\033[0;31m'; C_YEL='\033[0;33m'; C_GRN='\033[0;32m'; C_GRY='\033[0;37m'; C_CYN='\033[0;36m'; C_RST='\033[0m'
log() { # level message
  local ts; ts="$(date '+%Y-%m-%d %H:%M:%S')"
  local line="$ts $1 $2"
  case "$1" in
    ERROR) printf '%b\n' "${C_RED}${line}${C_RST}" ;;
    WARNING) printf '%b\n' "${C_YEL}${line}${C_RST}" ;;
    SUCCESS) printf '%b\n' "${C_GRN}${line}${C_RST}" ;;
    *) printf '%b\n' "${C_GRY}${line}${C_RST}" ;;
  esac
  printf '%s\n' "$line" >> "$TTLOG" 2>/dev/null
}

# ---------------------------------------------------------------- profiles (mac-bash3 safe: case, no assoc arrays)
# Verified = proven via Discussion #2542 + P10 wiki (flash allowed).
# Unverified = hypothesis only (analyze + export allowed, flash BLOCKED).
# LineageOS is a valid STARTING point (hides Huawei base -> assisted baseline),
# but the Magisk SOURCE stays the stock RECOVERY_RAMDISK.img.
target_partition() {
  case "$PROFILE_ID" in VTR-*|VKY-*) printf 'recovery_ramdisk' ;; *) printf '' ;; esac
}
marketing_name() {
  case "$PROFILE_ID" in VKY-*) printf 'Huawei P10 Plus' ;; GENERIC-TREBLE) printf 'Generic Treble device' ;; *) printf 'Huawei P10' ;; esac
}
profile_verified() {
  case "$PROFILE_ID" in VTR-L29|VTR-L09|VKY-L29) printf '1' ;; *) printf '0' ;; esac
}
profile_variant() {
  case "$PROFILE_ID" in
    VTR-L29) printf 'Global market (UFS storage)' ;;
    VTR-L09) printf 'Europe (UFS storage)' ;;
    VKY-L29) printf 'Global market Plus (UFS storage)' ;;
    VTR-AL00) printf 'China, no SIM restriction (eMMC or UFS - check!)' ;;
    VTR-TL00) printf 'China Mobile customized (eMMC or UFS - check!)' ;;
    VKY-L09) printf 'Europe Plus (UFS storage)' ;;
    VKY-AL00) printf 'China Plus, no SIM restriction (eMMC or UFS - check!)' ;;
    VKY-TL00) printf 'China Mobile Plus customized (eMMC or UFS - check!)' ;;
    *) printf 'Fallback (analyze only)' ;;
  esac
}
profile_gsi_advice() {
  case "$PROFILE_ID" in
    VTR-AL00) printf 'arm64 A-only; CN units with eMMC behave differently - see wiki storage note' ;;
    GENERIC-TREBLE) printf 'No verified method - analysis and recovery export only' ;;
    VTR-*|VKY-*) printf 'arm64 A-only images; slim builds if system partition is small' ;;
    *) printf '' ;;
  esac
}

# Wiki knowledge (P10 page, short form; full text in TUI screens).
WIKI_ANDROID13_WARN="Android 13 on P10/P10 Plus is unstable per wiki: no SIM/signal possible, hardware may fail. Android 10 (Q) GSIs are the recommended daily drivers on Kirin 960."

# ---------------------------------------------------------------- tools (with timeouts + tty reads)
# adb_run/fb_run guard every device call with `timeout` (e.g. su prompts must
# never hang the TUI). iread reads the terminal so `curl | bash` stays
# interactive with zero downloaded files.
USE_TMO=0; command -v timeout >/dev/null 2>&1 && USE_TMO=1
adb_run() { if [ "$USE_TMO" = 1 ]; then timeout 30 "$ADB_BIN" "$@"; else "$ADB_BIN" "$@"; fi; }
fb_run() { if [ "$USE_TMO" = 1 ]; then timeout 120 "$FB_BIN" "$@"; else "$FB_BIN" "$@"; fi; }
fb_flash() { if [ "$USE_TMO" = 1 ]; then timeout 600 "$FB_BIN" "$@"; else "$FB_BIN" "$@"; fi; }
iread() { if [ -c /dev/tty ] 2>/dev/null; then builtin read "$@" </dev/tty; else builtin read "$@"; fi; }
find_tools() {
  # Saved setup config first (data/config.json)
  if [ -f "$TOOL_ROOT/data/config.json" ]; then
    cfg_adb="$(sed -n 's/.*"adb"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$TOOL_ROOT/data/config.json" | head -1)"
    cfg_fb="$(sed -n 's/.*"fastboot"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$TOOL_ROOT/data/config.json" | head -1)"
    [ -n "$cfg_adb" ] && [ -x "$cfg_adb" ] && ADB_BIN="$cfg_adb"
    [ -n "$cfg_fb" ] && [ -x "$cfg_fb" ] && FB_BIN="$cfg_fb"
  fi
  [ -z "$ADB_BIN" ] && ADB_BIN="$(command -v adb 2>/dev/null || true)"
  [ -z "$FB_BIN" ] && FB_BIN="$(command -v fastboot 2>/dev/null || true)"
  if [ -n "$ADB_BIN" ]; then log SUCCESS "ADB: $ADB_BIN"; else log ERROR "$(L 'ADB not found (install android-tools / platform-tools).' 'ADB nicht gefunden (android-tools / platform-tools installieren).')"; fi
  if [ -n "$FB_BIN" ]; then log SUCCESS "Fastboot: $FB_BIN"; else log WARNING "$(L 'Fastboot not found.' 'Fastboot nicht gefunden.')"; fi
  # scrcpy is optional (screen mirror during rooting), never required
  SCRCPY_BIN="$(command -v scrcpy 2>/dev/null || true)"
  if [ -n "$SCRCPY_BIN" ]; then
    log SUCCESS "scrcpy: $SCRCPY_BIN ($("$SCRCPY_BIN" --version 2>&1 | head -1))"
  else
    log INFO "$(L 'scrcpy not found (optional, screen mirror only: https://github.com/Genymobile/scrcpy).' 'scrcpy nicht gefunden (optional, nur Screen-Mirror: https://github.com/Genymobile/scrcpy).')"
  fi
}
adb_prop() { # name -> value (single line)
  [ -n "$ADB_BIN" ] || return 0
  adb_run shell getprop "$1" 2>/dev/null | tr -d '\r\n'
}

# ---------------------------------------------------------------- mode + analysis
detect_mode() {
  MODE="none"; ADB_SERIAL=""; FB_SERIAL=""
  if [ -n "$ADB_BIN" ]; then
    local dev
    dev="$("$ADB_BIN" devices 2>/dev/null | awk 'NR>1 && $2=="device" {print $1; exit}')"
    if [ -n "$dev" ]; then MODE="android"; ADB_SERIAL="$dev"; return 0; fi
    if "$ADB_BIN" devices 2>/dev/null | grep -q unauthorized; then
      log WARNING "$(L 'ADB authorization pending (confirm dialog on device).' 'ADB-Autorisierung ausstehend (Dialog am Geraet bestaetigen).')"
    fi
  fi
  if [ "$MODE" = "none" ] && [ -n "$FB_BIN" ]; then
    local fb
    fb="$("$FB_BIN" devices 2>/dev/null | awk '$2=="fastboot" {print $1; exit}')"
    if [ -n "$fb" ]; then MODE="fastboot"; FB_SERIAL="$fb"; fi
  fi
}

PROPS_FILE="$LOG_DIR/.props-$STAMP.tmp"
BYNAME_RAW=""
CMDLINE=""
WHICHSU=""
MAGISKV=""
OS_RELEASE=""

os_classify() { # sets OS_KIND (pure logic on globals MODEL PNAME DISPLAY RELEASE EMUI)
  local blob
  blob="$(printf '%s %s %s' "$MODEL" "$PNAME" "$DISPLAY" | tr '[:upper:]' '[:lower:]')"
  local is_gsi=0 is_stock=0
  case "$blob" in *lineage_*|*trebledroid*|*treble*|*phh*|*aosp*|*pixel*|*superior*|*havoc*|*crdroid*|*gsi*|*arm64_b*) is_gsi=1 ;; esac
  if [ -n "$EMUI" ] || [[ "$DISPLAY" == *EMUI* ]] || [[ "$DISPLAY" =~ VTR-.*C[0-9] ]]; then is_stock=1; fi
  if [ "$is_gsi" = 1 ]; then is_stock=0; fi
  if [[ "$blob" == *trebledroid* ]]; then OS_KIND="TrebleDroid-GSI"
  elif [[ "$blob" == *lineage_* ]]; then OS_KIND="Lineage-GSI"
  elif [[ "$blob" == *pixel* ]]; then OS_KIND="PixelExperience-GSI"
  elif [[ "$blob" == *superior* ]]; then OS_KIND="SuperiorOS-GSI"
  elif [ "$is_gsi" = 1 ]; then OS_KIND="AOSP/GSI-custom"
  elif [ "$is_stock" = 1 ] && [[ "$EMUI" == *9.1* ]]; then OS_KIND="Stock-EMUI-9.1"
  elif [ "$is_stock" = 1 ] && [[ "$EMUI" == *9.0* ]]; then OS_KIND="Stock-EMUI-9.0"
  elif [ "$is_stock" = 1 ] && [[ "$EMUI" == *8.* ]]; then OS_KIND="Stock-EMUI-8"
  elif [ "$is_stock" = 1 ]; then OS_KIND="Stock-EMUI/Harmony-base"
  else OS_KIND="Unknown"; fi
}

android_analysis() {
  log INFO "$(L 'Android analysis (read-only, OS-independent) ...' 'Android-Analyse (read-only, OS-unabhaengig) ...')"
  : > "$PROPS_FILE"
  for p in ro.product.model ro.product.name ro.product.device ro.build.version.release ro.build.display.id ro.build.type ro.product.cpu.abi ro.hardware ro.treble.enabled ro.vndk.version ro.boot.slot_suffix ro.boot.verifiedbootstate ro.boot.flash.locked ro.boot.vbmeta.device_state ro.secure ro.debuggable ro.build.version.emui ro.emui.version ro.lineage.version ro.lineageos.version; do
    printf '%s=%s\n' "$p" "$(adb_prop "$p")" >> "$PROPS_FILE"
  done
  MODEL="$(grep '^ro.product.model=' "$PROPS_FILE" | cut -d= -f2-)"
  PNAME="$(grep '^ro.product.name=' "$PROPS_FILE" | cut -d= -f2-)"
  DISPLAY="$(grep '^ro.build.display.id=' "$PROPS_FILE" | cut -d= -f2-)"
  OS_RELEASE="$(grep '^ro.build.version.release=' "$PROPS_FILE" | cut -d= -f2-)"
  EMUI="$(grep '^ro.build.version.emui=' "$PROPS_FILE" | cut -d= -f2-)"
  [ -z "$EMUI" ] && EMUI="$(grep '^ro.emui.version=' "$PROPS_FILE" | cut -d= -f2-)"
  OS_DETAIL="$MODEL / $PNAME / Android $OS_RELEASE ($DISPLAY)"
  os_classify
  BYNAME_RAW="$(adb_run shell ls -l /dev/block/by-name/ 2>&1)"
  CMDLINE="$(adb_run shell cat /proc/cmdline 2>&1 | tr -d '\r')"
  WHICHSU="$(adb_run shell 'which su; ls -l /system/xbin/su 2>&1; ls -l /system/bin/su 2>&1' 2>&1)"
  MAGISKV="$(adb_run shell 'magisk -v 2>&1; su -c id 2>&1' 2>&1)"
  # Storage detection (eMMC vs UFS): informational, never assumed.
  STORAGE="unknown"
  blk="$(adb_run shell ls /sys/block/ 2>&1)"
  if printf '%s' "$blk" | grep -q -E '(^|[[:space:]])sd[a-z]([[:space:]]|$)'; then STORAGE="UFS (sd* present)"
  elif printf '%s' "$blk" | grep -q mmcblk; then STORAGE="eMMC (mmcblk, no sd*)"; fi
  case "$MODEL" in *VTR-L29*) PROFILE_ID="VTR-L29" ;; *VTR-L09*) PROFILE_ID="VTR-L09" ;; *VKY-L29*) PROFILE_ID="VKY-L29" ;; *VTR-AL00*) PROFILE_ID="VTR-AL00" ;; *VTR-TL00*) PROFILE_ID="VTR-TL00" ;; *VKY-L09*) PROFILE_ID="VKY-L09" ;; *VKY-AL00*) PROFILE_ID="VKY-AL00" ;; *VKY-TL00*) PROFILE_ID="VKY-TL00" ;;
    *) PROFILE_ID="VTR-L29"; log WARNING "$(L "Model string is GSI ('$MODEL'). Profile default VTR-L29, verification before flash mandatory." "Modellstring ist GSI. Profil-Default VTR-L29, Verifikation vor Flash Pflicht.")" ;; esac
  log SUCCESS "OS: $OS_KIND | $OS_DETAIL"
}

FBRAW_FILE="$LOG_DIR/.fbraw-$STAMP.tmp"
fastboot_analysis() {
  # Never probe getvar without a fastboot device (fastboot would wait forever).
  if [ "$MODE" != "fastboot" ]; then
    if ! "$FB_BIN" devices 2>/dev/null | grep -q fastboot; then
      log WARNING "$(L 'Not in fastboot mode, skipping getvar probes (would wait forever).' 'Nicht im Fastboot-Modus, getvar-Abfragen uebersprungen (wuerden ewig warten).')"
      return 0
    fi
    MODE="fastboot"
  fi
  log INFO "$(L 'Fastboot analysis (read-only) ...' 'Fastboot-Analyse (read-only) ...')"
  : > "$FBRAW_FILE"
  for v in product secure unlocked current-slot partition-type:recovery_ramdisk partition-size:recovery_ramdisk partition-type:boot partition-size:boot partition-type:recovery partition-size:recovery partition-type:system partition-size:system partition-type:vendor partition-size:vendor; do
    { printf '### getvar %s\n' "$v"; fb_run getvar "$v" 2>&1; } >> "$FBRAW_FILE"
  done
  { printf '### getvar all\n'; fb_run getvar all 2>&1; } >> "$FBRAW_FILE"
  if grep -q "Command not allowed" "$FBRAW_FILE"; then
    log WARNING "$(L 'Huawei refuses getvar (Command not allowed). NOT proof of lock.' 'Huawei verweigert getvar (Command not allowed). KEIN Lock-Beweis.')"
  fi
}

# ---------------------------------------------------------------- firmware
firmware_compat() { # model baseline region -> prints STATUS + sets FW_STATUS
  local model="$1" base="$2" region="$3"
  local fw status="PASS" reasons=""
  fw="$(printf '%s' "$base" | tr '[:lower:]' '[:upper:]')"
  local mo; mo="$(printf '%s' "$model" | tr '[:lower:]' '[:upper:]')"
  if [ -z "$base" ]; then FW_STATUS="FAIL"; printf 'FAIL|No firmware given (baseline missing).\n'; return; fi
  case "$mo" in
    VTR*) case "$fw" in *VTR*) ;; *) status="FAIL"; reasons="${reasons}Firmware has no VTR (P10), device is $model. " ;; esac ;;
    VKY*) case "$fw" in *VKY*) ;; *) status="FAIL"; reasons="${reasons}Firmware has no VKY (P10 Plus), device is $model. " ;; esac ;;
  esac
  case "$mo:$fw" in
    "VTR-L29:"*VTR-L09*) [ "$status" = PASS ] && status="WARN"; reasons="${reasons}Submodel mismatch L29 vs L09 -> check CUST/region. " ;;
    "VTR-L09:"*VTR-L29*) [ "$status" = PASS ] && status="WARN"; reasons="${reasons}Submodel mismatch L09 vs L29 -> check CUST/region. " ;;
  esac
  local fwcust=""; fwcust="$(printf '%s' "$fw" | grep -o '(C[0-9]*' | head -1 | tr -d '(')"
  if [ -n "$region" ] && [ -n "$fwcust" ] && [ "$(printf '%s' "$region" | tr '[:lower:]' '[:upper:]')" != "$fwcust" ]; then
    [ "$status" = PASS ] && status="WARN"; reasons="${reasons}Region: device $region vs firmware $fwcust. "
  fi
  case "$fw" in
    *9.1.0*) reasons="${reasons}EMUI 9.1 base ok. " ;;
    *9.0*) [ "$status" = PASS ] && status="WARN"; reasons="${reasons}EMUI 9.0 instead of 9.1. " ;;
    *8.*) [ "$status" = PASS ] && status="WARN"; reasons="${reasons}EMUI 8 base, different boot chain possible. " ;;
    *) [ "$status" = PASS ] && status="WARN"; reasons="${reasons}EMUI version not recognizable. " ;;
  esac
  [ -z "$reasons" ] && reasons="Base check passed. "
  FW_STATUS="$status"
  printf '%s|%s\n' "$status" "$reasons"
}

# ---------------------------------------------------------------- files: hash/magic/validate
file_hash() { # path -> prints "sha256 size" or fails
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'; stat -c%s "$1" 2>/dev/null || stat -f%z "$1"
  else
    shasum -a 256 "$1" | awk '{print $1}'; stat -f%z "$1" 2>/dev/null || stat -c%s "$1"
  fi
}
boot_magic_ver() { # path -> version int or -1 (ANDROID! magic)
  [ -f "$1" ] || { printf -- '-1'; return; }
  local magic
  magic="$(head -c 8 "$1" 2>/dev/null)"
  if [ "$magic" = "ANDROID!" ]; then
    od -An -tu1 -j8 -N1 "$1" 2>/dev/null | tr -d ' \n'
  else printf -- '-1'; fi
}
test_image() { # path -> PASS|FAIL + sets IMG_SHA IMG_SIZE
  IMG_SHA=""; IMG_SIZE=0
  [ -f "$1" ] || { printf 'FAIL|missing'; return; }
  local out; out="$(file_hash "$1")"
  IMG_SHA="$(printf '%s' "$out" | head -1)"; IMG_SIZE="$(printf '%s' "$out" | tail -1)"
  if [ "$IMG_SIZE" -ge 1048576 ] && [ "$IMG_SIZE" -le 104857600 ]; then printf 'PASS|%s|%s' "$IMG_SHA" "$IMG_SIZE"; else printf 'FAIL|%s|%s' "$IMG_SHA" "$IMG_SIZE"; fi
}
valid_url() { # url -> 0/1 (http(s) + archive ext, max 2048)
  local u="$1"
  [ -n "$u" ] || return 1
  [ "${#u}" -le 2048 ] || return 1
  case "$u" in http://*|https://*) ;; *) return 1 ;; esac
  local low; low="$(printf '%s' "$u" | cut -d'?' -f1 | tr '[:upper:]' '[:lower:]')"
  case "$low" in *.zip|*.7z|*.tar|*.gz|*.tgz|*.app|*.rar) return 0 ;; *) return 1 ;; esac
}

# ---------------------------------------------------------------- download with progress + confirm
download_firmware() { # url outfile [--yes]
  local url="$1" out="$2" yes="${3:-}"
  valid_url "$url" || { log ERROR "$(L 'URL rejected (http(s) archive only).' 'URL abgelehnt (nur http(s)-Archiv).')"; return 1; }
  mkdir -p "$(dirname "$out")"
  if [ -f "$out" ]; then log WARNING "$(L 'Target exists (offline cache, no re-download): ' 'Ziel existiert (Cache, kein Re-Download): ')$out"; printf '%s' "$out"; return 0; fi
  if [ -z "$yes" ]; then
    log INFO "$(L 'Ready (not loaded). Re-run with --yes to download (2-4 GB).' 'Bereit (nicht geladen). Mit --yes herunterladen (2-4 GB).')"
    return 4
  fi
  log INFO "$(L 'Download starting: ' 'Download startet: ')$url"
  if command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$out" "$url" || { rm -f "$out"; log ERROR "$(L 'Download failed.' 'Download fehlgeschlagen.')"; return 1; }
  elif command -v wget >/dev/null 2>&1; then
    wget --show-progress -O "$out" "$url" || { rm -f "$out"; log ERROR "$(L 'Download failed.' 'Download fehlgeschlagen.')"; return 1; }
  else log ERROR "$(L 'Neither curl nor wget found.' 'Weder curl noch wget gefunden.')"; return 1; fi
  local mb; mb=$(($(stat -c%s "$out" 2>/dev/null || stat -f%z "$out") / 1048576))
  log SUCCESS "$(L 'Download done (' 'Download fertig (')${mb} MB)."
  printf '%s' "$out"
}
verify_download() { # path -> 0/1, writes .sha256 sidecar
  local p="$1" size sha
  size="$(stat -c%s "$p" 2>/dev/null || stat -f%z "$p")"
  if command -v sha256sum >/dev/null 2>&1; then sha="$(sha256sum "$p" | awk '{print $1}')"; else sha="$(shasum -a 256 "$p" | awk '{print $1}')"; fi
  printf '%s' "$sha" > "$p.sha256"
  if [ "$size" -lt 104857600 ]; then log WARNING "$(L 'WARN: < 100 MB, implausibly small for full firmware.' 'WARN: < 100 MB, unplausibel klein. Site: ')"; return 3; fi
  log SUCCESS "$(L 'Firmware ready: ' 'Firmware bereit: ')$p"
  return 0
}

# ---------------------------------------------------------------- recovery export from custom ROMs
zip_entries() { # zip -> entry list
  if command -v unzip >/dev/null 2>&1; then unzip -l "$1" 2>/dev/null | awk 'NR>3 && $4 != "" && $4 != "----" {print $4}';
  elif command -v python3 >/dev/null 2>&1; then python3 -c "import zipfile,sys; [print(i.filename) for i in zipfile.ZipFile(sys.argv[1]).infolist()]" "$1" 2>/dev/null; fi
}
export_recovery() { # rompath -> prints dir; sets EXPORT_FILES
  local rom="$1" base dir low
  [ -f "$rom" ] || { log ERROR "$(L 'ROM file missing: ' 'ROM-Datei fehlt: ')$rom"; return 3; }
  base="$(basename "$rom" | sed 's/\.[^.]*$//')"
  dir="$REC_DIR/${base}-$STAMP"
  mkdir -p "$dir"
  EXPORT_FILES=""
  low="$(printf '%s' "$rom" | tr '[:upper:]' '[:lower:]')"
  case "$low" in
    *.img)
      local v; v="$(boot_magic_ver "$rom")"
      if [ "$v" != "-1" ] && [ -n "$v" ]; then
        cp "$rom" "$dir/"
        (if command -v sha256sum >/dev/null 2>&1; then sha256sum "$dir/$(basename "$rom")" | awk '{print $1}'; else shasum -a 256 "$dir/$(basename "$rom")" | awk '{print $1}'; fi) > "$dir/sha256.txt"
        printf '{"source": "%s", "kind": "direct-img", "bootimg_version": %s}\n' "$rom" "$v" > "$dir/metadata.json"
        EXPORT_FILES="$dir/$(basename "$rom")"
        log SUCCESS "$(L 'Direct boot image exported.' 'Direktes Boot-Image exportiert.')"
        printf '%s' "$dir"; return 0
      fi
      log ERROR "$(L 'Not an Android boot image (no ANDROID! magic). GSI system images contain no recovery.' 'Kein Android-Boot-Image (kein ANDROID!-Magic). GSI hat kein Recovery.')"
      return 3 ;;
    *.zip)
      local entries cands e dst
      entries="$(zip_entries "$rom")"
      cands="$(printf '%s' "$entries" | grep -i -E 'recovery.*\.img$|(^|/)boot\.img$' || true)"
      if [ -n "$cands" ]; then
        if command -v unzip >/dev/null 2>&1; then
          while IFS= read -r e; do
            [ -n "$e" ] || continue
            dst="$dir/$(basename "$e")"
            unzip -p "$rom" "$e" > "$dst" 2>/dev/null
            if [ "$(boot_magic_ver "$dst")" != "-1" ]; then EXPORT_FILES="$EXPORT_FILES $dst"; else log WARNING "$(L 'Extracted but no boot magic: ' 'Extrahiert aber kein Boot-Magic: ')$e"; rm -f "$dst"; fi
          done <<EOF
$cands
EOF
        elif command -v python3 >/dev/null 2>&1; then
          python3 - "$rom" "$dir" <<'EOF' || true
import zipfile,sys,re,os
z=zipfile.ZipFile(sys.argv[1]); d=sys.argv[2]
for i in z.infolist():
    n=i.filename
    if re.search(r'recovery.*\.img$',n,re.I) or re.match(r'(.*/)?boot\.img$',n,re.I):
        z.extract(i,d)
        print(os.path.join(d,n))
EOF
          while IFS= read -r f; do
            [ -n "$f" ] || continue
            if [ "$(boot_magic_ver "$f")" != "-1" ]; then EXPORT_FILES="$EXPORT_FILES $f"; else rm -f "$f"; fi
          done <<EOF2
$(python3 -c "pass" 2>/dev/null; true)
EOF2
        else log ERROR "$(L 'Neither unzip nor python3 available.' 'Weder unzip noch python3 verfuegbar.')"; return 1; fi
        if [ -n "$EXPORT_FILES" ]; then
          printf '{"source": "%s", "kind": "rom-zip"}\n' "$rom" > "$dir/metadata.json"
          log SUCCESS "$(L 'Exported from ROM zip.' 'Aus ROM-ZIP exportiert.')"
          printf '%s' "$dir"; return 0
        fi
      fi
      if printf '%s' "$entries" | grep -qi 'payload.bin'; then
        local dumper
        dumper="$(find "$TOOL_DIR" \( -iname '*payload*dumper*' -o -name 'payload-dumper-go' \) -type f 2>/dev/null | head -1)"
        if [ -n "$dumper" ]; then
          "$dumper" -o "$dir" -p "boot,recovery" "$rom" > "$dir/dumper.log" 2>&1 || true
          for o in "$dir"/*.img; do
            [ -f "$o" ] || continue
            if [ "$(boot_magic_ver "$o")" != "-1" ]; then EXPORT_FILES="$EXPORT_FILES $o"; fi
          done
          if [ -n "$EXPORT_FILES" ]; then printf '%s' "$dir"; return 0; fi
          log ERROR "$(L 'Dumper ran but no valid boot images found.' 'Dumper lief, keine gueltigen Images.')"
          return 3
        fi
        log ERROR "$(L 'payload.bin ROM: place payload-dumper-go in data/tools/ or extract boot.img manually.' 'payload.bin-ROM: payload-dumper-go nach data/tools/ legen oder boot.img manuell extrahieren.')"
        return 3
      fi
      log ERROR "$(L 'No boot.img/recovery.img/payload.bin in ZIP. Probably a GSI system package (no recovery by design).' 'Kein boot.img/recovery.img/payload.bin im ZIP. Wahrscheinlich GSI-System-Paket (kein Recovery).')"
      return 3 ;;
    *) log ERROR "$(L 'Unsupported format (use .img or ROM .zip).' 'Format nicht unterstuetzt (.img oder ROM-.zip).')"; return 3 ;;
  esac
}

# ---------------------------------------------------------------- magisk prep (real on-device patch, never fake)
magisk_apk() { find "$MAG_DIR" -maxdepth 1 -iname '*.apk' -type f 2>/dev/null | head -1; }
prepare_patch() { # stock_image -> staged path
  local stock="$1" stage dest guide
  stage="$MAG_DIR/to-patch"; mkdir -p "$stage"
  dest="$stage/$(basename "$stock")"
  cp -f "$stock" "$dest"
  guide="$stage/PATCH-INSTRUCTIONS.txt"
  {
    printf 'Magisk patch (real, never copy-and-claim)\n==============================================\n\n'
    printf '1. Install Magisk APK from official source:\n   https://github.com/topjohnwu/Magisk/releases\n   APK ideally in: data/magisk/\n\n'
    printf '2. Copy this file to the device:\n   %s\n   e.g.: adb push "%s" /sdcard/Download/\n\n' "$dest" "$dest"
    printf '3. On device (any OS, stock or GSI/custom):\n   Open Magisk -> Install -> Select and Patch a File\n   -> select RECOVERY_RAMDIS(K).img (exactly this file, NOT boot.img/recovery.img)\n\n'
    printf '4. Result on device: /sdcard/Download/magisk_patched-*.img\n   Copy back: adb pull /sdcard/Download/magisk_patched-XXXX.img data/magisk/\n\n'
    printf '5. Register the patched file here and let it validate (hash != stock required).\n'
  } > "$guide"
  log SUCCESS "$(L 'Patch preparation: ' 'Patch-Vorbereitung: ')$dest"
  printf '%s' "$dest"
}

# ---------------------------------------------------------------- backup / readiness / flash / verify / restore
do_backup() { # stock_image -> dir or fail
  local stock="$1" model="$PROFILE_ID" part stamp dir
  part="$(target_partition)"
  stamp="$(date +%Y%m%d-%H%M%S)"
  dir="$BACK_DIR/$model/$part/$stamp"
  mkdir -p "$dir"
  if [ -f "$stock" ]; then cp -f "$stock" "$dir/original.img"
  else log ERROR "$(L 'Backup NOT possible (no original). Nothing is faked.' 'Backup NICHT moeglich (kein Original). Nichts vorgetaeuscht.')"; return 3; fi
  local sha size
  if command -v sha256sum >/dev/null 2>&1; then sha="$(sha256sum "$dir/original.img" | awk '{print $1}')"; else sha="$(shasum -a 256 "$dir/original.img" | awk '{print $1}')"; fi
  size="$(stat -c%s "$dir/original.img" 2>/dev/null || stat -f%z "$dir/original.img")"
  printf '%s' "$sha" > "$dir/sha256.txt"
  printf '{"device": "%s", "model": "%s", "partition": "%s", "firmware": "%s", "os_kind": "%s", "size": %s, "sha256": "%s", "timestamp": "%s", "source": "firmware-extracted", "tool": "trebleManager %s"}\n' \
    "$(marketing_name)" "$model" "$part" "$FW_BASELINE" "$OS_KIND" "$size" "$sha" "$(date '+%Y-%m-%d %H:%M:%S')" "$TTVERSION" > "$dir/metadata.json"
  BACKUP_DIR="$dir"
  log SUCCESS "Backup: $dir"
  printf '%s' "$dir"
}
check_readiness() { # patched_image -> 0 = GO else 1 (prints table)
  local img="$1" part fail=0
  part="$(target_partition)"
  local p_ok=0 h_ok=0 s_ok=0 fw_ok=0 b_ok=0 fb_ok=0 d_ok=0 m_ok=0 v_ok=0
  case "$PROFILE_ID" in VTR-*|VKY-*) m_ok=1 ;; esac
  [ "$(profile_verified)" = "1" ] && v_ok=1
  if [ "$MODE" = "android" ] && printf '%s' "$BYNAME_RAW" | grep -q "$part"; then p_ok=1; fi
  if [ "$p_ok" = 0 ] && [ -f "$FBRAW_FILE" ] && grep -q "partition-size:$part" "$FBRAW_FILE" && ! grep -q "Command not allowed" "$FBRAW_FILE"; then p_ok=1; fi
  [ -f "$img" ] && h_ok=1
  if [ "$h_ok" = 1 ]; then
    local sz; sz="$(stat -c%s "$img" 2>/dev/null || stat -f%z "$img")"
    if [ "$sz" -ge 1048576 ] && [ "$sz" -le 104857600 ]; then s_ok=1; fi
    if [ -n "$STOCK_SHA" ]; then
      local ph; if command -v sha256sum >/dev/null 2>&1; then ph="$(sha256sum "$img" | awk '{print $1}')"; else ph="$(shasum -a 256 "$img" | awk '{print $1}')"; fi
      [ "$ph" != "$STOCK_SHA" ] && d_ok=1
    fi
  fi
  [ -n "$FW_BASELINE" ] && [ "$FW_STATUS" != "FAIL" ] && fw_ok=1
  [ -n "$BACKUP_DIR" ] && [ -f "$BACKUP_DIR/original.img" ] && b_ok=1
  if [ -n "$FB_BIN" ] && "$FB_BIN" devices 2>/dev/null | grep -q fastboot; then fb_ok=1; MODE="fastboot"; fi
  for row in "Model matches|$m_ok" "Profile verified (flash allowed)|$v_ok" "Partition exists ($part)|$p_ok" "Image exists|$h_ok" "Image size plausible|$s_ok" "Firmware compat|$fw_ok" "Backup available|$b_ok" "Fastboot connected|$fb_ok" "Patched != stock|$d_ok"; do
    local n="${row%%|*}" v="${row##*|}"
    if [ "$v" = 1 ]; then printf ' [OK]   %s\n' "$n"; else printf ' [FAIL] %s\n' "$n"; fail=1; fi
  done
  return "$fail"
}
safe_flash() { # patched_image [--yes]
  local img="$1" yes="${2:-}" part
  part="$(target_partition)"
  printf '%s\n' "$(L '=== Safety check before flash ===' '=== Sicherheitspruefung vor Flash ===')"
  if ! check_readiness "$img"; then
    printf '%b\n' "${C_RED}$(L 'DO NOT FLASH - at least one check failed.' 'DO NOT FLASH - mindestens eine Pruefung ist fehlgeschlagen.')${C_RST}"
    log ERROR "$(L 'Flash blocked (safety gate).' 'Flash blockiert (Safety-Gate).')"
    return 1
  fi
  local sha; if command -v sha256sum >/dev/null 2>&1; then sha="$(sha256sum "$img" | awk '{print $1}')"; else sha="$(shasum -a 256 "$img" | awk '{print $1}')"; fi
  printf 'WARNING\nYou are about to modify:\n  %s\nDevice: %s %s\nImage: %s\nSHA-256: %s\nOriginal backup: %s\nThis operation modifies the boot chain.\nContinue?\n' "$part" "$(marketing_name)" "$PROFILE_ID" "$img" "$sha" "$BACKUP_DIR"
  if [ -z "$yes" ]; then
    printf '%s' "$(L "Type 'FLASH' to continue (1/2): " "Zum Fortfahren 'FLASHEN' tippen (1/2): ")"; iread -r a
    if [ "$a" != "FLASH" ] && [ "$a" != "FLASHEN" ]; then log WARNING "$(L 'Flash aborted.' 'Flash abgebrochen.')"; return 1; fi
    printf '%s' "$(L "Type 'YES' again (2/2): " "Nochmal 'JA' (2/2): ")"; iread -r b
    if [ "$b" != "YES" ] && [ "$b" != "JA" ]; then log WARNING "$(L 'Flash aborted.' 'Flash abgebrochen.')"; return 1; fi
  else log WARNING "$(L 'CLI --yes: explicit consent documented.' 'CLI --yes dokumentiert.')"
  fi
  log WARNING "Starting: fastboot flash $part <patched>"
  local out; out="$(fb_flash flash "$part" "$img" 2>&1)"
  printf '%s\n' "$out" | tee -a "$TTLOG"
  if printf '%s' "$out" | grep -q -i -E 'OKAY|finished|Writing'; then log SUCCESS "$(L 'Flash reported: OK.' 'Flash gemeldet: OK.')"; return 0; fi
  log ERROR "$(L 'Flash output unclear/faulty.' 'Flash-Ausgabe unklar/fehlerhaft.')"; return 1
}
verify_root() { # [--no-reboot]
  local noreboot="${1:-}"
  if [ -z "$noreboot" ] && [ "$MODE" = "fastboot" ] && [ -n "$FB_BIN" ]; then
    log INFO "fastboot reboot ..."; fb_run reboot >/dev/null 2>&1 || true
  fi
  printf '%b\n' "${C_CYN}$(L 'Huawei boot procedure (mandatory, else no root):' 'Huawei Boot-Prozedur (Pflicht, sonst kein Root):')${C_RST}"
  printf '%s\n' "$(L '  Vol-Up + Power until Huawei logo, then release (Magisk boot cheat).' '  Vol-Up + Power bis Huawei-Logo, dann loslassen (Magisk boot cheat).')"
  if [ -n "$ADB_BIN" ]; then
    log INFO "$(L 'Waiting for adb (up to 120s) ...' 'Warte auf adb (bis 120s) ...')"
    if [ "$USE_TMO" = 1 ]; then timeout 150 "$ADB_BIN" wait-for-device 2>/dev/null || true
    else "$ADB_BIN" wait-for-device 2>/dev/null || true; fi
    sleep 5; detect_mode
  fi
  local w id mv
  w="$(adb_run shell which su 2>&1 | tr -d '\r')"
  id="$(adb_run shell su -c id 2>&1 | tr -d '\r')"
  mv="$(adb_run shell magisk -v 2>&1 | tr -d '\r')"
  printf '%s\n' "which su: $w" "su -c id: $id" "magisk -v: $mv" | tee -a "$TTLOG"
  case "$id" in *uid=0*) log SUCCESS "ROOT DETECTED (uid=0)."; printf 'ROOTED\n'; return 0 ;; esac
  if [ -n "$w" ] && [[ "$w" != *"not found"* ]]; then log WARNING "INCONCLUSIVE (su present, no uid=0)."; printf 'INCONCLUSIVE\n'; return 2; fi
  log WARNING "$(L 'No root verifiable.' 'Kein Root nachweisbar.')"; printf 'NOT_ROOTED\n'; return 2
}
do_restore() { # [backupdir] [--yes]
  local pick="${1:-}" yes="${2:-}" part
  part="$(target_partition)"
  # Never flash without a live fastboot device.
  detect_mode
  if [ "$MODE" != "fastboot" ]; then
    log ERROR "$(L 'Not in fastboot mode. Reboot to fastboot first, then restore.' 'Nicht im Fastboot-Modus. Erst nach Fastboot booten, dann Restore.')"
    return 1
  fi
  if [ -z "$pick" ]; then pick="$(find "$BACK_DIR/$PROFILE_ID/$part" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | sort -r | head -1)"; fi
  [ -z "$pick" ] && pick="$(find "$BACK_DIR" -name original.img 2>/dev/null | head -1 | xargs -r dirname)"
  if [ -z "$pick" ] || [ ! -f "$pick/original.img" ]; then log ERROR "$(L 'No backup with original.img found.' 'Kein Backup mit original.img gefunden.')"; return 1; fi
  log INFO "$(L 'Restore candidate: ' 'Restore-Kandidat: ')$pick"
  if [ -z "$yes" ]; then
    printf '%s' "$(L "Type 'RESTORE' (1/2): " "'RESTORE' tippen (1/2): ")"; iread -r a; [ "$a" = "RESTORE" ] || return 1
    printf '%s' "$(L "Type 'YES' (2/2): " "'JA' tippen (2/2): ")"; iread -r b; { [ "$b" = "YES" ] || [ "$b" = "JA" ]; } || return 1
  fi
  fb_flash flash "$part" "$pick/original.img" 2>&1 | tee -a "$TTLOG"
  log WARNING "$(L 'Restore executed.' 'Restore ausgefuehrt.')"
}
do_diagnostic() { # [--anonymize]
  local anon="${1:-}" stamp ser zipf tmp
  stamp="$(date +%Y%m%d-%H%M%S)"
  tmp="$(mktemp -d 2>/dev/null || printf '/tmp/ttdiag-%s' "$stamp")"
  mkdir -p "$tmp"
  ser="${ADB_SERIAL:-$FB_SERIAL}"
  [ -n "$anon" ] && ser="***ANONYMIZED***"
  {
    printf '{\n  "tool": "trebleManager %s",\n  "mode": "%s",\n  "serial": "%s",\n  "profile": "%s",\n  "os": "%s",\n  "firmware": "%s"\n}\n' "$TTVERSION" "$MODE" "$ser" "$PROFILE_ID" "$OS_KIND" "$FW_BASELINE"
  } > "$tmp/device.json"
  { [ -n "$ADB_BIN" ] && "$ADB_BIN" devices; } > "$tmp/adb.txt" 2>&1
  [ -f "$FBRAW_FILE" ] && cp "$FBRAW_FILE" "$tmp/fastboot.txt" || : > "$tmp/fastboot.txt"
  printf '%s' "${BYNAME_RAW:-"(none)"}" > "$tmp/partitions.txt"
  { [ -f "$PROPS_FILE" ] && cat "$PROPS_FILE"; } > "$tmp/properties.txt" 2>&1
  printf 'cmdline: %s\nbootkeys: Vol-Up + Power until Huawei logo (Magisk boot cheat)\n' "${CMDLINE:-}" > "$tmp/boot.txt"
  printf 'baseline: %s\nstatus: %s\n' "$FW_BASELINE" "$FW_STATUS" > "$tmp/firmware.txt"
  : > "$tmp/hashes.txt"
  for f in "$STOCK_IMAGE" "$PATCHED_IMAGE"; do
    if [ -n "$f" ] && [ -f "$f" ]; then
      if command -v sha256sum >/dev/null 2>&1; then sha256sum "$f" >> "$tmp/hashes.txt"; else shasum -a 256 "$f" >> "$tmp/hashes.txt"; fi
    fi
  done
  printf 'trebleManager %s\n' "$TTVERSION" > "$tmp/tool-version.txt"
  cp "$TTLOG" "$tmp/log.txt" 2>/dev/null || true
  zipf="$LOG_DIR/Huawei-P10-Diagnostic-$stamp.zip"
  if command -v zip >/dev/null 2>&1; then (cd "$tmp" && zip -qr "$zipf" ./*) \
  elif command -v python3 >/dev/null 2>&1; then python3 -c "import zipfile,os,sys; z=zipfile.ZipFile(sys.argv[2],'w',zipfile.ZIP_DEFLATED); [z.write(os.path.join(r,f),f) for r,_,fs in os.walk(sys.argv[1]) for f in fs]" "$tmp" "$zipf"
  else log ERROR "$(L 'Neither zip nor python3 for diagnostic archive.' 'Weder zip noch python3 fuer Diagnose-Archiv.')"; return 1; fi
  rm -rf "$tmp"
  log SUCCESS "$(L 'Diagnostic ZIP: ' 'Diagnose-ZIP: ')$zipf"
  printf '%s\n' "$zipf"
}

# Compatibility registry (JSON mirror of data/compatibility YAML).
compat_file() { printf '%s/data/compatibility/huawei/p10/%s.json' "$TOOL_ROOT" "$PROFILE_ID"; }
compat_roms() { # prints "name|status|reason-or-note" lines, or fails
  local f; f="$(compat_file)"
  [ -f "$f" ] || return 1
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "import json,sys; [print(r.get('name','')+'|'+r.get('status','')+'|'+str(r.get('reason',r.get('note','')))) for r in json.load(open(sys.argv[1])).get('roms',[])]" "$f" 2>/dev/null && return 0
  fi
  grep -o '"name": "[^"]*"[^"]*"status": "[^"]*"' "$f" 2>/dev/null | sed 's/"name": //;s/"status": /|/;s/"//g' || return 1
}
compat_broken_markers() { # prints lowercase markers, one per line
  local f; f="$(compat_file)"
  [ -f "$f" ] || return 1
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "import json,sys; [print(m.lower()) for r in json.load(open(sys.argv[1])).get('roms',[]) if r.get('status')=='broken' for m in (r.get('markers') or [])]" "$f" 2>/dev/null && return 0
  fi
  tr -d '\n' < "$f" | grep -o '"markers": \[[^]]*\]' 2>/dev/null | grep -o '"[a-z0-9]*"' | tr -d '"' | grep -v '^markers$' || true
}
vendor_advice() { # emui target_android -> message
  local emui="$1" tgt="$2"
  if [ -z "$emui" ]; then printf 'Vendor base unknown (GSI hides it) - assume nothing.'; return; fi
  case "$emui" in
    *8.*) printf 'Oreo vendor: Q/8.1 problematic, P good. Target %s on Oreo = RISK.' "$tgt" ;;
    *9.*) printf 'Pie vendor: Q/R/S boot. Target %s expected to boot.' "$tgt" ;;
    *) printf "Vendor base '%s' unclassified - verify manually." "$emui" ;;
  esac
}
screen_compat() {
  header "$(L 'Compatibility registry (researched ROM/firmware matrix)' 'Kompatibilitaets-Registry')"
  printf '\n'
  local roms; if ! roms="$(compat_roms)"; then
    printf '%s\n' "$(L 'No registry for this profile. Submit device data first.' 'Keine Registry. Erst Geraetedaten einreichen.')"; pause_tt; return
  fi
  printf '%s\n' "$(L 'Recommended (tested builds only):' 'Empfohlen (nur getestet):')"
  printf '%s\n' "$roms" | grep -E '\|working' | sed 's/^/ [+] /'
  printf '\n%s\n' "$(L 'NOT recommended (researched):' 'NICHT empfohlen:')"
  printf '%s\n' "$roms" | grep -v -E '\|working' | sed 's/^/ [X] /'
  pause_tt
}
header() { # title
  clear 2>/dev/null || true
  printf '%b\n' "${C_CYN}================================================================${C_RST}"
  printf '%b\n' "${C_CYN} Huawei P10 Root Manager  v$TTVERSION  |  bash ($(bash --version 2>/dev/null | head -1 | awk '{print $4}' || printf '?'))${C_RST}"
  printf '%b\n' "${C_GRY} Mode: $(printf '%s' "$MODE" | tr '[:lower:]' '[:upper:]')  Profile: $PROFILE_ID  OS: $OS_KIND${C_RST}"
  printf '%b\n' "${C_CYN}================================================================${C_RST}"
  printf ' %s\n' "$1"
}
pause_tt() { printf '%s' "$(L '[Enter] back ...' '[Enter] zurueck ...')"; iread -r _; }
menu() { # title opt1 opt2... -> prints index via REPLY_MENU (0-based), -1 on q
  local title="$1"; shift
  local n=$# i=0
  while true; do
    header "$title"; printf '\n'
    i=1; for o in "$@"; do printf '  [%d] %s\n' "$i" "$o"; i=$((i+1)); done
    printf '\n%s' "$(L 'Number (q=back): ' 'Nummer (q=zurueck): ')"; iread -r ans
    case "$ans" in q|Q|"") REPLY_MENU=-1; return ;; esac
    if [ "$ans" -ge 1 ] 2>/dev/null && [ "$ans" -le "$n" ]; then REPLY_MENU=$((ans-1)); return; fi
  done
}
status_screen() {
  header "$(L 'Status (any OS detected, nothing assumed)' 'Status (jedes OS erkannt, nichts vorausgesetzt)')"; printf '\n'
  printf 'Device profile : %s (%s)\nOS class         : %s\nOS detail        : %s\nAndroid          : %s\nMode             : %s\n' "$PROFILE_ID" "$(marketing_name)" "$OS_KIND" "$OS_DETAIL" "$OS_RELEASE" "$MODE"
  if printf '%s' "$BYNAME_RAW" | grep -q recovery_ramdisk; then printf 'RecoveryRamdisk: DETECTED\n'; else printf 'RecoveryRamdisk: UNKNOWN/NOT DETECTED\n'; fi
  printf 'Firmware         : %s\nStock image      : %s\nPatched image    : %s\nBackup           : %s\n' "${FW_BASELINE:-UNKNOWN}" "${STOCK_IMAGE:-missing}" "${PATCHED_IMAGE:-missing}" "${BACKUP_DIR:-missing}"
  pause_tt
}
screen_detect() {
  header "$(L 'Step 1 - Device (detect, auto ADB/fastboot)' 'Step 1 - Device (auto ADB/Fastboot)')"; printf '\n'
  detect_mode
  printf 'Mode: %s\n' "$MODE"
  [ -n "$ADB_SERIAL" ] && printf 'ADB: %s\n' "$ADB_SERIAL"
  [ -n "$FB_SERIAL" ] && printf 'Fastboot: %s\n' "$FB_SERIAL"
  if [ "$MODE" = "none" ]; then printf '\n%s\n' "$(L 'No device. Check USB debugging / drivers / cable.' 'Kein Geraet. Debugging/Treiber/Kabel pruefen.')"; else log SUCCESS "$(L 'Device mode: ' 'Device-Modus: ')$MODE"; fi
  pause_tt
}
screen_analyze() {
  header "$(L 'Step 2 - Analyze (Android + partitions + boot chain, read-only)' 'Step 2 - Analyse (read-only)')"; printf '\n'
  detect_mode
  if [ "$MODE" = "android" ]; then
    android_analysis
    printf '\nOS class: %s\nDetail: %s\n' "$OS_KIND" "$OS_DETAIL"
    linver="$(grep '^ro.lineage.version=' "$PROPS_FILE" | cut -d= -f2-)"
    [ -z "$linver" ] && linver="$(grep '^ro.lineageos.version=' "$PROPS_FILE" | cut -d= -f2-)"
    [ -n "$linver" ] && printf 'LineageOS: %s (%s)\n' "$linver" "$(L 'valid starting point - stock source still needed for Magisk' 'gueltiger Startpunkt - Stock-Quelle weiter noetig fuer Magisk')"
    case "$OS_KIND:$OS_RELEASE" in *GSI*:13*|*GSI*:14*)
      case "$PROFILE_ID" in VTR-*|VKY-*) printf 'WARN: %s\n' "$WIKI_ANDROID13_WARN" ;; esac ;;
    esac
    printf 'Storage: %s\n' "$STORAGE"
    emui="$(grep '^ro.build.version.emui=' "$PROPS_FILE" | cut -d= -f2-)"
    [ -z "$emui" ] && emui="$(grep '^ro.emui.version=' "$PROPS_FILE" | cut -d= -f2-)"
    printf 'Vendor advice: %s\n' "$(vendor_advice "$emui" "$OS_RELEASE")"
    printf '\n--- Partitions ---\n'
    printf '%s\n' "$BYNAME_RAW" | grep -E 'boot|recovery|ramdisk|system|vendor|vbmeta' || printf '%s\n' "$BYNAME_RAW"
    if printf '%s' "$BYNAME_RAW" | grep -q recovery_ramdisk; then printf 'recovery_ramdisk: DETECTED\n'; else printf '%s\n' "$(L 'recovery_ramdisk NOT in by-name -> fastboot analysis + firmware path needed.' 'recovery_ramdisk NICHT in by-name -> Fastboot + Firmware-Weg noetig.')"; fi
  elif [ "$MODE" = "fastboot" ]; then
    printf '%s\n' "$(L 'Device in fastboot -> fastboot analysis now.' 'Geraet in Fastboot -> jetzt Fastboot-Analyse.')"
    fastboot_analysis
    cat "$FBRAW_FILE"
  else printf '%s\n' "$(L 'No device connected.' 'Kein Geraet verbunden.')"; fi
  pause_tt
}
screen_firmware() {
  header "$(L 'Step 3 - Firmware (determine compatible)' 'Step 3 - Firmware (kompatibel bestimmen)')"; printf '\n'
  if [ "$MODE" = "android" ] && [ -z "$OS_KIND" -o "$OS_KIND" = "?" ]; then android_analysis; fi
  if [ -z "$FW_BASELINE" ]; then
    printf '%s' "$(L 'Original Huawei firmware (e.g. VTR-L29 9.1.0.297(C432E5R1P9), Enter=later): ' 'Original-Firmware (z.B. VTR-L29 9.1.0.297(C432E5R1P9), Enter=spaeter): ')"; iread -r FW_BASELINE
  fi
  printf 'Baseline: %s\n\nSources (no dubious auto-download):\n - https://professorjtj.github.io/v2/\n - Discussion #2542 example: VTR-L29 9.1.0.297(C432E5R1P9) (NOT exclusive)\n - Wiki P10: EMUI 9.1 base, RECOVERY_RAMDIS.img from UPDATE.APP\n' "${FW_BASELINE:-(empty)}"
  local region=""; region="$(printf '%s' "$FW_BASELINE" | grep -o '(C[0-9]*' | head -1 | tr -d '(')"
  if [ -z "$region" ]; then printf '%s' "$(L 'Region/CUST (e.g. C432, Enter=unknown): ' 'Region/CUST (z.B. C432, Enter=unbekannt): ')"; iread -r region; fi
  local out; out="$(firmware_compat "$PROFILE_ID" "$FW_BASELINE" "$region")"
  FW_STATUS="${out%%|*}"
  printf '\nCompatibility: %s\n%s\n' "$FW_STATUS" "${out##*|}"
  printf '\n%s' "$(L '[D] download stock firmware  [Enter] back: ' '[D] Stock-Firmware laden  [Enter] zurueck: ')"; iread -r k
  case "$k" in d|D) screen_download ;; esac
}
screen_download() {
  header "$(L 'Stock firmware download (original, progress + confirmation)' 'Stock-Download (original, Fortschritt + Bestaetigung)')"; printf '\n'
  printf '%s\n\nTrusted sources:\n [official] HiSuite: https://consumer.huawei.com/de/support/hisuite/\n [official] Consumer search: https://consumer.huawei.com/de/support/\n [historic] FIRM FINDER V2: https://professorjtj.github.io/v2/\n [archive] androidhost.ru: https://androidhost.ru/search.html?search=VTR-L29\n\n' "$(L 'Target: data/firmware/ (offline cache). Full packages only.' 'Ziel: data/firmware/ (Cache). Nur Full-Pakete.')"
  printf '%s' "$(L 'Direct link to full firmware ZIP (Enter=abort): ' 'Direktlink Full-Firmware-ZIP (Enter=Abbruch): ')"; iread -r url
  [ -z "$url" ] && { pause_tt; return; }
  valid_url "$url" || { log ERROR "$(L 'URL rejected.' 'URL abgelehnt.')"; pause_tt; return; }
  local out="$FIRM_DIR/stock-firmware-$PROFILE_ID-$STAMP.zip"
  printf '\nURL: %s\nTarget: %s\n%s' "$url" "$out" "$(L "Type 'YES' to download (2-4 GB): " "'JA' tippen (2-4 GB): ")"; iread -r c
  if [ "$c" != "YES" ] && [ "$c" != "JA" ]; then log WARNING "$(L 'Download aborted.' 'Download abgebrochen.')"; pause_tt; return; fi
  download_firmware "$url" "$out" --yes && verify_download "$out" || true
  pause_tt
}
screen_extract() {
  header "$(L 'Step 4 - Extract/validate RECOVERY_RAMDISK (UPDATE.APP)' 'Step 4 - RECOVERY_RAMDISK (UPDATE.APP)')"; printf '\n'
  local apps; apps="$(find "$FIRM_DIR" -iname '*.APP' -type f 2>/dev/null | head -5)"
  if [ -n "$apps" ]; then printf '%s\n%s\n' "$(L 'Found:' 'Gefunden:')" "$apps"; else printf '%s\n' "$(L 'No UPDATE.APP in data/firmware/.' 'Keine UPDATE.APP in data/firmware/.')"; fi
  local found; found="$(find "$FIRM_DIR" "$DATA_DIR" -maxdepth 3 \( -iname 'RECOVERY_RAMDISK.img' -o -iname 'RECOVERY_RAMDIS.img' -o -iname 'recovery_ramdisk.img' \) -type f 2>/dev/null | head -5)"
  if [ -n "$found" ]; then
    printf '%s\n' "$(L 'Recovery images found:' 'RECOVERY-Images gefunden:')"
    local f; f="$(printf '%s' "$found" | head -1)"
    STOCK_IMAGE="$f"
    local t; t="$(test_image "$f")"
    STOCK_SHA="$(printf '%s' "$t" | cut -d'|' -f2)"
    printf 'Check: %s\n' "$t"
    [ "${t%%|*}" != "PASS" ] && printf '%s\n' "$(L 'Image not verifiable -> Cannot safely flash.' 'Image nicht verifizierbar -> Cannot safely flash.')"
  else printf '%s\n' "$(L 'No RECOVERY_RAMDIS(K).img found. Obtain it first.' 'Keine RECOVERY_RAMDIS(K).img. Erst beschaffen.')"; fi
  pause_tt
}
screen_export() {
  header "$(L 'Recovery export from compatible custom ROMs' 'Recovery-Export aus Custom-ROMs')"; printf '\n'
  printf '%s\n' "$(L 'Drop ROM packages into data/roms/ (.zip or .img). GSI system images are refused honestly.' 'ROM-Pakete nach data/roms/ (.zip/.img). GSI wird ehrlich abgelehnt.')"
  local roms; roms="$(find "$ROM_DIR" -maxdepth 1 \( -iname '*.zip' -o -iname '*.img' \) -type f 2>/dev/null)"
  local rom=""
  if [ -z "$roms" ]; then
    printf '%s' "$(L 'ROM path (Enter=abort): ' 'ROM-Pfad (Enter=Abbruch): ')"; iread -r rom
    [ -z "$rom" ] && { pause_tt; return; }
  else
    printf '%s\n%s\n%s' "$(L 'Found:' 'Gefunden:')" "$roms" "$(L 'Number (Enter=first): ' 'Nummer (Enter=erste): ')"; iread -r n
    rom="$(printf '%s' "$roms" | sed -n "${n:-1}p")"
    [ -z "$rom" ] && rom="$(printf '%s' "$roms" | head -1)"
  fi
  local d; if d="$(export_recovery "$rom")"; then
    printf '%s\n%s\n' "$(L 'Export dir:' 'Export-Verz.:') $d" "$(L 'Next: patch in Magisk app, flash only to profile target after safety gate.' 'Weiter: in Magisk patchen, nur Zielpartition nach Safety-Gate flashen.')"
  fi
  pause_tt
}
screen_patch() {
  header "$(L 'Step 5 - Magisk (compatible, real patch)' 'Step 5 - Magisk (kompatibel, echt)')"; printf '\n'
  printf 'Official: https://github.com/topjohnwu/Magisk/releases\n'
  local apk; apk="$(magisk_apk)"
  if [ -z "$apk" ]; then
    printf '%s' "$(L 'No Magisk APK in data/magisk/. Place it there, or path (Enter=later): ' 'Keine Magisk-APK in data/magisk/. Dort ablegen oder Pfad (Enter=spaeter): ')"; iread -r p
    if [ -n "$p" ] && [ -f "$p" ]; then cp -f "$p" "$MAG_DIR/"; apk="$MAG_DIR/$(basename "$p")"; fi
  fi
  [ -n "$apk" ] && printf 'APK: %s\n' "$apk"
  if [ -z "$STOCK_IMAGE" ] || [ ! -f "$STOCK_IMAGE" ]; then printf '%s\n' "$(L 'No stock image -> step 4 first.' 'Kein Stock-Image -> erst Step 4.')"; pause_tt; return; fi
  printf 'Input: %s\nTarget: %s\nDevice: Huawei %s\nFirmware: %s\nSHA-256: %s\n' "$STOCK_IMAGE" "$(target_partition)" "$PROFILE_ID" "$FW_BASELINE" "$STOCK_SHA"
  printf '%s\n' "$(L '[1] Prepare patch (to-patch + instructions)  [2] Register patched file' '[1] Patch vorbereiten  [2] Gepatchte Datei registrieren')"; iread -r k
  case "$k" in
    1) prepare_patch "$STOCK_IMAGE" >/dev/null; printf '%s\n' "$(L 'Prepared. Patch on device per data/magisk/to-patch/PATCH-INSTRUCTIONS.txt.' 'Vorbereitet. Am Geraet patchen (Anleitung in to-patch/).')" ;;
    2) adb_run shell 'ls /sdcard/Download/magisk_patched*.img 2>&1' 2>/dev/null || true
       printf '%s' "$(L 'Path to patched file: ' 'Pfad gepatchte Datei: ')"; iread -r pp
       if [ -n "$pp" ] && [ -f "$pp" ]; then
         local t ph; t="$(test_image "$pp")"; ph="$(printf '%s' "$t" | cut -d'|' -f2)"
         if [ "$ph" = "$STOCK_SHA" ]; then log ERROR "$(L 'ERROR: patched == stock. NO fake patch accepted.' 'FEHLER: gepatcht == Stock. KEIN Fake-Patch.')"
         elif [ "${t%%|*}" = "PASS" ]; then PATCHED_IMAGE="$pp"; PATCHED_SHA="$ph"; log SUCCESS "Patched registered: $pp"
         else printf '%s\n' "$(L 'Image check FAIL.' 'Image-Pruefung FAIL.')"; fi
       fi ;;
  esac
  pause_tt
}
screen_backup() {
  header "$(L 'Step 6 - Backup (mandatory before flash)' 'Step 6 - Backup (Pflicht vor Flash)')"; printf '\n'
  if [ -z "$STOCK_IMAGE" ] || [ ! -f "$STOCK_IMAGE" ]; then printf '%s\n' "$(L 'No stock image -> step 4 first.' 'Kein Stock -> erst Step 4.')"; pause_tt; return; fi
  do_backup "$STOCK_IMAGE" && printf 'Backup: %s\n' "$BACKUP_DIR"
  pause_tt
}
screen_flash() {
  header "$(L 'Step 7 - Flash (only after safety gate)' 'Step 7 - Flash (nur nach Safety-Gate)')"; printf '\n'
  if [ "$MODE" != "fastboot" ]; then
    printf '%s' "$(L "'adb reboot bootloader' now? [y/N]: " "'adb reboot bootloader' jetzt? [y/N]: ")"; iread -r k
    case "$k" in y|Y|j|J) adb_run reboot bootloader 2>/dev/null; for _ in $(seq 1 30); do sleep 1; detect_mode; [ "$MODE" = "fastboot" ] && break; done ;; esac
  fi
  safe_flash "$PATCHED_IMAGE" || true
  pause_tt
}
screen_verify() {
  header "$(L 'Step 8+9 - Reboot (Huawei procedure) + verify (real)' 'Step 8+9 - Reboot + Verify (echt)')"; printf '\n'
  local r; if r="$(verify_root)"; then printf 'Result: %s\n' "$r"; else printf 'Result: %s (see above)\n' "$r"; fi
  pause_tt
}
screen_unlock() {
  header "$(L 'Bootloader unlock (PotatoNV, wiki method - guided only)' 'Bootloader-Unlock (PotatoNV, nur Anleitung)')"; printf '\n'
  printf '%s\n' "$(L 'The tool NEVER unlocks anything itself.' 'Das Tool unlockt NIEMALS selbst.')"
  printf '%s\n' "- Huawei locks TWO levels: USER LOCK and BL LOCK." \
    "- 1. PotatoNV testpoint method: https://github.com/mashed-potatoes/PotatoNV" \
    "- 2. Engineering fastboot -> 'Disable FBLock' (unlocks USER LOCK)." \
    "- 3. Normal fastboot shows unlocked (not fully). PotatoNV shows a 16-digit code." \
    "- 4. fastboot oem unlock XXXXXXXXXXXXXXXX (your code)."
  printf '\n%s\n' "$(L 'GSI already booting? Unlock is done - continue with step 2.' 'GSI bootet bereits? Unlock erledigt - weiter mit Step 2.')"
  pause_tt
}
screen_kernelfixes() {
  header "$(L 'Kernels + known fixes (P10 wiki)' 'Kernel + bekannte Fixes (Wiki)')"; printf '\n'
  printf '%s\n' "Kernels: Proto8/HyperPlus (EMUI8), Pangu (EMUI9 CN), KernelSU v0.9.2 only."
  printf '%s\n' "Fixes (need root): stop aptouch (touchscreen edges); chown root:audio + chmod 0660 /dev/nxp_smartpa_dev (speakers)."
  printf '%s\n' "$(L 'TWRP rule: factory reset ONLY via stock recovery.' 'TWRP-Regel: Reset NUR via Stock-Recovery.')"
  pause_tt
}
test_system_image() { # path -> PASS|FAIL|notes on stdout
  [ -f "$1" ] || { printf 'FAIL|missing'; return; }
  local sz; sz="$(stat -c%s "$1" 2>/dev/null || stat -f%z "$1")"
  [ "$sz" -ge 524288000 ] || { printf 'FAIL|too small (< 500 MB), full GSI expected'; return; }
  local magic; magic="$(od -An -tx1 -N4 "$1" 2>/dev/null | tr -d ' \n' | tr '[:lower:]' '[:upper:]')"
  local fn; fn="$(basename "$1" | tr '[:upper:]' '[:lower:]')"
  case "$fn" in *arm64*) ;; *) printf 'FAIL|filename not arm64 (P10 needs arm64 A-only)'; return ;; esac
  case "$fn" in *_ab*|*a/b*) printf 'FAIL|A/B image (P10 needs A-only)'; return ;; esac
  printf 'PASS|header=%s' "$magic"
}
system_flash() { # image [--yes]
  local img="$1" yes="${2:-}" t
  t="$(test_system_image "$img")"
  printf '%s\n' "$(L '=== System image check (ROM install) ===' '=== System-Image-Pruefung ===')"
  printf 'Result: %s\n' "$t"
  # Registry cross-check: researched-broken builds block hard.
  local low mk blocked=""
  low="$(basename "$img" | tr '[:upper:]' '[:lower:]')"
  for mk in $(compat_broken_markers); do
    case "$low" in *"$mk"*) blocked="$mk" ;; esac
  done
  [ -n "$blocked" ] && printf 'REGISTRY BLOCKED: marker "%s" is researched BROKEN.\n' "$blocked"
  if [ "${t%%|*}" != "PASS" ] || [ -n "$blocked" ] || [ "$(profile_verified)" != "1" ]; then
    printf 'DO NOT FLASH\n'; log ERROR "$(L 'System flash blocked.' 'System-Flash blockiert.')"; return 1
  fi
  printf 'WARNING\n%s\n%s\n' "$(L 'You are about to REPLACE the Android system (fastboot flash system).' 'Du ersetzt das Android-System.')" "$(L 'Back up storage first. Afterwards: eRecovery wipe + first boot.' 'Speicher sichern. Danach: eRecovery Wipe + Erstboot.')"
  if [ -z "$yes" ]; then
    printf '%s' "$(L "Type 'FLASH' (1/2): " "'FLASHEN' (1/2): ")"; iread -r a
    { [ "$a" = "FLASH" ] || [ "$a" = "FLASHEN" ]; } || return 1
    printf '%s' "$(L "Type 'YES' (2/2): " "'JA' (2/2): ")"; iread -r b
    { [ "$b" = "YES" ] || [ "$b" = "JA" ]; } || return 1
  fi
  detect_mode
  if [ "$MODE" != "fastboot" ]; then log ERROR "$(L 'Not in fastboot mode, aborting.' 'Nicht im Fastboot-Modus, Abbruch.')"; return 1; fi
  log WARNING "Starting: fastboot flash system <gsi>"
  local out; out="$(fb_flash flash system "$img" 2>&1)"
  printf '%s\n' "$out" | tee -a "$TTLOG"
  if printf '%s' "$out" | grep -q -i -E 'OKAY|finished|Writing'; then
    log SUCCESS "$(L 'System flash OK. Next: fastboot reboot -> eRecovery wipe -> setup.' 'System-Flash OK. Weiter: Reboot -> eRecovery Wipe -> Setup.')"; return 0
  fi
  log ERROR "$(L 'System flash output unclear.' 'Ausgabe unklar.')"; return 1
}
screen_flashsystem() {
  header "$(L 'Install ROM / GSI system image (fully guided)' 'ROM / GSI installieren (voll gefuehrt)')"; printf '\n'
  printf '%s\n' "- Base EMUI 8/9/9.1. Backup storage. Reset only via stock recovery." "- fastboot flash system <gsi.img>, then eRecovery wipe. Slim builds if system is small."
  local adv; adv="$(profile_gsi_advice)"; [ -n "$adv" ] && printf 'Profile advice: %s\n' "$adv"
  printf '%s' "$(L 'GSI image path (*-arm64_*.img, unpacked): ' 'GSI-Image-Pfad (entpackt): ')"; iread -r img
  if [ -z "$img" ] || [ ! -f "$img" ]; then log ERROR "$(L 'Invalid path, aborting.' 'Pfad ungueltig, Abbruch.')"; pause_tt; return; fi
  system_flash "$img" || true
  pause_tt
}
# Root methods in priority order. Magisk patched recovery_ramdisk is PREFERRED.
# TWRP shares the SAME partition slot - slots overwrite each other; restore switches back.
root_method_ids() { printf 'magisk-recovery\nmagisk-twrp\nphh-su\nkernelsu\n'; }
root_method_name() {
  case "$1" in
    magisk-recovery) printf 'Magisk patched recovery_ramdisk (PREFERRED)' ;;
    magisk-twrp) printf 'Magisk via TWRP zip (alternative)' ;;
    phh-su) printf 'phh superuser (legacy, no modules)' ;;
    kernelsu) printf 'KernelSU (experimental, v0.9.2 only)' ;;
  esac
}
ROOT_METHOD="magisk-recovery"
screen_rootmethods() {
  header "$(L 'Root methods (Magisk preferred + alternatives)' 'Root-Methoden (Magisk bevorzugt + Alternativen)')"; printf '\n'
  local m mark
  while IFS= read -r m; do
    mark=" "; [ "$m" = "$ROOT_METHOD" ] && mark="*"
    printf ' [%s] %s: %s\n' "$mark" "$m" "$(root_method_name "$m")"
  done <<EOF
$(root_method_ids)
EOF
  printf '\nCurrent: %s\n%s' "$ROOT_METHOD" "$(L 'ID to select (Enter=keep): ' 'ID zum Waehlen (Enter=behalten): ')"; iread -r s
  case "$s" in magisk-recovery|magisk-twrp|phh-su|kernelsu) ROOT_METHOD="$s"; log SUCCESS "Root method: $s" ;; "") ;; *) log WARNING "$(L 'Unknown method, kept.' 'Unbekannte Methode, behalten.')" ;; esac
  printf '%s\n' "$(L 'Note: TWRP and Magisk-recovery share the recovery_ramdisk slot.' 'Hinweis: TWRP und Magisk teilen den recovery_ramdisk-Slot.')"
  pause_tt
}
twrp_flash() { # image [--yes]
  local img="$1" yes="${2:-}" t part
  t="$(test_image "$img" 2>/dev/null || printf 'FAIL|missing')"
  printf '%s\n' "$(L '=== TWRP image check ===' '=== TWRP-Pruefung ===')"
  printf 'Result: %s\n' "$t"
  if [ "${t%%|*}" != "PASS" ] || [ "$(profile_verified)" != "1" ]; then
    printf 'DO NOT FLASH\n'; log ERROR "$(L 'TWRP flash blocked.' 'TWRP-Flash blockiert.')"; return 1
  fi
  part="$(target_partition)"
  printf 'WARNING\n%s\n%s\n' "$(L 'TWRP and Magisk-recovery SHARE the recovery_ramdisk slot (mutual overwrite).' 'TWRP und Magisk TEILEN den Slot (gegenseitiges Ueberschreiben).')" "$(L 'Boot TWRP: hold Vol-Up. NEVER wipe userdata in TWRP.' 'TWRP booten: Vol-Up halten. NIEMALS userdata in TWRP wipen.')"
  if [ -z "$yes" ]; then
    printf '%s' "$(L "Type 'FLASH' (1/2): " "'FLASHEN' (1/2): ")"; iread -r a
    { [ "$a" = "FLASH" ] || [ "$a" = "FLASHEN" ]; } || return 1
    printf '%s' "$(L "Type 'YES' (2/2): " "'JA' (2/2): ")"; iread -r b
    { [ "$b" = "YES" ] || [ "$b" = "JA" ]; } || return 1
  fi
  detect_mode
  if [ "$MODE" != "fastboot" ]; then log ERROR "$(L 'Not in fastboot mode, aborting.' 'Nicht im Fastboot-Modus, Abbruch.')"; return 1; fi
  log WARNING "Starting: fastboot flash $part <twrp>"
  local out; out="$(fb_flash flash "$part" "$img" 2>&1)"
  printf '%s\n' "$out" | tee -a "$TTLOG"
  if printf '%s' "$out" | grep -q -i -E 'OKAY|finished|Writing'; then log SUCCESS "$(L 'TWRP flash OK. Boot: hold Vol-Up.' 'TWRP-Flash OK. Boot: Vol-Up halten.')"; return 0; fi
  log ERROR "$(L 'TWRP flash output unclear.' 'Ausgabe unklar.')"; return 1
}
screen_twrp() {
  header "$(L 'TWRP path (guide + guided flash)' 'TWRP-Pfad (Anleitung + Flash)')"; printf '\n'
  printf '%s\n' "$(L 'Sources (device-exact builds only): XDA P10 Plus TWRP 3.2.1-0 (oreo).' 'Quellen (nur genaue Builds): XDA P10 Plus TWRP 3.2.1-0 (oreo).')"
  printf '%s\n' "$(L 'Rules: shared slot with Magisk; backup first; never wipe userdata in TWRP; boot with Vol-Up.' 'Regeln: Slot mit Magisk teilen; erst Backup; nie userdata in TWRP wipen; Boot mit Vol-Up.')"
  printf '%s' "$(L 'TWRP image path: ' 'TWRP-Image-Pfad: ')"; iread -r img
  if [ -z "$img" ] || [ ! -f "$img" ]; then log WARNING "$(L 'Invalid path, aborting.' 'Pfad ungueltig, Abbruch.')"; pause_tt; return; fi
  twrp_flash "$img" || true
  pause_tt
}
validate_device() { # read-only post-flash check; prints "name|1/0|detail" lines
  detect_mode
  if [ "$MODE" != "android" ]; then printf 'ADB|0|no android device\n'; return 1; fi
  local fails=0
  printf 'ADB|1|%s\n' "$ADB_SERIAL"
  local rel disp
  rel="$(adb_run shell getprop ro.build.version.release 2>/dev/null | tr -d '\r\n')"
  disp="$(adb_run shell getprop ro.build.display.id 2>/dev/null | tr -d '\r\n')"
  if [ -n "$rel" ]; then printf 'OS|1|%s / %s\n' "$rel" "$disp"; else printf 'OS|0|empty\n'; fi
  local se; se="$(adb_run shell getenforce 2>/dev/null | tr -d '\r\n')"
  if [ -n "$se" ]; then printf 'SELinux|1|%s\n' "$se"; else printf 'SELinux|0|empty\n'; fi
  local id; id="$(adb_run shell su -c id 2>&1 | tr -d '\r\n')"
  case "$id" in *uid=0*) printf 'ROOT|1|%s\n' "$id" ;; *) printf 'ROOT|0|%s\n' "$id" ;; esac
  local mnt; mnt="$(adb_run shell mount 2>&1)"
  case "$mnt" in */system*) printf 'MOUNTS-system|1|mounted\n' ;; *) printf 'MOUNTS-system|0|missing\n' ;; esac
  case "$mnt" in */vendor*) printf 'MOUNTS-vendor|1|mounted\n' ;; *) printf 'MOUNTS-vendor|0|missing\n' ;; esac
  local wifi; wifi="$(adb_run shell dumpsys wifi 2>&1 | grep -i -m1 'Wi-Fi is' | tr -d '\r')"
  if [ -n "$wifi" ]; then printf 'WIFI|1|%s\n' "$wifi"; else printf 'WIFI|0|empty\n'; fi
  local bt; bt="$(adb_run shell settings get global bluetooth_on 2>&1 | tr -d '\r\n')"
  if [ "$bt" = "1" ] || [ "$bt" = "0" ]; then printf 'BLUETOOTH|1|state=%s\n' "$bt"; else printf 'BLUETOOTH|0|%s\n' "$bt"; fi
  local bat; bat="$(adb_run shell dumpsys battery 2>&1 | grep -i -m1 level | tr -d '\r')"
  if [ -n "$bat" ]; then printf 'BATTERY|1|%s\n' "$bat"; else printf 'BATTERY|0|empty\n'; fi
  local sens; sens="$(adb_run shell dumpsys sensorservice 2>&1 | grep -c -i sensor | tr -d '\r')"
  if [ -n "$sens" ] && [ "$sens" != "0" ]; then printf 'SENSORS|1|entries=%s\n' "$sens"; else printf 'SENSORS|0|none\n'; fi
}
validate_checked() { # runs validate_device, prints output, returns 1 on any FAIL
  local out rc=0
  out="$(validate_device)"
  printf '%s\n' "$out"
  printf '%s' "$out" | grep -q '|0|' && rc=1
  return "$rc"
}
developer_dump() { # kind -> file or refusal
  local kind="$1"
  if [ "$RUNMODE" != "developer" ]; then
    printf '%s\n' "$(L 'Developer mode required: re-run with --mode developer.' 'Developer-Modus noetig: mit --mode developer starten.')"; return 1
  fi
  detect_mode
  if [ "$MODE" != "android" ]; then printf '%s\n' "$(L 'No android device for dump.' 'Kein Android-Geraet fuer Dump.')"; return 1; fi
  local out f
  case "$kind" in
    dump-partitions) out="$(adb_run shell cat /proc/partitions 2>&1)" ;;
    dump-properties) out="$(adb_run shell getprop 2>&1)" ;;
    dump-vendor) out="$(adb_run shell 'ls -l /vendor/etc/ 2>&1; cat /vendor/build.prop 2>&1')" ;;
    dump-logs) out="$(adb_run shell 'logcat -d -t 200 2>&1')" ;;
    *) return 1 ;;
  esac
  f="$LOG_DIR/$kind-$STAMP.txt"
  printf '%s\n' "$out" > "$f"
  log SUCCESS "Dump: $f"
  printf '%s\n' "$f"
}
screen_tools() {
  while true; do
    menu "$(L 'Tools (read-only where possible)' 'Tools (read-only wo moeglich)')" \
      "adb devices -l" "$(L 'Reboot menu' 'Reboot-Menue')" "fastboot devices + getvar" "getprop dump -> logs/" \
      "adb kill-server/start-server" "$(L 'Create diagnostic ZIP' 'Diagnose-ZIP erzeugen')" "$(L 'Mirror via scrcpy (optional)' 'Spiegeln via scrcpy (optional)')" "$(L 'Post-flash validation report' 'Post-Flash-Bericht')" "$(L 'Back' 'Zurueck')"
    c="$REPLY_MENU"; [ "$c" = "-1" ] || [ "$c" = "8" ] && return
    case "$c" in
      0) header "adb devices"; adb_run devices -l 2>&1; pause_tt ;;
      1) menu "$(L 'Reboot target' 'Reboot-Ziel')" bootloader recovery fastbootd system "$(L 'Cancel' 'Abbrechen')"
         rc="$REPLY_MENU"; case "$rc" in 0) adb_run reboot bootloader;; 1) adb_run reboot recovery;; 2) adb_run reboot fastboot;; 3) adb_run reboot;; esac; pause_tt ;;
      2) header "fastboot"; fastboot_analysis; cat "$FBRAW_FILE"; pause_tt ;;
      3) f="$LOG_DIR/getprop-full-$STAMP.txt"; adb_run shell getprop > "$f" 2>&1; log SUCCESS "Dump: $f"; pause_tt ;;
      4) adb_run kill-server >/dev/null 2>&1; adb_run start-server >/dev/null 2>&1; log SUCCESS "ADB reset."; pause_tt ;;
      5) do_diagnostic; pause_tt ;;
      6) if [ -n "${SCRCPY_BIN:-}" ]; then
           log INFO "$(L 'Starting scrcpy mirror (close window to continue) ...' 'Starte scrcpy (Fenster schliessen zum Fortfahren) ...')"
           "$SCRCPY_BIN" >/dev/null 2>&1 &
         else
           printf '%s\n' "$(L 'scrcpy not installed (optional). Get it: https://github.com/Genymobile/scrcpy' 'scrcpy nicht installiert (optional). Bezug: https://github.com/Genymobile/scrcpy')"
         fi
         pause_tt ;;
      7) rep="$(validate_checked)"; rc=$?
         f="$LOG_DIR/validation-$STAMP.json"
         printf '{"tool":"trebleManager %s","results":[' "$TTVERSION" > "$f"
         first=1; while IFS= read -r line; do
           n="${line%%|*}"; rest="${line#*|}"; v="${rest%%|*}"; d="${rest#*|}"
           [ "$first" = 1 ] || printf ',' >> "$f"; first=0
           printf '{"name":"%s","pass":%s,"detail":"%s"}' "$n" "$([ "$v" = 1 ] && printf true || printf false)" "$(printf '%s' "$d" | sed 's/"/\\"/g')" >> "$f"
         done <<EOF
$rep
EOF
         printf ']}\n' >> "$f"
         log SUCCESS "Validation report: $f"
         printf '%s\n' "$rep" | sed 's/^/ /'
         [ "$rc" = 0 ] || log WARNING "$(L 'Validation found FAILs.' 'Validierung fand FAILs.')"
         pause_tt ;;
    esac
  done
}
screen_bootkeys() {
  header "$(L 'Huawei boot mechanism (from #2542, exact)' 'Huawei Boot-Mechanismus (aus #2542, exakt)')"; printf '\n'
  printf '%s\n' "$(L '- Magisk boot: Vol-Up + Power until Huawei logo, then release (boot cheat).' '- Magisk-Boot: Vol-Up + Power bis Logo, dann loslassen.')"
  printf '%s\n' "$(L '- Without trick: stock boot (no root). NOT persistent.' '- Ohne Trick: Stock-Boot (kein Root). NICHT persistent.')"
  printf '%s\n' "$(L '- /dload must NOT be on storage, else EMUI updater instead of recovery.' '- /dload darf NICHT vorhanden sein, sonst EMUI-Updater.')"
  pause_tt
}
main_menu() {
  find_tools; detect_mode
  while true; do
    menu "$(L 'Main menu - Huawei P10 Root Manager (OS-independent)' 'Hauptmenue - Huawei P10 Root Manager')" \
      "$(L 'Status overview' 'Status-Uebersicht')" "$(L 'Wizard steps 1-9' 'Wizard Step 1-9')" \
      "Step 1 - Detect" "Step 2 - Analyze" "Step 3 - Firmware" "Step 4 - Extract" \
      "Step 5 - Magisk" "Step 6 - Backup" "Step 7 - Flash" "Step 8+9 - Verify" \
      "$(L 'Recovery export (custom ROMs)' 'Recovery-Export (Custom-ROMs)')" "$(L 'Install ROM / GSI (guided)' 'ROM / GSI installieren (gefuehrt)')" \
      "$(L 'Root methods (Magisk preferred)' 'Root-Methoden (Magisk bevorzugt)')" "$(L 'TWRP path (guide+flash)' 'TWRP-Pfad (Anleitung+Flash)')" \
      "$(L 'Compatibility registry' 'Kompatibilitaets-Registry')" \
      "$(L 'Unlock guide (PotatoNV)' 'Unlock-Anleitung (PotatoNV)')" "$(L 'Kernels + fixes (wiki)' 'Kernel + Fixes (Wiki)')" \
      "Restore / Unroot" "$(L 'Boot tricks (Huawei)' 'Boot-Tricks (Huawei)')" "$(L 'Tools + diagnostic ZIP' 'Tools + Diagnose-ZIP')" \
      "$(L 'Exit' 'Beenden')"
    c="$REPLY_MENU"
    case "$c" in
      -1|20) log SUCCESS "$(L 'Exiting. Log: ' 'Beendet. Log: ')$TTLOG"; break ;;
      0) status_screen ;; 1) wizard ;; 2) screen_detect ;; 3) screen_analyze ;;
      4) screen_firmware ;; 5) screen_extract ;; 6) screen_patch ;; 7) screen_backup ;;
      8) screen_flash ;; 9) screen_verify ;; 10) screen_export ;; 11) screen_flashsystem ;;
      12) screen_rootmethods ;; 13) screen_twrp ;; 14) screen_compat ;; 15) screen_unlock ;; 16) screen_kernelfixes ;;
      17) do_restore "" "" || true; pause_tt ;; 18) screen_bootkeys ;; 19) screen_tools ;;
    esac
  done
}
wizard() {
  for s in detect analyze firmware extract patch backup flash verify; do "screen_$s"; done
}

# ---------------------------------------------------------------- CLI
show_help() {
  printf 'Huawei P10 Root Manager v%s\n' "$TTVERSION"
  printf 'Usage: treble-toolkit.sh [detect|devices|analyze|firmware|download|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|validate|verify|restore|diagnostic|dump-partitions|dump-properties|dump-vendor|dump-logs|wizard|help] [--mode safe|unattended|developer] [--json] [--yes] [--image <path>] [--firmware-file <url|path>] [--anonymize] [--no-reboot]\n'
  printf '%s\n' "$(L 'No args: TUI. Download/flash/restore need --yes.' 'Ohne Args: TUI. Download/Flash/Restore brauchen --yes.')"
}
CMD=""; JSON=""; YES=""; IMAGE=""; FWFILE=""; ANON=""; NOREBOOT=""; RUNMODE="safe"
for a in "$@"; do
  case "$a" in
    detect|devices|analyze|firmware|download|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|validate|verify|restore|diagnostic|dump-partitions|dump-properties|dump-vendor|dump-logs|wizard|help) [ -z "$CMD" ] && CMD="$a" ;;
    --json) JSON=1 ;; --yes) YES=1 ;; --anonymize) ANON=1 ;; --no-reboot) NOREBOOT=1 ;;
    --mode) WANTVAL="--mode" ;;
    --image|--firmware-file) WANTVAL="$a" ;;
    *) if [ "${WANTVAL:-}" = "--image" ]; then IMAGE="$a"; WANTVAL=""; elif [ "${WANTVAL:-}" = "--firmware-file" ]; then FWFILE="$a"; WANTVAL=""; elif [ "${WANTVAL:-}" = "--mode" ]; then RUNMODE_REQ="$a"; WANTVAL=""; fi ;;
  esac
done
RUNMODE="$(resolve_mode "$RUNMODE_REQ")"
[ -z "$CMD" ] && { main_menu; exit 0; }
find_tools; detect_mode
case "$CMD" in
  help) show_help ;;
  devices)
    if [ -n "$JSON" ]; then
      printf '[{"id":"VTR-L29","verified":true},{"id":"VTR-L09","verified":true},{"id":"VKY-L29","verified":true},{"id":"VTR-AL00","verified":false},{"id":"VKY-L09","verified":false},{"id":"GENERIC-TREBLE","verified":false}]\n'
    else
      for p in VTR-L29 VTR-L09 VKY-L29 VTR-AL00 VKY-L09 GENERIC-TREBLE; do
        PROFILE_ID="$p"; printf ' - %s (%s) verified=%s target=%s\n' "$p" "$(marketing_name)" "$(profile_verified)" "$(target_partition)"
      done
      PROFILE_ID="VTR-L29"
    fi ;;
  detect)
    if [ -n "$JSON" ]; then printf '{"mode":"%s","adb":"%s","fastboot":"%s","scrcpy":"%s"}\n' "$MODE" "$ADB_SERIAL" "$FB_SERIAL" "${SCRCPY_BIN:-}"
    else printf 'mode: %s\nadb: %s\nfastboot: %s\nscrcpy: %s\n' "$MODE" "$ADB_SERIAL" "$FB_SERIAL" "${SCRCPY_BIN:-not found (optional)}"; fi ;;
  analyze)
    [ "$MODE" = "android" ] && android_analysis
    [ "$MODE" = "fastboot" ] && fastboot_analysis || true
    if [ -n "$JSON" ]; then printf '{"mode":"%s","os":"%s","detail":"%s"}\n' "$MODE" "$OS_KIND" "$OS_DETAIL"
    else printf 'OS: %s | %s\n' "$OS_KIND" "$OS_DETAIL"; fi ;;
  firmware)
    [ -n "$FWFILE" ] && FW_BASELINE="$FWFILE"
    [ -z "$FW_BASELINE" ] && [ "$MODE" = "android" ] && { android_analysis; FW_BASELINE="$(grep '^ro.build.display.id=' "$PROPS_FILE" | cut -d= -f2-)"; }
    out="$(firmware_compat "$PROFILE_ID" "$FW_BASELINE" "")"
    FW_STATUS="${out%%|*}"
    if [ -n "$JSON" ]; then printf '{"baseline":"%s","compat":"%s"}\n' "$FW_BASELINE" "$FW_STATUS"; else printf '%s [%s]\n' "$FW_BASELINE" "$FW_STATUS"; fi ;;
  download)
    url="${FWFILE:-$IMAGE}"
    [ -z "$url" ] && { printf 'URL missing. Example: treble-toolkit.sh download --firmware-file <https-URL> [--yes]\n'; exit 4; }
    valid_url "$url" || { printf 'URL rejected.\n'; exit 4; }
    out="$FIRM_DIR/stock-firmware-$PROFILE_ID-$STAMP.zip"
    if [ -z "$YES" ]; then printf 'Ready (not loaded): %s -> %s | confirm with --yes.\n' "$url" "$out"; exit 4; fi
    if download_firmware "$url" "$out" --yes && verify_download "$out"; then [ -n "$JSON" ] && printf '{"path":"%s"}\n' "$out"; else exit 1; fi ;;
  extract)
    found="$(find "$FIRM_DIR" "$DATA_DIR" -maxdepth 3 \( -iname 'RECOVERY_RAMDISK.img' -o -iname 'RECOVERY_RAMDIS.img' -o -iname 'recovery_ramdisk.img' \) -type f 2>/dev/null | head -1)"
    [ -z "$found" ] && { printf '%s\n' "$(L 'No RECOVERY_RAMDIS(K).img found.' 'Keine RECOVERY_RAMDIS(K).img.')"; exit 3; }
    t="$(test_image "$found")"; STOCK_IMAGE="$found"; STOCK_SHA="$(printf '%s' "$t" | cut -d'|' -f2)"
    if [ -n "$JSON" ]; then printf '{"path":"%s","check":"%s"}\n' "$found" "$t"; else printf '%s: %s\n' "$found" "$t"; fi
    [ "${t%%|*}" = "PASS" ] || exit 3 ;;
  export)
    rom="${IMAGE:-$FWFILE}"
    if [ -z "$rom" ]; then rom="$(find "$ROM_DIR" -maxdepth 1 \( -iname '*.zip' -o -iname '*.img' \) -type f 2>/dev/null | head -1)"; fi
    [ -z "$rom" ] && { printf '%s\n' "$(L 'No ROM package. Place .zip/.img in data/roms/.' 'Kein ROM-Paket. .zip/.img nach data/roms/.')"; exit 3; }
    d="" || true
    if d="$(export_recovery "$rom")"; then
      if [ -n "$JSON" ]; then printf '{"dir":"%s"}\n' "$d"; else printf 'Export dir: %s\n' "$d"; fi
    else exit 3; fi ;;
  patch)
    stock="${IMAGE:-$(find "$FIRM_DIR" "$DATA_DIR" -maxdepth 3 \( -iname 'RECOVERY_RAMDISK.img' -o -iname 'RECOVERY_RAMDIS.img' \) -type f 2>/dev/null | head -1)}"
    [ -f "$stock" ] || { printf '%s\n' "$(L "No stock image. Run 'extract' first." "Kein Stock-Image. Erst 'extract'.")"; exit 3; }
    t="$(test_image "$stock")"; [ "${t%%|*}" = "PASS" ] || { printf 'Stock FAIL: %s\n' "$t"; exit 3; }
    STOCK_IMAGE="$stock"; STOCK_SHA="$(printf '%s' "$t" | cut -d'|' -f2)"
    dest="$(prepare_patch "$stock")"
    if [ -n "$JSON" ]; then printf '{"staged":"%s"}\n' "$dest"; else printf 'Staged: %s\n%s\n' "$dest" "$(L 'Next: patch on device, then register patched file in TUI.' 'Weiter: am Geraet patchen, dann in TUI registrieren.')"; fi ;;
  backup)
    stock="${IMAGE:-$(find "$FIRM_DIR" "$DATA_DIR" -maxdepth 3 \( -iname 'RECOVERY_RAMDISK.img' -o -iname 'RECOVERY_RAMDIS.img' \) -type f 2>/dev/null | head -1)}"
    [ -f "$stock" ] || { printf 'No stock image.\n'; exit 3; }
    t="$(test_image "$stock")"; [ "${t%%|*}" = "PASS" ] || { printf 'Stock FAIL.\n'; exit 3; }
    STOCK_IMAGE="$stock"; STOCK_SHA="$(printf '%s' "$t" | cut -d'|' -f2)"
    d="$(do_backup "$stock")" && { if [ -n "$JSON" ]; then printf '{"backup":"%s"}\n' "$d"; else printf 'Backup: %s\n' "$d"; fi; } || exit 3 ;;
  flash)
    [ -n "$IMAGE" ] && { PATCHED_IMAGE="$IMAGE"; t="$(test_image "$IMAGE")"; PATCHED_SHA="$(printf '%s' "$t" | cut -d'|' -f2)"; STOCK_SHA="${STOCK_SHA:-x}"; }
    if [ -n "$YES" ]; then safe_flash "$PATCHED_IMAGE" --yes || exit 1; else safe_flash "$PATCHED_IMAGE" || exit 1; fi ;;
  flash-system)
    [ -z "$IMAGE" ] || [ ! -f "$IMAGE" ] && { printf '%s\n' "$(L 'GSI image path missing. Use --image <path>.' 'GSI-Pfad fehlt. Nutze --image <Pfad>.')"; exit 3; }
    if [ -n "$YES" ]; then system_flash "$IMAGE" --yes || exit 1; else system_flash "$IMAGE" || exit 1; fi ;;
  twrp)
    [ -z "$IMAGE" ] || [ ! -f "$IMAGE" ] && { printf '%s\n' "$(L 'TWRP image path missing. Use --image <path>.' 'TWRP-Pfad fehlt. Nutze --image <Pfad>.')"; exit 3; }
    if [ -n "$YES" ]; then twrp_flash "$IMAGE" --yes || exit 1; else twrp_flash "$IMAGE" || exit 1; fi ;;
  root-methods)
    if [ -n "$JSON" ]; then printf '[{"id":"magisk-recovery","preferred":true},{"id":"magisk-twrp","preferred":false},{"id":"phh-su","preferred":false},{"id":"kernelsu","preferred":false}]\n'
    else for m in $(root_method_ids); do printf ' - %s: %s\n' "$m" "$(root_method_name "$m")"; done; fi ;;
  compat)
    if roms="$(compat_roms)"; then
      if [ -n "$JSON" ]; then
        if command -v python3 >/dev/null 2>&1; then python3 -c "import json,sys; print(json.dumps(json.load(open(sys.argv[1]))))" "$(compat_file)"
        else printf '%s\n' "$roms"; fi
      else
        printf 'Recommended:\n'; printf '%s\n' "$roms" | grep -E '\|working' | sed 's/^/ [+] /'
        printf 'Not recommended:\n'; printf '%s\n' "$roms" | grep -v -E '\|working' | sed 's/^/ [X] /'
      fi
    else printf '%s\n' "$(L 'No registry for this profile.' 'Keine Registry.')"; exit 3; fi ;;
  verify)
    if [ -n "$NOREBOOT" ]; then verify_root --no-reboot; else verify_root; fi
    rc=$?
    [ -n "$JSON" ] && printf '{"rc":%s}\n' "$rc"
    exit "$rc" ;;
  validate)
    rep="$(validate_checked)"; rc=$?
    if [ -n "$JSON" ]; then printf '{"ok":%s}\n' "$([ "$rc" = 0 ] && printf true || printf false)"; else printf '%s\n' "$rep"; fi
    if [ "$rc" = 0 ]; then exit 0; else exit 2; fi ;;
  dump-partitions|dump-properties|dump-vendor|dump-logs)
    developer_dump "$CMD" || exit 1 ;;
  restore)
    if [ -n "$YES" ]; then do_restore "$IMAGE" --yes || exit 1; else do_restore "$IMAGE" "" || exit 1; fi ;;
  diagnostic)
    if [ -n "$ANON" ]; then z="$(do_diagnostic --anonymize)"; else z="$(do_diagnostic)"; fi
    if [ -n "$JSON" ]; then printf '{"zip":"%s"}\n' "$z"; else printf 'ZIP: %s\n' "$z"; fi ;;
  wizard) main_menu ;;
esac
