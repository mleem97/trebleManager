#!/usr/bin/env bash
# trebleManager - Linux bash port (Windows PowerShell TUI parity)
# Huawei P10 (VTR-L29 and others): detect, analyze, firmware, download,
# extract, export, patch, backup, flash, verify, restore, diagnostic.
# No extra dependencies: only adb, fastboot, and coreutils. Optional:
# unzip or python3 (zip listing), curl or wget (download), zip or python3 (diagnostic).
# Repo language: English. TUI German if $LANG starts with de.
set -u

TTVERSION="2.22.0"
# Run modes: safe (confirm everything), unattended (--yes auto-confirms, gates
# still enforced), developer (unlocks dump-* commands).
RUNMODE_REQ=""
resolve_mode() { # pure: safe|unattended|developer -> itself, else safe
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in unattended|developer) printf '%s' "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" ;; *) printf 'safe' ;; esac
}
TTLANG="en"
case "${LANG:-en}" in de*) TTLANG="de" ;; esac

# ---------------------------------------------------------------- paths/state
# Works from file, pipe (curl|bash) or process substitution: BASH_SOURCE may be
# unset/empty there, so fall back safely (set -u is active).
_BSSRC="${BASH_SOURCE[0]:-}"
if [ -n "$_BSSRC" ] && [ -f "$_BSSRC" ]; then
  SCRIPT_DIR="$(cd "$(dirname "$_BSSRC")" && pwd)"
else
  SCRIPT_DIR="$(pwd)"
fi
TOOL_ROOT="$(dirname "$SCRIPT_DIR")"
if [ "$(basename "$TOOL_ROOT")" = "scripts" ]; then TOOL_ROOT="$(dirname "$TOOL_ROOT")"; fi
# Remote-run fallback: current directory layout
if [ ! -d "$TOOL_ROOT/data" ]; then TOOL_ROOT="$(pwd)"; fi
# Self-bootstrap: single-file/pipe run without layout -> fetch the FULL release
# ZIP (same trust root: this repo) and re-exec from it. Full run, not degraded.
# Tag order: own script version first (no network guesswork), API last
# (rate-limited). Guard TT_BOOTSTRAPPED=1 prevents loops. Offline -> degraded.
bootstrap_fetch() { # tag relpath -> prints dest file path, rc 0/1 (quiet; errors to stderr)
  local _tag="$1" _rel="$2" _base _dest _file _zip _exp _act
  _base="${XDG_DATA_HOME:-$HOME/.local/share}/trebleManager"
  mkdir -p "$_base" 2>/dev/null || return 1
  _dest="$_base/$_tag"; _file="$_dest/$_rel"
  if [ ! -f "$_file" ]; then
    _zip="$_base/trebleManager-$_tag.zip"
    curl -fsSL -o "$_zip" "https://github.com/mleem97/trebleManager/releases/download/$_tag/trebleManager-$_tag.zip" 2>/dev/null || return 1
    if curl -fsSL -o "$_zip.sha256" "https://github.com/mleem97/trebleManager/releases/download/$_tag/trebleManager-$_tag.zip.sha256" 2>/dev/null; then
      _exp="$(awk '{print $1}' "$_zip.sha256" 2>/dev/null | tr 'a-z' 'A-Z')"
      _act="$(sha256sum "$_zip" 2>/dev/null | awk '{print $1}' | tr 'a-z' 'A-Z')"
      if [ -z "$_act" ]; then _act="$(shasum -a 256 "$_zip" 2>/dev/null | awk '{print $1}' | tr 'a-z' 'A-Z')"; fi
      if [ -n "$_exp" ] && [ "$_exp" != "$_act" ]; then
        echo "ERROR: SHA256 MISMATCH - deleted, aborting bootstrap." >&2; rm -f "$_zip"; return 1
      fi
      echo "SHA256 OK." >&2
    else echo "WARN: no .sha256 asset, skipping verify." >&2; fi
    mkdir -p "$_dest" 2>/dev/null || return 1
    if command -v unzip >/dev/null 2>&1; then unzip -qo "$_zip" -d "$_dest" 2>/dev/null || return 1
    else python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "$_zip" "$_dest" 2>/dev/null || return 1; fi
  fi
  [ -f "$_file" ] || return 1
  printf '%s' "$_file"
}
if [ -z "${TT_BOOTSTRAPPED:-}" ] && [ ! -d "$TOOL_ROOT/data/compatibility" ]; then
  echo "Single-file run detected - fetching full toolkit layout ..."
  echo "Einzeldatei erkannt - lade volles Toolkit-Layout ..."
  _bf="$(bootstrap_fetch "v$TTVERSION" "scripts/treble-toolkit.sh" 2>/dev/null)"
  if [ -z "$_bf" ]; then
    _atag="$(curl -fsSL https://api.github.com/repos/mleem97/trebleManager/releases/latest 2>/dev/null | grep -m1 '"tag_name"' | cut -d'"' -f4)"
    [ -n "$_atag" ] && _bf="$(bootstrap_fetch "$_atag" "scripts/treble-toolkit.sh" 2>/dev/null)"
  fi
  if [ -n "$_bf" ]; then
    export TT_BOOTSTRAPPED=1
    chmod +x "$_bf" 2>/dev/null
    exec "$_bf" "$@"
  fi
  echo "WARN: bootstrap failed (offline?) - continuing degraded without registry."
  unset _bf _atag
fi
# Config: repo data/config.json if it looks like a checkout, else user config.
if [ -d "$TOOL_ROOT/scripts" ]; then CONFIG_FILE="$TOOL_ROOT/data/config.json"
else CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/trebleManager/config.json"; fi
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
flash_verdict() { # fastboot flash/erase output on stdin -> clear summary on stdout
  # rc 0=OK 1=FAILED 2=UNCLEAR. FAILED lines veto everything: progress words
  # like Writing/Erasing alone are NOT success. OK needs OKAY + Finished.
  # Bilingual fixed text (no LANG dependency, testable pure logic).
  local okay=0 first_failed="" total="" line
  while IFS= read -r line; do
    case "$line" in *OKAY*) okay=$((okay+1)) ;; esac
    if printf '%s' "$line" | grep -qi -E 'FAILED|remote:|^[[:space:]]*error'; then
      [ -z "$first_failed" ] && first_failed="$line"
    fi
    case "$line" in *[Ff]inished*) total="$(printf '%s' "$line" | sed -n 's/.*[Tt]otal time:[[:space:]]*//p')" ;; esac
  done
  if [ -n "$first_failed" ]; then
    printf 'FLASH RESULT: FAILED - nothing claimed as done.\n'
    printf ' ! %s\n' "$first_failed"
    printf 'Hints / Hinweise: Command not allowed = Huawei refused (retry, cable, TROUBLESHOOTING). too large = image bigger than partition. Full output is in the log.\n'
    return 1
  fi
  if [ "$okay" -gt 0 ] && [ -n "$total" ]; then
    printf 'FLASH RESULT: OK (%s OKAY, %s).\n' "$okay" "$total"
    return 0
  fi
  printf 'FLASH RESULT: UNCLEAR - no FAILED, but no Finished either. Verify manually before rebooting.\n'
  printf 'FLASH-ERGEBNIS: UNKLAR - kein FAILED, aber auch kein Finished. Vor Reboot manuell verifizieren.\n'
  return 2
}
iread() { if [ -c /dev/tty ] 2>/dev/null; then builtin read "$@" </dev/tty; else builtin read "$@"; fi; }
find_tools() {
  # Saved setup config first (repo data/config.json or user config).
  for cfg in "$TOOL_ROOT/data/config.json" "$CONFIG_FILE"; do
    if [ -f "$cfg" ]; then
      cfg_adb="$(sed -n 's/.*"adb"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$cfg" | head -1)"
      cfg_fb="$(sed -n 's/.*"fastboot"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$cfg" | head -1)"
      cfg_scr="$(sed -n 's/.*"scrcpy"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$cfg" | head -1)"
      [ -n "$cfg_adb" ] && [ -x "$cfg_adb" ] && ADB_BIN="$cfg_adb"
      [ -n "$cfg_fb" ] && [ -x "$cfg_fb" ] && FB_BIN="$cfg_fb"
      [ -n "$cfg_scr" ] && [ -x "$cfg_scr" ] && SCRCPY_BIN="$cfg_scr"
    fi
  done
  [ -z "$ADB_BIN" ] && ADB_BIN="$(command -v adb 2>/dev/null || true)"
  [ -z "$FB_BIN" ] && FB_BIN="$(command -v fastboot 2>/dev/null || true)"
  [ -z "${SCRCPY_BIN:-}" ] && SCRCPY_BIN="$(command -v scrcpy 2>/dev/null || true)"
  if [ -n "$ADB_BIN" ]; then log SUCCESS "ADB: $ADB_BIN"; else log ERROR "$(L 'ADB not found (install android-tools / platform-tools).' 'ADB nicht gefunden (android-tools / platform-tools installieren).')"; fi
  if [ -n "$FB_BIN" ]; then log SUCCESS "Fastboot: $FB_BIN"; else log WARNING "$(L 'Fastboot not found.' 'Fastboot nicht gefunden.')"; fi
  # scrcpy is optional (screen mirror during rooting), never required
  if [ -n "$SCRCPY_BIN" ]; then
    log SUCCESS "scrcpy: $SCRCPY_BIN ($("$SCRCPY_BIN" --version 2>&1 | head -1))"
  else
    log INFO "$(L 'scrcpy not found (optional, screen mirror only: https://github.com/Genymobile/scrcpy).' 'scrcpy nicht gefunden (optional, nur Screen-Mirror: https://github.com/Genymobile/scrcpy).')"
  fi
}

save_config() { # persist resolved tool paths (repo config or user config)
  local f="$TOOL_ROOT/data/config.json"
  mkdir -p "$(dirname "$f")" 2>/dev/null || f="$CONFIG_FILE"
  mkdir -p "$(dirname "$f")" 2>/dev/null || return 0
  printf '{"adb":"%s","fastboot":"%s","scrcpy":"%s","updated":"%s"}\n' \
    "${ADB_BIN:-}" "${FB_BIN:-}" "${SCRCPY_BIN:-}" "$(date '+%Y-%m-%d %H:%M:%S')" > "$f" 2>/dev/null || true
  log SUCCESS "$(L 'Config saved: ' 'Config gespeichert: ')$f"
}

# ------------------------------------------------- installed ROM (what is ON the phone)
INSTALLED_ROM=""
rom_file() { printf '%s/data/installed-rom.txt' "$TOOL_ROOT"; }
save_rom() { # id -> persist (shared file with the PowerShell side)
  mkdir -p "$(dirname "$(rom_file)")" 2>/dev/null
  printf '%s' "$1" > "$(rom_file)" 2>/dev/null
  INSTALLED_ROM="$1"
}
load_rom() {
  local f; f="$(rom_file)"
  if [ -f "$f" ]; then INSTALLED_ROM="$(tr -d ' \r\n' < "$f")"; fi
}
rom_suggest() { # pure: os-kind -> stock|empty (never guesses a custom ROM)
  case "$1" in *Stock*|*EMUI*) printf 'stock' ;; *) printf '' ;; esac
}
rom_label() { # pure: id -> display name
  case "$1" in
    stock) printf 'Stock EMUI' ;;
    other) printf 'Other custom ROM' ;;
    rom:*) printf '%s' "${1#rom:}" ;;
    "") printf '?' ;;
    *) printf '%s' "$1" ;;
  esac
}
rom_options() { # prints id|label lines: stock, supported registry ROMs, other
  printf 'stock|Stock EMUI (Huawei original)\n'
  local f; f="$(compat_file)"
  if [ -f "$f" ] && command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
seen=set()
for r in json.load(open(sys.argv[1])).get('roms',[]):
    st=r.get('status','')
    if st not in ('working','working-slim','working-with-fixes','variant-dependent'): continue
    nm=str(r.get('name','')).strip()
    if not nm: continue
    ver=str(r.get('version',r.get('android',r.get('build',''))))
    label=nm+(' '+ver if ver else '')
    variant=str(r.get('variant',''))
    if variant and variant not in label: label+=' '+variant
    bld=str(r.get('build',''))
    if bld and bld not in label: label+=' ('+bld+')'
    if label not in seen:
        seen.add(label); print('rom:'+label+'|'+label)
" "$f" 2>/dev/null
  fi
  printf 'other|Other custom ROM (not in list)\n'
}
rom_broken() { # prints name - reason for researched-broken ROMs (info only)
  local f; f="$(compat_file)"
  [ -f "$f" ] || return 0
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
for r in json.load(open(sys.argv[1])).get('roms',[]):
    if r.get('status')=='broken':
        print(str(r.get('name','?'))+' - '+str(r.get('reason','')))
" "$f" 2>/dev/null
  fi
  return 0
}
select_rom() { # THE question: which system is on the phone. Plain words, persisted.
  header "$(L 'Which system is on your phone right now?' 'Welches System ist gerade auf deinem Handy?')"; printf '\n'
  printf '%s\n' "$(L 'This decides everything: patch source, skipped steps, blocked ROMs.' 'Das entscheidet alles: Patch-Quelle, uebersprungene Steps, blockierte ROMs.')"
  local i=1 line id label mark sug
  ROM_OPTS="$(rom_options)"
  sug="$(rom_suggest "$OS_KIND")"
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    id="${line%%|*}"; label="${line##*|}"
    mark=""
    if [ "$id" = "$INSTALLED_ROM" ]; then mark="  <-- $(L 'current, Enter keeps it' 'aktuell, Enter behaelt es')"
    elif [ -z "$INSTALLED_ROM" ] && [ -n "$sug" ] && [ "$id" = "$sug" ]; then mark="  <-- $(L 'detected, Enter selects it' 'erkannt, Enter waehlt es')"; fi
    printf ' [%d] %s%s\n' "$i" "$label" "$mark"
    i=$((i+1))
  done <<EOF
$ROM_OPTS
EOF
  local bl; bl="$(rom_broken)"
  if [ -n "$bl" ]; then
    printf '\n%s\n' "$(L 'NOT supported (researched broken - cannot be selected):' 'NICHT supported (nachweislich kaputt - nicht waehlbar):')"
    printf '%s\n' "$bl" | while IFS= read -r line; do printf '  X %s\n' "$line"; done
  fi
  printf '\n%s' "$(L 'Number + Enter (Enter = keep): ' 'Nummer + Enter (Enter = behalten): ')"; iread -r s
  case "$s" in
    "") [ -z "$INSTALLED_ROM" ] && INSTALLED_ROM="$sug" ;;
    *[!0-9]*) ;;
    *) id="$(printf '%s' "$ROM_OPTS" | sed -n "${s}p" | cut -d'|' -f1)"
       [ -n "$id" ] && INSTALLED_ROM="$id" ;;
  esac
  save_rom "$INSTALLED_ROM"
  printf '\n%s %s\n' "$(L 'Phone runs:' 'Handy laeuft mit:')" "$(rom_label "$INSTALLED_ROM")"
  if [ -z "$INSTALLED_ROM" ] || [ "$INSTALLED_ROM" = "stock" ]; then
    printf '%s\n' "$(L 'Rule: patch base = stock UPDATE.APP recovery image. Nothing else.' 'Regel: Patch-Basis = Stock-UPDATE.APP-Recovery. Nichts anderes.')"
  elif [ -n "$(rom_entry_gsi "$INSTALLED_ROM" "$(compat_file)")" ]; then
    printf '%s\n' "$(L 'GSI (system-only): your recovery is untouched stock, so the patch base IS the stock recovery. Correct, not a workaround.' 'GSI (nur System): dein Recovery ist unberuehrt Stock, also ist die Patch-Basis das Stock-Recovery. Korrekt, kein Workaround.')"
  else
    printf '%s\n' "$(L 'RULE: your Magisk patch file MUST come from this ROM package.' 'REGEL: Deine Magisk-Patch-Datei MUSS aus diesem ROM-Paket kommen.')"
    printf '%s\n' "$(L 'NOT from stock firmware. A stock-based patched image will NOT boot on this ROM.' 'NICHT aus der Stock-Firmware. Ein Stock-basiertes Image bootet auf diesem ROM NICHT.')"
  fi
}
rom_base_image() { # newest .img under data/recovery (export output), or empty
  local d="$DATA_DIR/recovery" best="" bt=0 f t
  [ -d "$d" ] || return 0
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    t="$(stat -c%Y "$f" 2>/dev/null || stat -f%m "$f" 2>/dev/null || printf 0)"
    if [ "$t" -ge "$bt" ]; then bt="$t"; best="$f"; fi
  done < <(find "$d" -type f -name '*.img' 2>/dev/null)
  [ -n "$best" ] && printf '%s' "$best"
  return 0
}
patch_base() { # single source of truth: prints source|image|label
  # GSI ROMs (system-only, registry gsi field) use STOCK recovery: a GSI never
  # touches recovery_ramdisk. Full device ROMs use their exported package image.
  if [ -z "$INSTALLED_ROM" ] || [ "$INSTALLED_ROM" = "stock" ]; then
    printf 'stock|%s|Stock EMUI\n' "$STOCK_IMAGE"
  elif [ -n "$(rom_entry_gsi "$INSTALLED_ROM" "$(compat_file)")" ]; then
    printf 'stock-gsi|%s|%s\n' "$STOCK_IMAGE" "$(rom_label "$INSTALLED_ROM")"
  else
    printf 'rom|%s|%s\n' "$(rom_base_image)" "$(rom_label "$INSTALLED_ROM")"
  fi
}

# ------------------------------------------- target image resolver

# ------------------------------------------- target image resolver
# Device -> install type -> Android -> system -> variant -> full config.
# Only Android versions with real registry images are listed (with counts).
target_androids() { # prints android|count lines, sorted
  local f; f="$(compat_file)"
  [ -f "$f" ] || return 0
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
from collections import Counter
c=Counter()
for r in json.load(open(sys.argv[1])).get('roms',[]):
    if str(r.get('status','')) not in ('working','working-slim','working-with-fixes'): continue
    try: a=int(r.get('android',0))
    except Exception: continue
    if a>0: c[a]+=1
for a in sorted(c): print('%d|%d' % (a,c[a]))
" "$f" 2>/dev/null
  fi
  return 0
}
resolver_entries() { # [android] [label] -> n|label|variant|build|gsi|url|file|status|root_type|root_source
  local want_a="${1:-}" want_l="${2:-}" f
  f="$(compat_file)"
  [ -f "$f" ] || return 0
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
want_a=sys.argv[2]; want_l=sys.argv[3]
n=0
for r in json.load(open(sys.argv[1])).get('roms',[]):
    st=str(r.get('status',''))
    if st not in ('working','working-slim','working-with-fixes'): continue
    try: a=int(r.get('android',0))
    except Exception: continue
    if want_a and str(a)!=want_a: continue
    nm=str(r.get('name','')).strip()
    if not nm: continue
    ver=str(r.get('version',r.get('android',r.get('build',''))))
    label=nm+(' '+ver if ver else '')
    variant=str(r.get('variant',''))
    if variant and variant not in label: label+=' '+variant
    bld=str(r.get('build',''))
    if bld and bld not in label: label+=' ('+bld+')'
    if want_l and label!=want_l: continue
    ra=r.get('root_artifact',{}) or {}
    n+=1
    print('%d|%s|%s|%s|%s|%s|%s|%s|%s|%s' % (n,label,variant,bld,str(r.get('gsi','')),str(r.get('url','')),str(r.get('file','')),st,str(ra.get('type','recovery_ramdisk')),str(ra.get('source','stock_firmware'))))
" "$f" "$want_a" "$want_l" 2>/dev/null
  fi
  return 0
}
select_target_image() { # sets TARGET_* globals; returns 1 on abort
  header "$(L 'Target system resolver (device -> Android -> system -> variant -> config)' 'Zielsystem-Resolver (Geraet -> Android -> System -> Variante -> Config)')"; printf '\n'
  printf '%s Huawei %s\n' "$(L 'Device:' 'Geraet:')" "$PROFILE_ID"
  local avs; avs="$(target_androids)"
  if [ -z "$avs" ]; then printf '%s\n' "$(L 'No working images in registry.' 'Keine working Images in Registry.')"; return 1; fi
  printf '\n%s\n' "$(L 'Which Android version should be installed?' 'Welche Android-Version soll installiert werden?')"
  printf '%s\n' "$avs" | while IFS='|' read -r a c; do printf ' [%s] Android %s   (%s %s)\n' "$a" "$a" "$c" "$(L 'images' 'Images')"; done
  printf '%s' "$(L 'Number (Enter=abort): ' 'Nummer (Enter=Abbruch): ')"; iread -r an
  local android; android="$(printf '%s' "$avs" | sed -n "${an}p" 2>/dev/null | cut -d'|' -f1)"
  [ -n "$android" ] || return 1
  local syslist; syslist="$(resolver_entries "$android" | cut -d'|' -f1,2 | awk -F'|' '!seen[$2]++')"
  header "$(L 'Systems for Android' 'Systeme fuer Android') $android"; printf '\n'
  local nsys; nsys="$(printf '%s' "$syslist" | grep -c .)"
  local label
  if [ "$nsys" = "1" ]; then
    label="$(printf '%s' "$syslist" | cut -d'|' -f2)"
    printf '%s %s\n' "$(L 'Only one:' 'Nur eins:')" "$label"
  else
    printf '%s\n' "$syslist" | while IFS='|' read -r i l; do printf ' [%s] %s\n' "$i" "$l"; done
    printf '%s' "$(L 'Number (Enter=abort): ' 'Nummer (Enter=Abbruch): ')"; iread -r sn
    label="$(printf '%s' "$syslist" | sed -n "${sn}p" 2>/dev/null | cut -d'|' -f2-)"
    [ -n "$label" ] || return 1
  fi
  local variants; variants="$(resolver_entries "$android" "$label")"
  local nvar; nvar="$(printf '%s' "$variants" | grep -c .)"
  local entry
  if [ "$nvar" = "1" ]; then entry="$variants"
  else
    header "$(L 'Variant for' 'Variante fuer') $label"; printf '\n'
    printf '%s\n' "$variants" | while IFS='|' read -r i l v b rest; do
      [ -n "$v" ] && printf ' [%s] %s\n' "$i" "$v" || printf ' [%s] %s\n' "$i" "${b:-$l}"
    done
    printf '%s' "$(L 'Number (Enter=abort): ' 'Nummer (Enter=Abbruch): ')"; iread -r vn
    entry="$(printf '%s' "$variants" | sed -n "${vn}p" 2>/dev/null)"
    [ -n "$entry" ] || return 1
  fi
  TARGET_ANDROID="$android"
  TARGET_LABEL="$(printf '%s' "$entry" | cut -d'|' -f2)"
  TARGET_VARIANT="$(printf '%s' "$entry" | cut -d'|' -f3)"
  TARGET_BUILD="$(printf '%s' "$entry" | cut -d'|' -f4)"
  TARGET_GSI="$(printf '%s' "$entry" | cut -d'|' -f5)"
  TARGET_URL="$(printf '%s' "$entry" | cut -d'|' -f6)"
  TARGET_FILE="$(printf '%s' "$entry" | cut -d'|' -f7)"
  TARGET_STATUS="$(printf '%s' "$entry" | cut -d'|' -f8)"
  TARGET_ROOT_TYPE="$(printf '%s' "$entry" | cut -d'|' -f9)"
  TARGET_ROOT_SOURCE="$(printf '%s' "$entry" | cut -d'|' -f10)"
  TARGET_FWBASE="$(compat_firmware_base)"
  header "$(L 'Resolved target configuration' 'Aufgeloeste Ziel-Config')"; printf '\n'
  printf ' Device   : Huawei %s\n Android  : %s\n System   : %s\n Base     : %s\n Vendor   : Stock %s vendor\n Recovery : RECOVERY_RAMDIS(K).img from UPDATE.APP (%s)\n Root     : Magisk / %s (%s)\n' \
    "$PROFILE_ID" "$TARGET_ANDROID" "$TARGET_LABEL" "$TARGET_FWBASE" "$TARGET_FWBASE" "$TARGET_FWBASE" "$TARGET_ROOT_TYPE" "$TARGET_ROOT_SOURCE"
  if [ -n "$TARGET_FILE" ]; then printf ' System   : %s\n' "$TARGET_FILE"; fi
  if [ -n "$TARGET_URL" ]; then printf ' Download : %s\n' "$TARGET_URL"
  else printf '%s\n' "$(L ' Download : no verified direct link (manual package into data/roms/).' ' Download : kein gepruefter Direktlink (Paket manuell nach data/roms/).')"; fi
  return 0
}
compat_firmware_base() { # registry required_base (honest advisory text lives in screens)
  local f; f="$(compat_file)"
  if [ -f "$f" ] && command -v python3 >/dev/null 2>&1; then
    python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('firmware',{}).get('required_base',''))" "$f" 2>/dev/null
  fi
}
select_root_target() { # root needs an explicit target too; sets INSTALLED_ROM; 1=abort→return 1
  header "$(L 'Root target: which system stays on the phone?' 'Root-Ziel: welches System bleibt auf dem Handy?')"; printf '\n'
  local cur; cur="$(rom_label "$INSTALLED_ROM")"
  [ -z "$INSTALLED_ROM" ] && cur="$(L '(unknown - analyze first)' '(unbekannt - erst analysieren)')"
  printf '%s\n' "[1] $(L 'Current system:' 'Aktuelles System:') $cur"
  printf '%s\n' "[2] $(L 'Stock EMUI (choose version)' 'Stock-EMUI (Version waehlen)')"
  printf '%s\n' "[3] $(L 'Other system / ROM (choose Android -> ROM -> variant)' 'Anderes System / ROM (Android -> ROM -> Variante waehlen)')"
  printf '%s' "$(L '[Enter] back: ' '[Enter] zurueck: ')"; iread -r k
  case "$k" in
    1) if [ -z "$INSTALLED_ROM" ]; then select_rom; fi
       [ -z "$INSTALLED_ROM" ] && return 1
       local pb; pb="$(patch_base)"; printf '\n%s %s -> root artifact: %s\n' "$(L 'Target kept:' 'Ziel bleibt:')" "$(rom_label "$INSTALLED_ROM")" "${pb%%|*}"
       return 0 ;;
    2) header "$(L 'Which Stock EMUI version?' 'Welche Stock-EMUI-Version?')"; printf '\n[1] Android 8 / EMUI 8\n[2] Android 9 / EMUI 9.0\n[3] Android 9 / EMUI 9.1\n'
       printf '%s' "$(L '[Enter] back: ' '[Enter] zurueck: ')"; iread -r k2
       case "$k2" in 1) em="EMUI 8 (Android 8)";; 2) em="EMUI 9.0 (Android 9)";; 3) em="EMUI 9.1 (Android 9)";; *) return 1;; esac
       save_rom "stock"
       printf '\n%s Stock %s\n' "$(L 'Target:' 'Ziel:')" "$em"
       printf '%s %s\n' "$(L 'Required base:' 'Benoetigte Basis:')" "$(compat_firmware_base)"
       printf '%s\n' "$(L 'Firmware portals are gated (login/pack): place the full UPDATE.APP/ZIP into data/firmware/ or use the firmware downloader.' 'Firmware-Portale sind gated (Login/Paket): Full-UPDATE.APP/ZIP nach data/firmware/ legen oder Firmware-Downloader nutzen.')"
       return 0 ;;
    3) TARGET_ANDROID=""; TARGET_LABEL=""; TARGET_URL=""; TARGET_FILE=""
       select_target_image || return 1
       save_rom "rom:$TARGET_LABEL"
       printf '\n%s %s / Android %s\n' "$(L 'Target kept for root (system is NOT replaced):' 'Ziel fuer Root (System wird NICHT ersetzt):')" "$TARGET_LABEL" "$TARGET_ANDROID"
       printf '%s %s (%s)\n' "$(L 'Root artifact:' 'Root-Artefakt:')" "$TARGET_ROOT_TYPE" "$TARGET_ROOT_SOURCE"
       return 0 ;;
  esac
  return 1
}

rom_entry_gsi() { # rom-id + compat.json -> gsi field (empty if none/not found)
  local want="$1" f="$2"
  [ -f "$f" ] || return 0
  case "$want" in rom:*) want="${want#rom:}" ;; esac
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
want=sys.argv[2]
for r in json.load(open(sys.argv[1])).get('roms',[]):
    nm=str(r.get('name','')).strip()
    if not nm: continue
    ver=str(r.get('version',r.get('android',r.get('build',''))))
    label=nm+(' '+ver if ver else '')
    variant=str(r.get('variant',''))
    if variant and variant not in label: label+=' '+variant
    bld=str(r.get('build',''))
    if bld and bld not in label: label+=' ('+bld+')'
    if label==want:
        print(r.get('gsi','')); break
" "$f" "$want" 2>/dev/null
  fi
  return 0
}

platform_tools_url() { # [uname_s] -> official portable zip URL (pure, testable)
  case "$(printf '%s' "${1:-$(uname -s)}" | tr '[:upper:]' '[:lower:]')" in
    linux*) printf 'https://dl.google.com/android/repository/platform-tools-latest-linux.zip' ;;
    darwin*) printf 'https://dl.google.com/android/repository/platform-tools-latest-darwin.zip' ;;
    *) printf 'https://dl.google.com/android/repository/platform-tools-latest-windows.zip' ;;
  esac
}

install_base_dir() { # central tools folder (repo tools/ if writable, else user share)
  if [ -w "$TOOL_DIR" ] || mkdir -p "$TOOL_DIR" 2>/dev/null; then printf '%s' "$TOOL_DIR";
  else printf '%s/.local/share/trebleManager/tools' "$HOME"; fi
}

download_file() { # url outfile -> 0/1 with simple progress
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$out" "$url" || return 1
  elif command -v wget >/dev/null 2>&1; then
    wget --show-progress -O "$out" "$url" || return 1
  else
    log ERROR "$(L 'Neither curl nor wget found.' 'Weder curl noch wget gefunden.')"; return 1
  fi
}

scan_dir_for_tools() { # dir -> adopts adb/fastboot/scrcpy found there
  local d="$1" f t
  for t in adb fastboot scrcpy; do
    for cand in "$d/$t" "$d/$t.exe"; do
      if [ -x "$cand" ]; then
        case "$t" in
          adb) ADB_BIN="$cand" ;; fastboot) FB_BIN="$cand" ;; scrcpy) SCRCPY_BIN="$cand" ;;
        esac
        log SUCCESS "$t: $cand"
      fi
    done
    f="$(find "$d" -maxdepth 2 \( -name "$t" -o -name "$t.exe" \) -type f -executable 2>/dev/null | head -1)"
    if [ -n "$f" ]; then
      case "$t" in
        adb) ADB_BIN="$f" ;; fastboot) FB_BIN="$f" ;; scrcpy) SCRCPY_BIN="$f" ;;
      esac
      log SUCCESS "$t: $f"
    fi
  done
}

add_to_path() { # dir -> session PATH (+ persist/symlink handled by caller choice)
  local d="$1"
  case ":$PATH:" in *":$d:"*) ;; *) export PATH="$d:$PATH" ;; esac
  log INFO "PATH += $d ($(L 'this session' 'diese Sitzung'))"
}

install_platform_tools() { # official portable zip -> central tools + PATH
  local url tmp base dest
  url="$(platform_tools_url)"
  base="$(install_base_dir)"
  tmp="/tmp/tt-platform-tools.zip"
  printf '%s\n%s\n' "$(L 'Downloading official platform-tools ...' 'Lade offizielle platform-tools ...')" "$url"
  download_file "$url" "$tmp" || return 1
  mkdir -p "$base"
  if command -v unzip >/dev/null 2>&1; then unzip -q -o "$tmp" -d "$base" || return 1
  elif command -v python3 >/dev/null 2>&1; then python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "$tmp" "$base" || return 1
  else log ERROR "unzip/python3 needed to extract."; return 1; fi
  rm -f "$tmp"
  dest="$base/platform-tools"
  [ -d "$dest" ] || dest="$base"
  scan_dir_for_tools "$dest"
  add_to_path "$dest"
  [ -n "$ADB_BIN" ] && [ -n "$FB_BIN" ]
}

link_into_tools() { # dir -> symlinks found binaries into central tools folder
  local d="$1" base t src
  base="$(install_base_dir)"
  mkdir -p "$base" 2>/dev/null || return 0
  for t in adb adb.exe fastboot fastboot.exe scrcpy scrcpy.exe; do
    src="$(find "$d" -maxdepth 1 -name "$t" -type f 2>/dev/null | head -1)"
    if [ -n "$src" ] && [ ! -e "$base/$t" ]; then
      ln -sf "$src" "$base/$t" 2>/dev/null && log SUCCESS "symlink: $base/$t"
    fi
  done
  add_to_path "$base"
}

ensure_tool() { # id [adb|fastboot] -> loops with options until present or user aborts
  local id="$1"
  while true; do
    if [ "$id" = "adb" ] && [ -n "$ADB_BIN" ]; then return 0; fi
    if [ "$id" = "fastboot" ] && [ -n "$FB_BIN" ]; then return 0; fi
    printf '\n%s\n' "$(L "$id missing. Install or point to it?" "$id fehlt. Installieren oder Pfad angeben?")"
    printf '%s\n' "$(L '[1] Install into PATH (official Google platform-tools, portable)' '[1] In PATH installieren (offizielle Google platform-tools, portable)')"
    printf '%s\n' "$(L '[2] Select one executable (scans its folder for the others)' '[2] Eine Binary waehlen (Ordner wird nach den anderen gescannt)')"
    printf '%s\n' "$(L '[3] Select every needed executable manually' '[3] Jede noetige Binary einzeln waehlen')"
    printf '%s' "$(L '[4] Use custom folder as PATH (adds folder to PATH / symlinks into central tools)  [q] Abort: ' '[4] Eigenen Ordner als PATH nutzen (Ordner in PATH / Symlinks in zentrale tools)  [q] Abbruch: ')"
    iread -r c
    case "$c" in
      1) install_platform_tools || continue ;;
      2) printf '%s' "$(L 'Path to adb/adb.exe or fastboot: ' 'Pfad zu adb/adb.exe oder fastboot: ')"; iread -r p
         p="$(printf '%s' "$p" | sed "s/^['\"]//;s/['\"]$//")"
         if [ -x "$p" ]; then scan_dir_for_tools "$(dirname "$p")"; else log ERROR "$(L 'Not executable.' 'Nicht ausfuehrbar.')"; fi ;;
      3) for t in adb fastboot; do
           cur=""; case "$t" in adb) cur="$ADB_BIN" ;; fastboot) cur="$FB_BIN" ;; esac
           [ -n "$cur" ] && continue
           printf '%s: ' "$t"; iread -r p
           p="$(printf '%s' "$p" | sed "s/^['\"]//;s/['\"]$//")"
           if [ -x "$p" ]; then
             case "$t" in adb) ADB_BIN="$p" ;; fastboot) FB_BIN="$p" ;; esac
             log SUCCESS "$t: $p"
           else log ERROR "$(L 'Not executable, skipped.' 'Nicht ausfuehrbar, uebersprungen.')"; fi
         done ;;
      4) printf '%s' "$(L 'Custom folder: ' 'Eigener Ordner: ')"; iread -r d
         d="$(printf '%s' "$d" | sed "s/^['\"]//;s/['\"]$//")"
         if [ -d "$d" ]; then
           add_to_path "$d"
           scan_dir_for_tools "$d"
           link_into_tools "$d"
         else log ERROR "$(L 'No such folder.' 'Kein solcher Ordner.')"; fi ;;
      q|Q) return 1 ;;
      *) printf '%s\n' "$(L 'Enter 1-4 or q.' '1-4 oder q eingeben.')"; continue ;;
    esac
    save_config
  done
}

ensure_scrcpy() { # finale scrcpy question: install/select/skip (optional forever)
  [ -n "${SCRCPY_BIN:-}" ] && return 0
  printf '\n%s\n' "$(L 'scrcpy (screen mirror) is optional. Want it?' 'scrcpy (Screen-Mirror) ist optional. Gewuenscht?')"
  printf '%s\n' "$(L '[1] Install hint for your OS  [2] Select executable (scans folder)  [3] Skip' '[1] Install-Hinweis fuers OS  [2] Binary waehlen (Ordner-Scan)  [3] Ueberspringen')"
  printf '%s' "[1/2/3]: "; iread -r c
  case "$c" in
    1) case "$(uname -s)" in
         Linux*) printf 'Debian/Ubuntu: sudo apt install scrcpy\nFedora: sudo dnf install scrcpy\nArch: sudo pacman -S scrcpy\n';;
         Darwin*) printf 'macOS: brew install scrcpy\n' ;;
         *) printf 'Windows: Setup-TrebleToolkit.bat or: https://github.com/Genymobile/scrcpy/releases\n' ;;
       esac
       printf '%s' "$(L 'Retry detection now? [Y/n]: ' 'Erkennung jetzt wiederholen? [J/n]: ')"; iread -r r
       case "$r" in ""|y|Y|j|J) SCRCPY_BIN="$(command -v scrcpy 2>/dev/null || true)"
         [ -n "$SCRCPY_BIN" ] && log SUCCESS "scrcpy: $SCRCPY_BIN" || log INFO "$(L 'Still missing - staying optional.' 'Weiter fehlend - bleibt optional.')" ;; esac ;;
    2) printf '%s' "$(L 'Path to scrcpy executable: ' 'Pfad zu scrcpy: ')"; iread -r p
       p="$(printf '%s' "$p" | sed "s/^['\"]//;s/['\"]$//")"
       if [ -x "$p" ]; then SCRCPY_BIN="$p"; scan_dir_for_tools "$(dirname "$p")"; save_config
       else log ERROR "$(L 'Not executable.' 'Nicht ausfuehrbar.')"; fi ;;
    *) log INFO "$(L 'scrcpy skipped (stays optional).' 'scrcpy uebersprungen (bleibt optional).')" ;;
  esac
  return 0
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
boot_magic_ver() { # path -> version int or -1 (ANDROID! magic, NUL-safe)
  [ -f "$1" ] || { printf -- '-1'; return; }
  local magic
  magic="$(od -An -tx1 -N8 "$1" 2>/dev/null | tr -d ' \n' | tr '[:lower:]' '[:upper:]')"
  if [ "$magic" = "414E44524F494421" ]; then
    od -An -tu1 -j8 -N1 "$1" 2>/dev/null | tr -d ' \n'
  else printf -- '-1'; fi
}
image_kind() { # path -> boot|system|unknown (boot=ANDROID!, system=sparse/ext4 filesystem)
  local f="$1" h
  [ -f "$f" ] || { printf 'unknown'; return; }
  h="$(od -An -tx1 -N8 "$f" 2>/dev/null | tr -d ' \n' | tr '[:lower:]' '[:upper:]')"
  if [ "$h" = "414E44524F494421" ]; then printf 'boot'; return; fi
  if [ "$(printf '%s' "$h" | cut -c1-8)" = "3AFF26ED" ]; then printf 'system'; return; fi
  if [ "$(od -An -tx1 -j1080 -N2 "$f" 2>/dev/null | tr -d ' \n' | tr '[:lower:]' '[:upper:]')" = "53EF" ]; then printf 'system'; return; fi
  printf 'unknown'
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
  case "$low" in */download) low="${low%/download}" ;; esac  # SourceForge direct links
  case "$low" in *.zip|*.7z|*.tar|*.gz|*.tgz|*.xz|*.app|*.rar|*.img) return 0 ;; *) return 1 ;; esac
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
rom_downloads() { # prints num|label|file|url for registry ROMs with verified direct links
  local f; f="$(compat_file)"
  [ -f "$f" ] || return 1
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
n=0
for r in json.load(open(sys.argv[1])).get('roms',[]):
    url=str(r.get('url',''))
    if not url: continue
    nm=str(r.get('name','')).strip()
    if not nm: continue
    ver=str(r.get('version',r.get('android',r.get('build',''))))
    label=nm+(' '+ver if ver else '')
    variant=str(r.get('variant',''))
    if variant and variant not in label: label+=' '+variant
    bld=str(r.get('build',''))
    if bld and bld not in label: label+=' ('+bld+')'
    n+=1
    print('%d|%s|%s|%s|%s' % (n,label,str(r.get('file','')),url,str(r.get('status',''))))
" "$f" 2>/dev/null
  fi
}
download_rom() { # [number|label] [--yes] -> prints ready file; downloads+decompresses+validates
  local pick="${1:-}" yes="" n=0 line
  [ "${2:-}" = "--yes" ] && yes=1
  local list; list="$(rom_downloads)"
  if [ -z "$list" ]; then
    printf '%s\n' "$(L 'No downloadable ROMs in registry - drop the package into data/roms/ manually.' 'Keine ladbaren ROMs in Registry - Paket manuell nach data/roms/ legen.')"
    return 1
  fi
  printf '\n%s\n' "$(L 'Working downloads (verified links):' 'Working-Downloads (gepruefte Links):')"
  printf '%s\n' "$list" | while IFS= read -r line; do
    printf ' [%s] %s\n' "${line%%|*}" "$(printf '%s' "$line" | cut -d'|' -f2)"
  done
  case "$pick" in ""|*[!0-9]*)
    if [ -n "$pick" ]; then
      pick="$(printf '%s' "$list" | awk -F'|' -v want="$pick" '$2==want {print $1; exit}')"
    fi
    if [ -z "$pick" ]; then
      printf '%s' "$(L 'Number + Enter (Enter=abort): ' 'Nummer + Enter (Enter=Abbruch): ')"; iread -r pick
    fi ;;
  esac
  line="$(printf '%s' "$list" | sed -n "${pick}p" 2>/dev/null)"
  [ -n "$line" ] || return 1
  local fname url
  fname="$(printf '%s' "$line" | cut -d'|' -f3)"
  url="$(printf '%s' "$line" | cut -d'|' -f4)"
  [ -z "$fname" ] && fname="$(basename "$url" | cut -d'?' -f1)"
  mkdir -p "$ROM_DIR"
  local dst="$ROM_DIR/$fname" got
  if [ -z "$yes" ]; then
    printf '%s %s\n%s' "$(L 'Download ~1 GB from:' 'Download ~1 GB von:')" "$url" "$(L 'Start download? [Y/n]: ' 'Download starten? [J/n]: ')"; iread -r yn
    case "$yn" in ""|y|Y|j|J) yes=1 ;; *) return 1 ;; esac
  fi
  got="$(download_firmware "$url" "$dst" --yes)" || return 1
  verify_download "$dst" || true
  local ready="$dst"
  case "$ready" in *.gz|*.GZ)
    printf '%s\n' "$(L 'Decompressing (.gz) ...' 'Dekomprimiere (.gz) ...')"
    local rawname; rawname="$(basename "$ready")"; rawname="${rawname%.gz}"; rawname="${rawname%.GZ}"
    [ -z "$rawname" ] && rawname="system.img"
    if command -v gunzip >/dev/null 2>&1; then gunzip -c "$ready" > "$ROM_DIR/$rawname" 2>/dev/null
    else python3 -c "import gzip,shutil,sys; shutil.copyfileobj(gzip.open(sys.argv[1],'rb'),open(sys.argv[2],'wb'))" "$ready" "$ROM_DIR/$rawname" 2>/dev/null; fi
    if [ -f "$ROM_DIR/$rawname" ] && [ -s "$ROM_DIR/$rawname" ]; then ready="$ROM_DIR/$rawname"
    else printf '%s\n' "$(L 'Decompress failed.' 'Dekomprimieren fehlgeschlagen.')"; return 1; fi ;;
  esac
  local kind; kind="$(image_kind "$ready")"
  if [ "$kind" = "system" ]; then printf '%s\n' "$(L 'Ready: SYSTEM image (for Install ROM / flash system).' 'Fertig: SYSTEM-Image (fuer ROM-Installation / flash system).')"
  elif [ "$kind" = "boot" ]; then printf '%s\n' "$(L 'Ready: BOOT/RECOVERY image (for Magisk patch base via export).' 'Fertig: BOOT/RECOVERY-Image (fuer Magisk-Patch-Basis via Export).')"
  else printf '%s\n' "$(L 'Downloaded, but content unclear - validate before use.' 'Geladen, aber Inhalt unklar - vor Nutzung validieren.')"; fi
  printf 'Ready: %s\n' "$ready"
  DL_READY="$ready"
  printf '%s' "$ready"
}

# ---------------------------------------------------------------- recovery export from custom ROMs
zip_entries() { # zip -> entry list
  if command -v unzip >/dev/null 2>&1; then unzip -l "$1" 2>/dev/null | awk 'NR>3 && $4 != "" && $4 != "----" {print $4}';
  elif command -v python3 >/dev/null 2>&1; then python3 -c "import zipfile,sys; [print(i.filename) for i in zipfile.ZipFile(sys.argv[1]).infolist()]" "$1" 2>/dev/null; fi
}
export_recovery() { # rompath -> prints dir; sets EXPORT_FILES
  local rom="$1" base dir low
  [ -f "$rom" ] || { log ERROR "$(L 'ROM file missing: ' 'ROM-Datei fehlt: ')$rom"; return 3; }
  base="$(basename "$rom" | sed -e 's/\.tar\.gz$//' -e 's/\.tgz$//' -e 's/\.[^.]*$//')"
  dir="$REC_DIR/${base}-$STAMP"
  mkdir -p "$dir"
  EXPORT_FILES=""
  low="$(printf '%s' "$rom" | tr '[:upper:]' '[:lower:]')"
  # Normalize single-file compression first (.gz/.xz, but NOT tar containers):
  case "$low" in
    *.tar.gz|*.tar.xz|*.tgz) ;;
    *.gz)
      rawname="$(basename "$rom")"; rawname="${rawname%.gz}"; rawname="${rawname%.GZ}"
      [ -z "$rawname" ] && rawname="image.img"
      if command -v gunzip >/dev/null 2>&1; then gunzip -c "$rom" > "$dir/$rawname" 2>/dev/null
      else python3 -c "import gzip,shutil,sys; shutil.copyfileobj(gzip.open(sys.argv[1],'rb'),open(sys.argv[2],'wb'))" "$rom" "$dir/$rawname" 2>/dev/null; fi
      if [ -f "$dir/$rawname" ] && [ -s "$dir/$rawname" ]; then rom="$dir/$rawname"; low="$(printf '%s' "$rom" | tr '[:upper:]' '[:lower:]')"
      else log ERROR "$(L 'Decompress failed (corrupt file?).' 'Dekomprimieren fehlgeschlagen (Datei kaputt?).')"; return 1; fi ;;
    *.xz)
      rawname="$(basename "$rom")"; rawname="${rawname%.xz}"; rawname="${rawname%.XZ}"
      [ -z "$rawname" ] && rawname="image.img"
      if command -v unxz >/dev/null 2>&1; then unxz -c "$rom" > "$dir/$rawname" 2>/dev/null
      elif command -v xz >/dev/null 2>&1; then xz -dc "$rom" > "$dir/$rawname" 2>/dev/null
      else python3 -c "import lzma,shutil,sys; shutil.copyfileobj(lzma.open(sys.argv[1],'rb'),open(sys.argv[2],'wb'))" "$rom" "$dir/$rawname" 2>/dev/null; fi
      if [ -f "$dir/$rawname" ] && [ -s "$dir/$rawname" ]; then rom="$dir/$rawname"; low="$(printf '%s' "$rom" | tr '[:upper:]' '[:lower:]')"
      else log ERROR "$(L 'Decompress failed (corrupt file or missing python lzma?).' 'Dekomprimieren fehlgeschlagen (Datei kaputt oder python-lzma fehlt?).')"; return 1; fi ;;
  esac
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
      if [ "$(image_kind "$rom")" = "system" ]; then
        log ERROR "$(L 'This is a SYSTEM image (filesystem with folders). Magisk cannot patch system - it needs boot/recovery. A GSI leaves recovery untouched stock: use the stock UPDATE.APP recovery as patch base.' 'Das ist ein SYSTEM-Image (Dateisystem mit Ordnern). Magisk kann kein System patchen - es braucht boot/recovery. Ein GSI laesst Recovery unberuehrt Stock: nimm das Stock-UPDATE.APP-Recovery als Patch-Basis.')"
        return 3
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
    *.tar|*.tar.gz|*.tar.xz|*.tgz)
      local stage="$dir/_archive" f dst2
      mkdir -p "$stage"
      if command -v tar >/dev/null 2>&1 && tar -xf "$rom" -C "$stage" 2>/dev/null; then
        while IFS= read -r f; do
          [ -n "$f" ] || continue
          dst2="$dir/$(basename "$f")"
          cp -f "$f" "$dst2"
          if [ "$(boot_magic_ver "$dst2")" != "-1" ]; then EXPORT_FILES="$EXPORT_FILES $dst2"; else rm -f "$dst2"; fi
        done < <(find "$stage" -type f \( -iname '*recovery*.img' -o -iname 'boot.img' \) 2>/dev/null)
        if [ -n "$EXPORT_FILES" ]; then
          printf '{"source": "%s", "kind": "rom-tar"}\n' "$rom" > "$dir/metadata.json"
          log SUCCESS "$(L 'Exported from ROM tar archive. Your patch base when this ROM is installed.' 'Aus ROM-TAR exportiert. Deine Patch-Basis wenn dieses ROM installiert ist.')"
          printf '%s' "$dir"; return 0
        fi
        local pay; pay="$(find "$stage" -type f -iname 'payload.bin' 2>/dev/null | head -1)"
        if [ -n "$pay" ]; then
          local dumper2
          dumper2="$(find "$TOOL_DIR" \( -iname '*payload*dumper*' -o -name 'payload-dumper-go' \) -type f 2>/dev/null | head -1)"
          if [ -n "$dumper2" ]; then
            "$dumper2" -o "$dir" -p "boot,recovery" "$pay" > "$dir/dumper.log" 2>&1 || true
            for o in "$dir"/*.img; do
              [ -f "$o" ] || continue
              if [ "$(boot_magic_ver "$o")" != "-1" ]; then EXPORT_FILES="$EXPORT_FILES $o"; fi
            done
            if [ -n "$EXPORT_FILES" ]; then printf '%s' "$dir"; return 0; fi
          else
            log ERROR "$(L 'payload.bin inside tar: place payload-dumper-go in data/tools/ or extract boot.img manually.' 'payload.bin im TAR: payload-dumper-go nach data/tools/ legen oder boot.img manuell extrahieren.')"
            return 3
          fi
        fi
      else
        log ERROR "$(L 'Archive extract failed (need tar).' 'Archiv-Extrakt fehlgeschlagen (braucht tar).')"
        return 1
      fi
      log ERROR "$(L 'No boot.img/recovery.img/payload.bin in archive. Probably a GSI system package (no recovery by design).' 'Kein boot.img/recovery.img/payload.bin im Archiv. Wahrscheinlich GSI-System-Paket (kein Recovery).')"
      return 3 ;;
    *) log ERROR "$(L 'Unsupported format (use .img, .img.gz/.img.xz, ROM .zip or .tar/.tar.gz/.tgz).' 'Format nicht unterstuetzt (.img, .img.gz/.img.xz, ROM-.zip oder .tar/.tar.gz/.tgz).')"; return 3 ;;
  esac
}

# ---------------------------------------------------------------- magisk prep (real on-device patch, never fake)
magisk_apk() { find "$MAG_DIR" -maxdepth 1 -iname '*.apk' -type f 2>/dev/null | head -1; }
magisk_stable() { # prints version|code|link from official stable.json (never hardcoded)
  local url="https://raw.githubusercontent.com/topjohnwu/magisk-files/master/stable.json" j
  if [ -f "$(compat_file)" ] && command -v python3 >/dev/null 2>&1; then
    local reg
    reg="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('magisk',{}).get('stable_json',''))" "$(compat_file)" 2>/dev/null)"
    [ -n "$reg" ] && url="$reg"
  fi
  j="$(curl -fsSL -H 'User-Agent: trebleManager' "$url" 2>/dev/null)" || return 1
  if command -v python3 >/dev/null 2>&1; then
    printf '%s' "$j" | python3 -c "import json,sys; m=json.load(sys.stdin).get('magisk',{}); print(str(m.get('version',''))+'|'+str(m.get('versionCode',''))+'|'+str(m.get('link','')))"
  else
    printf '%s|%s|%s\n' "$(printf '%s' "$j" | grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | cut -d'"' -f4)" "" "$(printf '%s' "$j" | grep -o '"link"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | cut -d'"' -f4)"
  fi
}
download_magisk() { # [--yes] -> prints APK path; official stable only
  local yes="${1:-}" st ver link
  st="$(magisk_stable)" || { printf '%s\n' "$(L 'Magisk stable info unreachable (offline?). Place the APK from https://github.com/topjohnwu/Magisk/releases into data/magisk/ manually.' 'Magisk-Stable-Info unerreichbar (offline?). APK manuell nach data/magisk/ legen.')"; return 1; }
  ver="$(printf '%s' "$st" | cut -d'|' -f1)"; link="$(printf '%s' "$st" | cut -d'|' -f3)"
  if [ -z "$link" ]; then printf '%s\n' "$(L 'No APK link in stable info.' 'Kein APK-Link in Stable-Info.')"; return 1; fi
  mkdir -p "$MAG_DIR"
  local dst="$MAG_DIR/Magisk-v$ver.apk"
  if [ -f "$dst" ]; then printf '%s %s\n' "$(L 'Magisk cached:' 'Magisk gecached:')" "$dst" >&2; printf '%s' "$dst"; return 0; fi
  if [ -z "$yes" ]; then
    printf '%s' "$(L "Fetch official Magisk v$ver? [Y/n]: " "Offizielles Magisk v$ver laden? [J/n]: ")"; iread -r a
    case "$a" in ""|y|Y|j|J) ;; *) return 1 ;; esac
  fi
  printf '%s\n' "$(L "Downloading Magisk v$ver ..." "Lade Magisk v$ver ...")" >&2
  if command -v curl >/dev/null 2>&1; then curl -fsSL -o "$dst" "$link" 2>/dev/null
  else wget -q -O "$dst" "$link" 2>/dev/null; fi
  if [ -f "$dst" ] && [ -s "$dst" ]; then
    (if command -v sha256sum >/dev/null 2>&1; then sha256sum "$dst" | awk '{print $1}'; else shasum -a 256 "$dst" | awk '{print $1}'; fi) > "$dst.sha256"
    log SUCCESS "Magisk downloaded: $dst"
    printf '%s' "$dst"; return 0
  fi
  rm -f "$dst"; log ERROR "$(L 'Magisk download failed.' 'Magisk-Download fehlgeschlagen.')"; return 1
}
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
  printf '%s\n' "$out" >> "$TTLOG"
  if printf '%s\n' "$out" | flash_verdict; then log SUCCESS "$(L 'Flash reported: OK.' 'Flash gemeldet: OK.')"; save_slot "magisk"; return 0; fi
  log ERROR "$(L 'Flash not OK (see result above).' 'Flash nicht OK (siehe Ergebnis oben).')"; return 1
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
  case "$id" in *uid=0*) log SUCCESS "ROOT DETECTED (uid=0)."; save_root_state "ROOTED"; printf 'ROOTED\n'; return 0 ;; esac
  if [ -n "$w" ] && [[ "$w" != *"not found"* ]]; then log WARNING "INCONCLUSIVE (su present, no uid=0)."; save_root_state "INCONCLUSIVE"; printf 'INCONCLUSIVE\n'; return 2; fi
  log WARNING "$(L 'No root verifiable.' 'Kein Root nachweisbar.')"; save_root_state "NOT_ROOTED"; printf 'NOT_ROOTED\n'; return 2
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
  save_slot "stock"
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

# ---------------------------------------------------------------- orchestrator (P0/P1)
# Unknown is never compatible; --yes never bypasses prerequisites.
device_states() { # sets ADB_STATE FB_STATE OVERALL (+ADB_DEVS/FB_DEVS); explicit states
  ADB_STATE="ADB_NOT_FOUND"; FB_STATE="FASTBOOT_NOT_FOUND"
  ADB_DEVS=""; FB_DEVS=""
  if [ -n "$ADB_BIN" ]; then
    ADB_DEVS="$("$ADB_BIN" devices 2>/dev/null | awk 'NR>1 && NF>=2 {print $1":"$2}')"
    ready="$(printf '%s' "$ADB_DEVS" | grep -c ':device$' || true)"
    unauth="$(printf '%s' "$ADB_DEVS" | grep -c ':unauthorized$' || true)"
    off="$(printf '%s' "$ADB_DEVS" | grep -c ':offline$' || true)"
    if [ "$ready" -gt 1 ]; then ADB_STATE="ADB_MULTIPLE_DEVICES"
    elif [ "$ready" = 1 ]; then ADB_STATE="ADB_READY"
    elif [ "$unauth" -gt 0 ]; then ADB_STATE="ADB_UNAUTHORIZED"
    elif [ "$off" -gt 0 ]; then ADB_STATE="ADB_OFFLINE"
    else ADB_STATE="NO_DEVICE"; fi
  fi
  if [ -n "$FB_BIN" ]; then
    FB_DEVS="$("$FB_BIN" devices 2>/dev/null | awk 'NF>=2 && $2=="fastboot" {print $1}')"
    n="$(printf '%s' "$FB_DEVS" | grep -c . || true)"
    if [ "$n" -gt 1 ]; then FB_STATE="FASTBOOT_MULTIPLE_DEVICES"
    elif [ "$n" = 1 ]; then FB_STATE="FASTBOOT_READY"
    else FB_STATE="FASTBOOT_NO_DEVICE"; fi
  fi
  OVERALL="UNKNOWN_DEVICE_STATE"
  if { [ "$ADB_STATE" = "ADB_NOT_FOUND" ] || [ "$ADB_STATE" = "NO_DEVICE" ]; } && { [ "$FB_STATE" = "FASTBOOT_NOT_FOUND" ] || [ "$FB_STATE" = "FASTBOOT_NO_DEVICE" ]; }; then OVERALL="NO_DEVICE"; fi
  if [ "$ADB_STATE" = "ADB_READY" ] || [ "$FB_STATE" = "FASTBOOT_READY" ]; then OVERALL="READY"; fi
}
select_target() { # explicit selection -> ANDROID_SERIAL (honored by adb+fastboot)
  local all
  all="$(printf '%s' "$ADB_DEVS" | grep ':device$' | sed 's/:device$/ (adb)/'; printf '%s' "$FB_DEVS" | grep . | sed 's/$/ (fastboot)/')"
  [ "$(printf '%s' "$all" | grep -c .)" -le 1 ] && return 0
  printf '%s\n' "$(L 'Multiple devices - select target:' 'Mehrere Geraete - Ziel waehlen:')"
  printf '%s\n' "$all" | awk '{printf "  [%d] %s\n", NR, $0}'
  printf '%s' "$(L 'Number: ' 'Nummer: ')"; iread -r s
  local ser; ser="$(printf '%s' "$all" | sed -n "${s}p" | awk '{print $1}')"
  [ -n "$ser" ] || { log ERROR "$(L 'No target selected.' 'Kein Ziel gewaehlt.')"; return 1; }
  export ANDROID_SERIAL="$ser"
  log SUCCESS "Target: $ser (ANDROID_SERIAL)"
}
preflight() { # global gate; prints blocks; returns 0 when menu allowed
  local blocks=""
  [ -n "$ADB_BIN" ] || blocks="${blocks}adb missing; "
  [ -n "$FB_BIN" ] || blocks="${blocks}fastboot missing; "
  for d in "$LOG_DIR" "$BACK_DIR" "$FIRM_DIR" "$MAG_DIR"; do
    [ -w "$d" ] || blocks="${blocks}not writable: $d; "
  done
  device_states
  if [ -n "$blocks" ]; then
    printf 'PREFLIGHT BLOCKED: %s\nADB: %s | Fastboot: %s\n' "$blocks" "$ADB_STATE" "$FB_STATE"
    return 1
  fi
  printf 'PREFLIGHT READY (all green): ADB=%s Fastboot=%s\n' "$ADB_STATE" "$FB_STATE"
  return 0
}
step_gate() { # step -> 0 pass / prints reasons; never bypassed
  local step="$1" reasons=""
  device_states >/dev/null 2>&1 || true
  case "$step" in analyze|verify|patch|backup|flash|restore|flash-system|twrp|validate|export|diagnostic)
    [ "$OVERALL" = "NO_DEVICE" ] && reasons="${reasons}no device; " ;;
  esac
  case "$step" in analyze|verify|validate)
    [ "$ADB_STATE" != "ADB_READY" ] && reasons="${reasons}need ADB (now $ADB_STATE); " ;;
  esac
  case "$step" in flash|restore|flash-system|twrp)
    [ "$FB_STATE" != "FASTBOOT_READY" ] && reasons="${reasons}need fastboot (now $FB_STATE); " ;;
  esac
  case "$step" in flash|twrp|flash-system)
    [ "$(profile_verified)" != "1" ] && reasons="${reasons}profile unverified; " ;;
  esac
  [ "$step" = "flash" ] && [ -z "$PATCHED_IMAGE" ] && reasons="${reasons}no patched image; "
  if [ -n "$reasons" ]; then printf 'BLOCKED: %s\n' "$reasons"; return 1; fi
  return 0
}
goal_steps() { # goal -> ordered steps (pure)
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    root) printf 'reconnaissance compatibility firmware extract magisk_patch backup flash reboot root_verify validate' ;;
    custom_rom) printf 'reconnaissance compatibility firmware rom_validation backup_if_required flash_system reboot validate' ;;
    stock_rom) printf 'reconnaissance firmware_selection firmware_validation artifact_extraction backup flash_plan safety_gate flash reboot validate' ;;
    root_custom_rom) printf 'reconnaissance custom_rom_compatibility firmware rom_installation boot root_preparation backup root_flash root_verify validate' ;;
    root_stock_rom) printf 'reconnaissance stock_firmware_validation root_image_preparation backup root_flash reboot root_verify validate' ;;
    root_custom_rom_recovery) printf 'reconnaissance rom_compatibility recovery_compatibility firmware backup rom_flash recovery_flash root_preparation root_flash boot verify validate' ;;
    root_stock_rom_recovery) printf 'reconnaissance stock_firmware_validation recovery_validation backup recovery_flash root_preparation root_flash boot verify validate' ;;
    restore_original) printf 'reconnaissance identify_original_artifact validate_backup rollback_plan safety_gate restore reboot validate' ;;
    full_reinstall) printf 'reconnaissance compatibility firmware rom_validation backup wipe flash_system reboot validate' ;;
    *) printf '' ;;
  esac
}
state_file() { printf '%s/workflow-state.json' "$LOG_DIR"; }
write_state() { # goal step status
  local goal="$1" step="$2" status="$3" f
  f="$(state_file)"
  if command -v python3 >/dev/null 2>&1; then
    ST_GOAL="$goal" ST_STEP="$step" ST_STATUS="$status" ST_FILE="$f" python3 - <<'PYEOF' 2>/dev/null || true
import json,os,datetime
f=os.environ['ST_FILE']
try: st=json.load(open(f))
except Exception: st={"goal":os.environ['ST_GOAL'],"steps":[]}
st["goal"]=os.environ['ST_GOAL']
st["steps"]=[s for s in st.get("steps",[]) if s.get("id")!=os.environ['ST_STEP']]
st["steps"].append({"id":os.environ['ST_STEP'],"status":os.environ['ST_STATUS']})
st["updated"]=datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
json.dump(st,open(f,"w"),indent=2)
PYEOF
  else
    printf '{"goal":"%s","step":"%s","status":"%s"}\n' "$goal" "$step" "$status" >> "$f"
  fi
}
read_state_goal() {
  local f; f="$(state_file)"
  [ -f "$f" ] || return 1
  if command -v python3 >/dev/null 2>&1; then python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('goal',''))" "$f" 2>/dev/null && return 0; fi
  grep -o '"goal"[[:space:]]*:[[:space:]]*"[^"]*"' "$f" | tail -1 | cut -d'"' -f4
}
goal_screen() { # dispatch one plan step to its screen (reuse!)
  case "$1" in
    reconnaissance) screen_analyze ;;
    compatibility) screen_compat ;;
    firmware|firmware_selection|stock_firmware_validation|firmware_validation) screen_firmware ;;
    extract|artifact_extraction) screen_extract ;;
    magisk_patch|root_preparation|root_image_preparation) screen_patch ;;
    backup|backup_if_required) screen_backup ;;
    flash|root_flash|safety_gate|flash_plan) screen_flash ;;
    rom_validation) screen_extract ;;
    rom_installation|rom_flash) screen_flashsystem ;;
    recovery_compatibility) screen_rootmethods ;;
    recovery_validation|recovery_flash) screen_twrp ;;
    reboot|boot) screen_verify ;;
    root_verify|verify) screen_verify ;;
    validate) screen_verify ;;
    restore|identify_original_artifact|validate_backup|rollback_plan) do_restore "" "" || true; pause_tt ;;
    custom_rom_compatibility|rom_compatibility) screen_compat ;;
    rom_export|artifact_export) screen_export ;;
    wipe) screen_wipe ;;
    flash_system) screen_flashsystem ;;
  esac
}
run_goal() { # goal -> stepwise with persisted state, controlled stop on failure
  local goal="$1" steps s
  steps="$(goal_steps "$goal")"
  [ -z "$steps" ] && { log ERROR "Unknown goal: $goal"; return 1; }
  for s in $steps; do
    write_state "$goal" "$s" "active"
    goal_screen "$s" || true
    write_state "$goal" "$s" "done"
  done
  log SUCCESS "Workflow '$goal' COMPLETED."
}
screen_goals() {
  printf '\n%s\n' "$(L 'Select workflow goal (planner shows plan first):' 'Workflow-Ziel (Planner zeigt Plan zuerst):')"
  local i=1 g
  for g in root custom_rom stock_rom root_custom_rom root_stock_rom root_custom_rom_recovery root_stock_rom_recovery restore_original full_reinstall; do
    printf '  [%d] %s\n' "$i" "$g"; i=$((i+1))
  done
  printf '%s' "$(L 'Number (Enter=back): ' 'Nummer (Enter=zurueck): ')"; iread -r n
  case "$n" in 1) g=root;; 2) g=custom_rom;; 3) g=stock_rom;; 4) g=root_custom_rom;; 5) g=root_stock_rom;; 6) g=root_custom_rom_recovery;; 7) g=root_stock_rom_recovery;; 8) g=restore_original;; 9) g=full_reinstall;; *) pause_tt; return ;; esac
  printf '\nPlan for %s:\n' "$g"
  for s in $(goal_steps "$g"); do printf ' - %s\n' "$s"; done
  printf '%s' "$(L 'Run now? [Y/n]: ' 'Jetzt starten? [J/n]: ')"; iread -r a
  case "$a" in ""|y|Y|j|J) run_goal "$g" ;; esac
  pause_tt
}
screen_resume() {
  local g; g="$(read_state_goal)" || { log WARNING "$(L 'No saved workflow.' 'Kein gespeicherter Workflow.')"; pause_tt; return; }
  [ -z "$g" ] && { log WARNING "$(L 'No saved workflow.' 'Kein gespeicherter Workflow.')"; pause_tt; return; }
  header "$(L 'Resume workflow: ' 'Workflow fortsetzen: ')$g"
  printf '%s' "$(L 'Re-run from plan? [Y/n]: ' 'Neu aus Plan starten? [J/n]: ')"; iread -r a
  case "$a" in ""|y|Y|j|J) run_goal "$g" ;; esac
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
  printf 'Phone runs       : %s\n' "$(rom_label "$INSTALLED_ROM")"
  printf 'Slot (recovery_ramdisk): %s  (TWRP/Magisk share it, last flashed wins)\n' "$(read_slot | cut -d'|' -f1)"
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
    [ -n "$linver" ] && printf 'LineageOS: %s\n' "$linver"
    [ -n "$linver" ] && printf '%s\n' "$(L 'Custom ROM detected: the wizard determines the correct patch base (stock recovery for GSIs, ROM package for device builds).' 'Custom-ROM erkannt: Der Wizard bestimmt die korrekte Patch-Basis (Stock-Recovery fuer GSIs, ROM-Paket fuer Device-Builds).')"
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
  load_rom
  printf '\n'
  if [ -z "$INSTALLED_ROM" ]; then
    printf '%s\n' "$(L "I don't know your system yet - one question:" 'Ich kenne dein System noch nicht - eine Frage:')"
    select_rom
  else
    printf '%s %s\n' "$(L 'Phone runs (saved):' 'Handy laeuft mit (gespeichert):')" "$(rom_label "$INSTALLED_ROM")"
    printf '%s' "$(L '[C] change system  [Enter] keep: ' '[C] System aendern  [Enter] behalten: ')"; iread -r rc
    case "$rc" in [Cc]) select_rom ;; esac
  fi
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
  printf '%s\n' "$(L 'Drop ROM packages into data/roms/ (.zip/.tar.gz/.img.gz/.img.xz or .img). GSI system images are refused honestly.' 'ROM-Pakete nach data/roms/ (.zip/.tar.gz/.img.gz/.img.xz/.img). GSI wird ehrlich abgelehnt.')"
  local roms; roms="$(find "$ROM_DIR" -maxdepth 1 \( -iname '*.zip' -o -iname '*.img' -o -iname '*.tar' -o -iname '*.tar.gz' -o -iname '*.tgz' -o -iname '*.gz' -o -iname '*.xz' \) -type f 2>/dev/null)"
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
    printf '%s' "$(L 'No Magisk APK in data/magisk/. Place it there, or path (Enter=fetch automatically): ' 'Keine Magisk-APK in data/magisk/. Dort ablegen oder Pfad (Enter=automatisch laden): ')"; iread -r p
    if [ -n "$p" ] && [ -f "$p" ]; then cp -f "$p" "$MAG_DIR/"; apk="$MAG_DIR/$(basename "$p")"
    else apk="$(download_magisk)" || apk=""; fi
  fi
  [ -n "$apk" ] && printf 'APK: %s\n' "$apk"
  local base; base="$(patch_base)"
  local bsrc="${base%%|*}"; base="${base#*|}"
  local bimg="${base%%|*}"; local blabel="${base##*|}"
  if [ "$bsrc" = "rom" ]; then
    printf '\n%s %s %s\n' "$(L 'Phone runs:' 'Handy laeuft mit:')" "$blabel" "$(L '(full device ROM)' '(volles Device-ROM)')"
    printf '%s\n' "$(L 'RULE: patch base MUST come from this ROM package. NOT from stock firmware.' 'REGEL: Patch-Basis MUSS aus diesem ROM-Paket kommen. NICHT aus Stock-Firmware.')"
  elif [ "$bsrc" = "stock-gsi" ]; then
    printf '\n%s %s %s\n' "$(L 'Phone runs:' 'Handy laeuft mit:')" "$blabel" "$(L '(GSI, system-only)' '(GSI, nur System)')"
    printf '%s\n' "$(L 'A GSI never touches recovery: your recovery_ramdisk is still stock, so the patch base IS the stock recovery. Correct, not a workaround.' 'Ein GSI fasst Recovery nie an: dein recovery_ramdisk ist weiter Stock, also ist die Patch-Basis das Stock-Recovery. Korrekt, kein Workaround.')"
  fi
  if [ -z "$bimg" ] || [ ! -f "$bimg" ]; then
    if [ "$bsrc" = "rom" ]; then
      printf '%s' "$(L 'No ROM base image yet. Open recovery export now? [Y/n]: ' 'Noch kein ROM-Basis-Image. Jetzt Recovery-Export oeffnen? [J/n]: ')"; iread -r oe
      case "$oe" in ""|y|Y|j|J) screen_export ;;
      esac
      base="$(patch_base)"; bsrc="${base%%|*}"; base="${base#*|}"; bimg="${base%%|*}"; blabel="${base##*|}"
    fi
  fi
  if [ -z "$bimg" ] || [ ! -f "$bimg" ]; then
    if [ "$bsrc" = "rom" ]; then printf '%s\n' "$(L 'Still no ROM base image: put the ROM package into data/roms/ and export first.' 'Immer noch kein ROM-Basis-Image: ROM-Paket nach data/roms/ legen und erst exportieren.')"
    else printf '%s\n' "$(L 'No stock image -> step 4 first.' 'Kein Stock-Image -> erst Step 4.')"; fi
    pause_tt; return
  fi
  local bsha=""; bsha="$(printf '%s' "$(test_image "$bimg")" | cut -d'|' -f2)"
  printf 'Input: %s\n' "$bimg"
  local srcnote=" (stock)"
  if [ "$bsrc" = "rom" ]; then srcnote="$(L ' (this ROM - correct)' ' (dieses ROM - korrekt)')"
  elif [ "$bsrc" = "stock-gsi" ]; then srcnote="$(L ' (stock recovery - correct for GSI)' ' (Stock-Recovery - korrekt fuer GSI)')"; fi
  printf '%s %s%s\n' "$(L 'Source:' 'Quelle:')" "$blabel" "$srcnote"
  printf 'Target: %s\nDevice: Huawei %s\nFirmware: %s\nSHA-256: %s\n' "$(target_partition)" "$PROFILE_ID" "$FW_BASELINE" "$bsha"
  printf '%s\n' "$(L '[1] Prepare patch (to-patch + instructions)  [2] Register patched file' '[1] Patch vorbereiten  [2] Gepatchte Datei registrieren')"; iread -r k
  case "$k" in
    1) prepare_patch "$bimg" >/dev/null
       printf '%s\n' "$(L 'Prepared. Patch on device per data/magisk/to-patch/PATCH-INSTRUCTIONS.txt.' 'Vorbereitet. Am Geraet patchen (Anleitung in to-patch/).')"
       printf '%s %s\n' "$(L 'In the Magisk app select EXACTLY this file:' 'In der Magisk-App EXAKT diese Datei auswaehlen:')" "$(basename "$bimg")" ;;
    2) adb_run shell 'ls /sdcard/Download/magisk_patched*.img 2>&1' 2>/dev/null || true
       printf '%s' "$(L 'Path to patched file: ' 'Pfad gepatchte Datei: ')"; iread -r pp
       case "$pp" in *.gz|*.GZ)
         if [ -f "$pp" ]; then
           printf '%s\n' "$(L 'GZip file: decompressing first ...' 'GZip-Datei: erst dekomprimieren ...')"
           _pdn="$(basename "$pp")"; _pdn="${_pdn%.gz}"; _pdn="${_pdn%.GZ}"
           [ -z "$_pdn" ] && _pdn="patched.img"
           if command -v gunzip >/dev/null 2>&1; then gunzip -c "$pp" > "$MAG_DIR/$_pdn" 2>/dev/null
           else python3 -c "import gzip,shutil,sys; shutil.copyfileobj(gzip.open(sys.argv[1],'rb'),open(sys.argv[2],'wb'))" "$pp" "$MAG_DIR/$_pdn" 2>/dev/null; fi
           [ -f "$MAG_DIR/$_pdn" ] && pp="$MAG_DIR/$_pdn"
         fi ;;
       esac
       if [ -n "$pp" ] && [ -f "$pp" ]; then
         local t ph; t="$(test_image "$pp")"; ph="$(printf '%s' "$t" | cut -d'|' -f2)"
         if [ -n "$bsha" ] && [ "$ph" = "$bsha" ]; then log ERROR "$(L 'ERROR: patched == base. NO fake patch accepted.' 'FEHLER: gepatcht == Basis. KEIN Fake-Patch.')"
         elif [ "${t%%|*}" = "PASS" ]; then PATCHED_IMAGE="$pp"; PATCHED_SHA="$ph"; log SUCCESS "Patched registered: $pp"
         else printf '%s\n' "$(L 'Image check FAIL.' 'Image-Pruefung FAIL.')"; fi
       fi ;;
  esac
  pause_tt
}
screen_backup() {
  header "$(L 'Step 6 - Backup (mandatory before flash)' 'Step 6 - Backup (Pflicht vor Flash)')"; printf '\n'
  local base; base="$(patch_base)"
  local bsrc="${base%%|*}"; base="${base#*|}"
  local bimg="${base%%|*}"; local blabel="${base##*|}"
  if [ -z "$bimg" ] || [ ! -f "$bimg" ]; then
    if [ "$bsrc" = "rom" ]; then printf '%s\n' "$(L 'No ROM base image -> recovery export first.' 'Kein ROM-Basis-Image -> erst Recovery-Export.')"
    else printf '%s\n' "$(L 'No stock image -> step 4 first.' 'Kein Stock -> erst Step 4.')"; fi
    pause_tt; return
  fi
  printf '%s %s\n' "$(L 'Backing up base from:' 'Sichere Basis aus:')" "$blabel"
  do_backup "$bimg" && printf 'Backup: %s\n' "$BACKUP_DIR"
  pause_tt
}
screen_flash() {
  header "$(L 'Step 7 - Flash (only after safety gate)' 'Step 7 - Flash (nur nach Safety-Gate)')"; printf '\n'
  if [ "$MODE" != "fastboot" ]; then
    printf '%s' "$(L "'adb reboot bootloader' now? [y/N]: " "'adb reboot bootloader' jetzt? [y/N]: ")"; iread -r k
    case "$k" in y|Y|j|J) adb_run reboot bootloader 2>/dev/null; for _ in $(seq 1 30); do sleep 1; detect_mode; [ "$MODE" = "fastboot" ] && break; done ;; esac
  fi
  safe_flash "$PATCHED_IMAGE" || true
  tw=""
  if [ -n "${TWRP_IMAGE:-}" ] && [ -f "$TWRP_IMAGE" ]; then tw="$TWRP_IMAGE"; fi
  if [ -z "$tw" ]; then tw="$(read_slot | cut -d'|' -f2)"; [ -f "$tw" ] || tw=""; fi
  if [ -n "$tw" ]; then
    printf '\n%s' "$(L 'Slot holds Magisk. Install known TWRP now (one tap, overwrites Magisk slot)? [y/N]: ' 'Slot hat Magisk. Bekanntes TWRP jetzt installieren (one tap, ueberschreibt Magisk-Slot)? [j/N]: ')"; iread -r tw2
    case "$tw2" in y|Y|j|J) TWRP_IMAGE="$tw"; twrp_flash "$tw" || true ;; esac
  fi
  pause_tt
}
screen_verify() {
  header "$(L 'Step 8+9 - Reboot (Huawei procedure) + verify (real)' 'Step 8+9 - Reboot + Verify (echt)')"; printf '\n'
  local r; if r="$(verify_root)"; then printf 'Result: %s\n' "$r"; else printf 'Result: %s (see above)\n' "$r"; fi
  if [ "$r" = "ROOTED" ] && [ -z "${SKIP_FIX_OFFER:-}" ]; then
    printf '\n%s' "$(L 'Root is live right now - install the permanent APTouch fix? (service.d + immediate stop, non-destructive, removable) [Y/n]: ' 'Root ist gerade live - permanenten APTouch-Fix installieren? (service.d + Sofort-Stopp, zerstoerungsfrei, entfernbar) [J/n]: ')"; iread -r pf
    case "$pf" in ""|y|Y|j|J)
      if install_persist_fixes; then printf '%s\n' "$(L 'APTouch fix active now and on every rooted boot.' 'APTouch-Fix jetzt aktiv und bei jedem gerooteten Boot.')"
      else printf '%s\n' "$(L 'Persist install failed - offered again at every verified root.' 'Persist-Install fehlgeschlagen - wird bei jedem verifizierten Root erneut angeboten.')"; fi ;;
    esac
    printf '%s\n' "$(L 'Want every power-on rooted (no Vol-Up trick)? Boot tricks -> persistent boot. Conscious choice: it needs an eRecovery wipe.' 'Jeden Power-On gerootet (ohne Vol-Up-Trick)? Boot-Tricks -> persistenter Boot. Bewusste Entscheidung: braucht eRecovery-Wipe.')"
  fi
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
  printf '%s\n' "$out" >> "$TTLOG"
  if printf '%s\n' "$out" | flash_verdict; then
    log SUCCESS "$(L 'System flash OK. Next: fastboot reboot -> eRecovery wipe -> setup.' 'System-Flash OK. Weiter: Reboot -> eRecovery Wipe -> Setup.')"; return 0
  fi
  log ERROR "$(L 'System flash not OK (see result above).' 'System-Flash nicht OK (siehe Ergebnis oben).')"; return 1
}
screen_flashsystem() {
  header "$(L 'Install ROM / GSI system image (fully guided)' 'ROM / GSI installieren (voll gefuehrt)')"; printf '\n'
  printf '%s\n' "- Base EMUI 8/9/9.1. Backup storage. Reset only via stock recovery." "- fastboot flash system <gsi.img>, then eRecovery wipe. Slim builds if system is small."
  local adv; adv="$(profile_gsi_advice)"; [ -n "$adv" ] && printf 'Profile advice: %s\n' "$adv"
  printf '%s' "$(L 'GSI image path (*-arm64_*.img, unpacked - Enter = download working GSI): ' 'GSI-Image-Pfad (entpackt - Enter = Working-GSI laden): ')"; iread -r img
  if [ -z "$img" ]; then
    DL_READY=""
    download_rom || true
    img="$DL_READY"
  fi
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
  printf '%s\n' "$out" >> "$TTLOG"
  if printf '%s\n' "$out" | flash_verdict; then log SUCCESS "$(L 'TWRP flash OK. Boot: hold Vol-Up.' 'TWRP-Flash OK. Boot: Vol-Up halten.')"; save_slot "twrp" "$img"; return 0; fi
  log ERROR "$(L 'TWRP flash not OK (see result above).' 'TWRP-Flash nicht OK (siehe Ergebnis oben).')"; return 1
}
screen_twrp() {
  header "$(L 'TWRP path (guide + guided flash)' 'TWRP-Pfad (Anleitung + Flash)')"; printf '\n'
  printf '%s\n' "$(L 'Sources (device-exact builds only): XDA P10 Plus TWRP 3.2.1-0 (oreo).' 'Quellen (nur genaue Builds): XDA P10 Plus TWRP 3.2.1-0 (oreo).')"
  printf '%s\n' "$(L 'Rules: shared slot with Magisk; backup first; never wipe userdata in TWRP; boot with Vol-Up.' 'Regeln: Slot mit Magisk teilen; erst Backup; nie userdata in TWRP wipen; Boot mit Vol-Up.')"
  printf '%s' "$(L 'TWRP image path: ' 'TWRP-Image-Pfad: ')"; iread -r img
  if [ -z "$img" ] || [ ! -f "$img" ]; then log WARNING "$(L 'Invalid path, aborting.' 'Pfad ungueltig, Abbruch.')"; pause_tt; return; fi
  TWRP_IMAGE="$img"
  if twrp_flash "$img"; then
    if [ -n "$PATCHED_IMAGE" ] && [ -f "$PATCHED_IMAGE" ]; then
      printf '\n%s' "$(L 'Slot now holds TWRP. Flash Magisk back now (one tap, same safety gate)? [Y/n]: ' 'Slot hat jetzt TWRP. Jetzt Magisk zurueckflashen (one tap, gleiches Safety-Gate)? [J/n]: ')"; iread -r sw
      case "$sw" in ""|y|Y|j|J)
        if safe_flash "$PATCHED_IMAGE" --yes; then printf '%s\n' "$(L 'Slot holds Magisk again. Reboot with Vol-Up + Power for root.' 'Slot hat wieder Magisk. Reboot mit Vol-Up + Power fuer Root.')"; fi ;;
      esac
    fi
  fi
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
guided_wipe() { # double-confirmed userdata wipe, never automatic; eRecovery fallback
  local yes="${1:-}"
  detect_mode
  if [ "$MODE" != "fastboot" ]; then
    log ERROR "$(L 'Wipe needs fastboot mode. Reboot to fastboot first.' 'Wipe braucht Fastboot. Erst nach Fastboot booten.')"; return 1
  fi
  printf '\nWARNING\n%s\n' "$(L 'This erases ALL user data (apps, photos, settings). System stays.' 'Das loescht ALLE Nutzerdaten. System bleibt.')"
  if [ -z "$yes" ]; then
    printf '%s' "$(L "Type 'WIPE' (1/2): " "'WIPE' tippen (1/2): ")"; iread -r a
    [ "$a" = "WIPE" ] || return 1
    printf '%s' "$(L "Type 'YES' (2/2): " "'JA' (2/2): ")"; iread -r b
    { [ "$b" = "YES" ] || [ "$b" = "JA" ]; } || return 1
  else log WARNING "CLI --yes: explicit wipe consent documented."; fi
  local out; out="$(fb_run erase userdata 2>&1)"
  printf '%s\n' "$out" >> "$TTLOG"
  if printf '%s\n' "$out" | flash_verdict; then
    log SUCCESS "$(L 'userdata erase reported OK.' 'userdata-erase gemeldet OK.')"; return 0
  fi
  printf '%s\n' "$(L 'Device refused erase. Fallback (manual, safe): stock eRecovery (Vol-Up 3s) -> Wipe data / factory reset -> confirm on phone.' 'Geraet lehnt ab. Fallback (manuell, sicher): Stock-eRecovery (Vol-Up 3s) -> Wipe -> am Handy bestaetigen.')"
  return 1
}
screen_wipe() {
  header "$(L 'Wipe userdata (guided, double-confirmed)' 'Userdata wipen (gefuehrt, doppelt)')"
  if guided_wipe; then printf '%s\n' "$(L 'Wipe done. Reboot and set up again.' 'Wipe fertig. Neu starten + einrichten.')"; fi
  pause_tt
}
screen_reinstall() {
  header "$(L 'Full reinstall (guided: ROM choice, optional wipe, flash, verify)' 'Komplett-Reinstall (gefuehrt)')"
  printf '\n%s\n' "$(L 'Parts exist (flash-system/TWRP/restore). This chains: ROM choice -> backup reminder -> optional wipe -> flash -> reboot -> verify.' 'Teile existieren. Kette: ROM-Wahl -> Backup-Hinweis -> optional Wipe -> Flash -> Reboot -> Verify.')"
  printf '%s' "$(L '[1] Custom ROM / GSI image  [2] Stock full firmware path  [Enter] back: ' '[1] Custom-ROM / GSI  [2] Stock-Full-Weg  [Enter] zurueck: ')"; iread -r k
  case "$k" in
    1) printf '%s' "$(L 'Wipe userdata first? [W]=wipe (double-confirmed) [Enter]=skip: ' 'Userdata vorher wipen? [W]=wipen (doppelt) [Enter]=ueberspringen: ')"; iread -r w
       case "$w" in [Ww]) guided_wipe || true ;; esac
       screen_flashsystem ;;
    2) printf '\n%s\n' "$(L 'Stock return runs via HiSuite (official) or service flow with the full firmware (step 3 downloader). Then stock eRecovery wipe + setup.' 'Stock-Rueckweg via HiSuite (offiziell) oder Service-Flow mit Full-Firmware. Dann eRecovery-Wipe + Setup.')"
       printf '%s\n' "$(L 'Already extracted SYSTEM.img? Flash it via Install ROM with that file.' 'SYSTEM.img schon extrahiert? Per ROM-Installation damit flashen.')" ;;
  esac
  pause_tt
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
save_root_state() { # ROOT [BOOTMODE] -> merges last_root into state file
  local root="$1" bmode="${2:-}" f
  f="$(state_file)"
  if command -v python3 >/dev/null 2>&1; then
    ST_ROOT="$root" ST_BMODE="$bmode" ST_FILE="$f" python3 - <<'PYEOF' 2>/dev/null || true
import json,os,datetime
f=os.environ['ST_FILE']
try: st=json.load(open(f))
except Exception: st={"goal":"","steps":[]}
lr={"state":os.environ['ST_ROOT'],"timestamp":datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")}
if os.environ['ST_BMODE']: lr["boot_mode"]=os.environ['ST_BMODE']
elif isinstance(st.get("last_root"),dict) and st["last_root"].get("boot_mode"): lr["boot_mode"]=st["last_root"]["boot_mode"]
st["last_root"]=lr
st["updated"]=datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
json.dump(st,open(f,"w"),indent=2)
PYEOF
  fi
  log INFO "Root state persisted: $root"
}
save_slot() { # occupant [detail] -> slot state in state file (shared recovery_ramdisk slot)
  local occ="$1" det="${2:-}" f
  f="$(state_file)"
  if command -v python3 >/dev/null 2>&1; then
    ST_OCC="$occ" ST_DET="$det" ST_FILE="$f" python3 - <<'PYEOF' 2>/dev/null || true
import json,os,datetime
f=os.environ['ST_FILE']
try: st=json.load(open(f))
except Exception: st={"goal":"","steps":[]}
st["slot"]={"occupant":os.environ['ST_OCC'],"detail":os.environ['ST_DET'],"timestamp":datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")}
st["updated"]=datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
json.dump(st,open(f,"w"),indent=2)
PYEOF
  fi
  log INFO "Slot state: $occ"
}
read_slot() { # prints occupant|detail (unknown when unset)
  local f; f="$(state_file)"
  if [ -f "$f" ] && command -v python3 >/dev/null 2>&1; then
    python3 -c "import json,sys; s=json.load(open(sys.argv[1])).get('slot',{}); print(str(s.get('occupant','unknown'))+'|'+str(s.get('detail','')))" "$f" 2>/dev/null && return 0
  fi
  printf 'unknown|'
}
install_persist_fixes() { # service.d boot scripts (aptouch+smartpa); needs uid=0
  local id
  id="$(adb_run shell su -c id 2>&1 | tr -d '\r\n')"
  case "$id" in *uid=0*) ;; *) log ERROR "$(L 'No live root (need uid=0). Boot rooted first, grant Magisk, retry.' 'Kein live Root (uid=0 noetig). Erst gerootet booten, freigeben, erneut.')"; return 1 ;; esac
  local stopout
  stopout="$(adb_run shell "su -c 'stop aptouch' 2>&1" | tr -d '\r')"
  log INFO "Immediate stop aptouch: $stopout"
  local ok=1 name tmp
  for name in 000-treblemanager-aptouch.sh 000-treblemanager-smartpa.sh; do
    tmp="/tmp/$name"
    if [ "$name" = "000-treblemanager-aptouch.sh" ]; then
      printf '#!/system/bin/sh\n# trebleManager: stop aptouch (touchscreen edges)\nstop aptouch\n' > "$tmp"
    else
      printf '#!/system/bin/sh\n# trebleManager: smartpa speaker fix\nchown root:audio /dev/nxp_smartpa_dev\nchmod 0660 /dev/nxp_smartpa_dev\n' > "$tmp"
    fi
    adb_run push "$tmp" "/sdcard/Download/$name" >/dev/null 2>&1
    adb_run shell "su -c 'cp /sdcard/Download/$name /data/adb/service.d/$name && chmod 755 /data/adb/service.d/$name' 2>&1" >/dev/null 2>&1
    if adb_run shell "su -c 'ls -l /data/adb/service.d/$name' 2>&1" | grep -q "$name"; then
      log SUCCESS "Persisted: /data/adb/service.d/$name"
    else
      log ERROR "FAILED: $name"; ok=0
    fi
    rm -f "$tmp"
  done
  [ "$ok" = 1 ] && save_root_state "ROOTED+persisted-fixes"
  [ "$ok" = 1 ]
}
screen_persist() {
  header "$(L 'Persist root fixes (service.d, survives reboot)' 'Root-Fixes persistieren (service.d, rebootfest)')"; printf '\n'
  printf '%s\n' "$(L 'Honest scope: P10 Magisk lives in recovery_ramdisk - every ROOTED boot needs Vol-Up + Power. No safe way around that.' 'Ehrlich: P10-Magisk lebt in recovery_ramdisk - jeder ROOT-Boot braucht Vol-Up + Power. Kein sicherer Weg daran vorbei.')"
  printf '%s\n' "$(L 'What persists: aptouch + speaker fixes as service.d scripts, plus verified root state.' 'Was persistiert: aptouch + Speaker als service.d, plus Root-Status.')"
  printf '%s' "$(L 'Install now? Needs live root. [Y/n]: ' 'Jetzt installieren? Braucht live Root. [J/n]: ')"; iread -r a
  case "$a" in ""|y|Y|j|J)
    if install_persist_fixes; then printf '%s\n' "$(L 'Fixes apply on every rooted boot automatically.' 'Fixes greifen bei jedem gerooteten Boot automatisch.')"; fi ;;
  esac
  pause_tt
}
screen_bootkeys() {
  header "$(L 'Huawei boot mechanism (from #2542, exact)' 'Huawei Boot-Mechanismus (aus #2542, exakt)')"; printf '\n'
  printf '%s\n' "$(L '- Magisk boot: Vol-Up + Power until Huawei logo, then release (boot cheat).' '- Magisk-Boot: Vol-Up + Power bis Logo, dann loslassen.')"
  printf '%s\n' "$(L '- Without trick: stock boot (no root). NOT persistent.' '- Ohne Trick: Stock-Boot (kein Root). NICHT persistent.')"
  printf '%s\n' "$(L '- Persistent Magisk boot (no-ramdisk devices like P10): SET via eRecovery wipe trick (discussion step 13) -> every power-on boots Magisk; CLEAR via Vol-Up+Vol-Down+Power with NO /dload present (step 14).' '- Persistenter Magisk-Boot (Geraete ohne Ramdisk wie P10): SETZEN via eRecovery-Wipe-Trick (Schritt 13) -> jeder Boot mit Magisk; LOESCHEN via Vol-Up+Vol-Down+Power OHNE /dload (Schritt 14).')"
  printf '%s\n' "$(L '- /dload must NOT be on storage, else EMUI updater instead of recovery.' '- /dload darf NICHT vorhanden sein, sonst EMUI-Updater.')"
  local bm=""
  if command -v python3 >/dev/null 2>&1 && [ -f "$(state_file)" ]; then
    bm="$(python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(d.get('last_root',{}).get('boot_mode',''))" "$(state_file)" 2>/dev/null)"
  fi
  [ -n "$bm" ] && printf 'Persisted boot mode: %s\n' "$bm"
  printf '\n%s' "$(L '[1] Guide: SET persistent boot  [2] Guide: CLEAR it  [3] Verify (normal reboot, then check)  [Enter] back: ' '[1] SET anleiten  [2] CLEAR anleiten  [3] Verifizieren (normal rebooten, dann check)  [Enter] zurueck: ')"; iread -r k
  case "$k" in
    1) printf '%s\n' "$(L '1. Boot once into Magisk (Vol-Up + Power). 2. Yellow-text screen -> eRecovery (Vol-Up 3s). 3. Confirm wipe + reboot. 4. Every power-on now boots Magisk (wipe NOT executed).' '1. Einmal ins Magisk-System (Vol-Up + Power). 2. Gelb-Text -> eRecovery (Vol-Up 3s). 3. Wipe bestaetigen + Reboot. 4. Jeder Boot jetzt Magisk (Wipe NICHT ausgefuehrt).')"
       printf '%s' "$(L "Type 'SET' then 'YES' (changes every boot, reversible): " "'SET' dann 'YES' (aendert jeden Boot, reversibel): ")"; iread -r a
       [ "$a" = "SET" ] || { pause_tt; return; }
       printf '%s' "Type 'YES': "; iread -r b
       if [ "$b" = "YES" ] || [ "$b" = "JA" ]; then save_root_state "ROOTED" "persistent-pending"; printf '%s\n' "$(L 'Do the steps on the phone now, then use [3].' 'Jetzt Schritte am Handy, dann [3].')"; fi ;;
    2) printf '%s\n' "$(L '1. Remove /dload! 2. Power off. 3. Vol-Up+Vol-Down+Power until logo. 4. EMUI upgrade-fail screen -> reboot -> clean.' '1. /dload entfernen! 2. Ausschalten. 3. Vol-Up+Vol-Down+Power bis Logo. 4. EMUI-Fehler -> Reboot -> clean.')"
       printf '%s' "$(L 'Done on phone? [Y/n]: ' 'Am Handy erledigt? [J/n]: ')"; iread -r c
       case "$c" in ""|y|Y|j|J) save_root_state "UNKNOWN" "cheat" ;; esac ;;
    3) printf '%s\n' "$(L 'Reboot NORMALLY now (no keys), wait for Android, then Enter here.' 'Jetzt NORMAL rebooten (keine Tasten), Android abwarten, dann Enter.')"
       pause_tt; detect_mode
       if [ "$MODE" != "android" ]; then printf '%s\n' "$(L 'No Android yet.' 'Noch kein Android.')"; pause_tt; return; fi
       if adb_run shell su -c id 2>&1 | grep -q 'uid=0'; then
         save_root_state "ROOTED" "persistent"
         printf '%s\n' "$(L 'PERSISTENT ROOT CONFIRMED (uid=0, no cheat).' 'PERSISTENTER ROOT BESTAETIGT (uid=0, kein Cheat).')"
       else
         save_root_state "NOT_ROOTED" "cheat"
         printf '%s\n' "$(L 'Not persistent: normal boot unrooted.' 'Nicht persistent: normaler Boot ungerootet.')"
       fi ;;
  esac
  pause_tt
}
main_menu() {
  find_tools
  # Guided preflight: resolve missing tools with the user until all
  # prerequisites are met (or the user aborts) - never a dead-end block.
  while true; do
    if preflight >/dev/null 2>&1; then break; fi
    printf '\n%s\n' "$(L 'Some prerequisites are missing - fixing them together now.' 'Einige Voraussetzungen fehlen - beheben wir sie gemeinsam.')"
    ensure_tool adb || { printf '%s\n' "$(L 'Aborted (adb still missing).' 'Abgebrochen (adb fehlt weiter).')"; return 1; }
    ensure_tool fastboot || { printf '%s\n' "$(L 'Aborted (fastboot still missing).' 'Abgebrochen (fastboot fehlt weiter).')"; return 1; }
    ensure_scrcpy
    save_config
    find_tools
  done
  ensure_scrcpy
  save_config
  select_target || return 1
  load_rom
  find_tools; detect_mode
  while true; do
    menu "$(L 'Main menu - what do you want to do?' 'Hauptmenue - was willst du tun?')" \
      "$(L 'Guided run (asks system + goal, runs automatically)' 'Gefuehrter Lauf (fragt System + Ziel, laeuft automatisch)')" \
      "$(L 'Check device (status, detect, analyze, logs)' 'Geraet pruefen (Status, Detect, Analyse, Logs)')" \
      "$(L 'Single steps (individual screens)' 'Einzel-Steps (einzelne Screens)')" \
      "$(L 'Workflows (goals, resume, reinstall)' 'Workflows (Ziele, Resume, Reinstall)')" \
      "$(L 'Settings (my system)' 'Einstellungen (mein System)')" \
      "$(L 'Exit' 'Beenden')"
    c="$REPLY_MENU"
    case "$c" in
      -1|5) log SUCCESS "$(L 'Exiting. Log: ' 'Beendet. Log: ')$TTLOG"; break ;;
      0) wizard ;; 1) menu_check ;; 2) menu_steps ;; 3) menu_workflows ;; 4) menu_settings ;;
    esac
  done
}
menu_check() {
  while true; do
    menu "$(L 'Check device (read-only)' 'Geraet pruefen (read-only)')" \
      "$(L 'Status overview' 'Status-Uebersicht')" "Step 1 - Detect" "Step 2 - Analyze" \
      "$(L 'Logs + diagnostic ZIP' 'Logs + Diagnose-ZIP')" "$(L 'Back' 'Zurueck')"
    c="$REPLY_MENU"
    case "$c" in
      -1|4) return ;;
      0) status_screen ;; 1) screen_detect ;; 2) screen_analyze ;; 3) screen_tools ;;
    esac
  done
}
menu_steps() {
  while true; do
    menu "$(L 'Single steps (extras)' 'Einzel-Steps (Extras)')" \
      "$(L 'Firmware (find + download)' 'Firmware (finden + laden)')" \
      "$(L 'Extract stock recovery' 'Stock-Recovery extrahieren')" \
      "$(L 'Recovery export (custom ROMs)' 'Recovery-Export (Custom-ROMs)')" \
      "$(L 'Magisk patch' 'Magisk-Patch')" "$(L 'Backup' 'Backup')" \
      "$(L 'Flash (safety gate)' 'Flash (Safety-Gate)')" \
      "$(L 'Install ROM / GSI' 'ROM / GSI installieren')" \
      "$(L 'TWRP path' 'TWRP-Pfad')" "$(L 'Root methods' 'Root-Methoden')" \
      "$(L 'Compatibility registry' 'Kompatibilitaets-Registry')" \
      "$(L 'Persist root fixes' 'Root-Fixes persistieren')" \
      "$(L 'Unlock guide' 'Unlock-Anleitung')" "$(L 'Kernels + fixes' 'Kernel + Fixes')" \
      "$(L 'Boot tricks' 'Boot-Tricks')" "$(L 'Wipe userdata' 'Userdata wipen')" \
      "Restore / Unroot" "$(L 'Reboot + verify' 'Reboot + Verify')" \
      "$(L 'Tools' 'Tools')" "$(L 'Back' 'Zurueck')"
    c="$REPLY_MENU"
    case "$c" in
      -1|18) return ;;
      0) screen_firmware ;; 1) screen_extract ;; 2) screen_export ;; 3) screen_patch ;;
      4) screen_backup ;; 5) screen_flash ;; 6) screen_flashsystem ;; 7) screen_twrp ;;
      8) screen_rootmethods ;; 9) screen_compat ;; 10) screen_persist ;; 11) screen_unlock ;;
      12) screen_kernelfixes ;; 13) screen_bootkeys ;; 14) guided_wipe || true; pause_tt ;;
      15) do_restore "" "" || true; pause_tt ;; 16) screen_verify ;; 17) screen_tools ;;
    esac
  done
}
menu_workflows() {
  while true; do
    menu "$(L 'Workflows (planner + resume)' 'Workflows (Planner + Resume)')" \
      "$(L 'Workflow goals' 'Workflow-Ziele')" \
      "$(L 'Resume saved workflow' 'Gespeicherten Workflow fortsetzen')" \
      "$(L 'Full reinstall' 'Komplett-Reinstall')" \
      "$(L 'Back' 'Zurueck')"
    c="$REPLY_MENU"
    case "$c" in
      -1|3) return ;;
      0) screen_goals ;; 1) screen_resume ;; 2) screen_reinstall ;;
    esac
  done
}
menu_settings() {
  while true; do
    load_rom
    menu "$(L 'Settings' 'Einstellungen') $(L 'Phone runs:' 'Handy laeuft mit:') $(rom_label "$INSTALLED_ROM")" \
      "$(L 'My system (Stock / custom ROM)' 'Mein System (Stock / Custom-ROM)')" \
      "$(L 'Back' 'Zurueck')"
    c="$REPLY_MENU"
    case "$c" in
      -1|1) return ;;
      0) select_rom; pause_tt ;;
    esac
  done
}
wizard() { # ROM-aware guided path: detect -> analyze (+ROM question) -> goal words -> plan with SKIPs -> run
  load_rom
  screen_detect
  screen_analyze
  if [ -z "$INSTALLED_ROM" ]; then select_rom; fi
  local rlabel; rlabel="$(rom_label "$INSTALLED_ROM")"
  local pbsrc; pbsrc="$(patch_base)"; pbsrc="${pbsrc%%|*}"
  local custom=0; [ "$pbsrc" = "rom" ] && custom=1
  header "$(L 'What do you want to do?' 'Was willst du tun?')"; printf '\n'
  printf '%s %s\n\n' "$(L 'Your system:' 'Dein System:')" "$rlabel"
  printf '%s\n' "$(L '[1] Root only (keep my system exactly as it is)' '[1] Nur Root (mein System bleibt exakt wie es ist)')"
  printf '%s\n' "$(L '[2] Install a custom ROM / GSI' '[2] Custom-ROM / GSI installieren')"
  printf '%s\n' "$(L '[3] Back to stock' '[3] Zurueck zu Stock')"
  printf '%s' "$(L '[Enter] back: ' '[Enter] zurueck: ')"; iread -r gk
  local steps="" skipped="" goal="" autofix=0
  case "$gk" in
    1) goal="$(L 'Root only' 'Nur Root')"
       autofix=1
       select_root_target || return
       rlabel="$(rom_label "$INSTALLED_ROM")"
       pbsrc="$(patch_base)"; pbsrc="${pbsrc%%|*}"
       custom=0; [ "$pbsrc" = "rom" ] && custom=1
       if [ "$custom" = 1 ]; then
         steps="screen_export screen_patch screen_backup screen_flash screen_verify"
         skipped="$(L 'stock firmware search/download - not needed, base comes from ' 'Stock-Firmware-Suche/Download - nicht noetig, Basis kommt aus ')$rlabel|$(L 'stock UPDATE.APP extract - not needed' 'Stock-UPDATE.APP-Extrakt - nicht noetig')"
       else
         steps="screen_firmware screen_extract screen_patch screen_backup screen_flash screen_verify"
       fi ;;
    2) goal="$(L 'Install custom ROM' 'Custom-ROM installieren')"
       TARGET_ANDROID=""; TARGET_LABEL=""; TARGET_URL=""; TARGET_FILE=""
       select_target_image || return
       rlabel="$TARGET_LABEL"
       img="$(find_local_system_image)"
       if [ -z "$img" ] && [ -n "$TARGET_URL" ]; then
         printf '\n%s\n' "$(L 'No local image - downloading working system now.' 'Kein lokales Image - lade Working-System jetzt.')"
         DL_READY=""
         if download_rom "$TARGET_LABEL" --yes; then img="$DL_READY"; fi
       fi
       if [ -z "$img" ] || [ ! -f "$img" ]; then
         printf '\n%s\n' "$(L 'No image ready - manual install screen next.' 'Kein Image bereit - weiter mit manuellem Install-Screen.')"
         steps="screen_compat screen_flashsystem screen_verify"
       else
         printf '\n%s %s\n' "$(L 'Preset image from resolver:' 'Preset-Image aus Resolver:')" "$img"
         FLASH_PRESET="$img"
         steps="screen_compat screen_flashsystem screen_verify"
       fi
       skipped="$(L 'root/patch/flash - run wizard again with [1] afterwards for root' 'Root/Patch/Flash - danach Wizard erneut mit [1] starten falls Root gewuenscht')" ;;
    3) goal="$(L 'Back to stock' 'Zurueck zu Stock')"
       steps="screen_firmware screen_reinstall"
       skipped="$(L 'Magisk patch/flash - stock return needs no root steps' 'Magisk-Patch/Flash - Stock-Rueckweg braucht keine Root-Steps')" ;;
    *) return ;;
  esac
  header "$(L 'Your path:' 'Dein Weg:') $rlabel -> $goal"; printf '\n'
  if [ "$pbsrc" = "stock-gsi" ]; then
    printf '%s\n\n' "$(L 'GSI detected: recovery is untouched stock, so the normal stock steps below are correct.' 'GSI erkannt: Recovery ist unberuehrt Stock, also sind die Stock-Steps unten korrekt.')"
  fi
  local n=1 s
  for s in $steps; do
    case "$s" in
      screen_firmware) printf ' [%d] %s\n' "$n" "$(L 'Stock firmware (find + download)' 'Stock-Firmware (finden + laden)')" ;;
      screen_extract) printf ' [%d] %s\n' "$n" "$(L 'Extract stock recovery image' 'Stock-Recovery-Image extrahieren')" ;;
      screen_export) printf ' [%d] %s\n' "$n" "$(L 'Get patch base from YOUR rom package' 'Patch-Basis aus DEINEM ROM-Paket holen')" ;;
      screen_patch) printf ' [%d] %s\n' "$n" "$(L 'Magisk patch (on your phone)' 'Magisk-Patch (an deinem Handy)')" ;;
      screen_backup) printf ' [%d] %s\n' "$n" "$(L 'Backup' 'Backup')" ;;
      screen_flash) printf ' [%d] %s\n' "$n" "$(L 'Flash + safety gate' 'Flash + Safety-Gate')" ;;
      screen_verify) printf ' [%d] %s\n' "$n" "$(L 'Reboot + verify root' 'Reboot + Root verify')" ;;
      screen_compat) printf ' [%d] %s\n' "$n" "$(L 'Check ROM compatibility' 'ROM-Kompatibilitaet pruefen')" ;;
      screen_flashsystem) printf ' [%d] %s\n' "$n" "$(L 'Install ROM image' 'ROM-Image installieren')" ;;
      screen_reinstall) printf ' [%d] %s\n' "$n" "$(L 'Stock return guide' 'Stock-Rueckweg-Anleitung')" ;;
    esac
    n=$((n+1))
  done
  local old_ifs="$IFS"; IFS='|'
  for s in $skipped; do [ -n "$s" ] && printf ' [SKIP] %s\n' "$s"; done
  IFS="$old_ifs"
  printf '\n%s' "$(L 'Run now? [Y/n]: ' 'Jetzt starten? [J/n]: ')"; iread -r yn
  case "$yn" in ""|y|Y|j|J) ;; *) return ;; esac
  if [ "$autofix" = 1 ]; then SKIP_FIX_OFFER=1; else unset SKIP_FIX_OFFER; fi
  for s in $steps; do "$s" || true; done
  unset SKIP_FIX_OFFER
  if [ "$autofix" = 1 ]; then
    printf '\n%s\n' "$(L 'Automatic: permanent APTouch fix attempt (non-destructive service.d + immediate stop). Skips cleanly without live root.' 'Automatisch: permanenter APTouch-Fix-Versuch (zerstoerungsfreies service.d + Sofort-Stopp). Ohne live Root sauber uebersprungen.')"
    detect_mode
    if install_persist_fixes; then printf '%s\n' "$(L 'APTouch fix active now and on every rooted boot.' 'APTouch-Fix jetzt aktiv und bei jedem gerooteten Boot.')"
    else printf '%s\n' "$(L 'Fix not installed (no live root or install failed) - offered again at every verified root.' 'Fix nicht installiert (kein live Root oder fehlgeschlagen) - wird bei jedem verifizierten Root erneut angeboten.')"; fi
    printf '%s\n' "$(L 'Want every power-on rooted (no Vol-Up trick)? Boot tricks -> persistent boot. Conscious choice: it needs an eRecovery wipe.' 'Jeden Power-On gerootet (ohne Vol-Up-Trick)? Boot-Tricks -> persistenter Boot. Bewusste Entscheidung: braucht eRecovery-Wipe.')"
  fi
  log SUCCESS "$(L 'Wizard path completed: ' 'Wizard-Weg fertig: ')$goal"
}

# ---------------------------------------------------------------- CLI
show_help() {
  printf 'Huawei P10 Root Manager v%s\n' "$TTVERSION"
  printf 'Usage: treble-toolkit.sh [detect|devices|analyze|firmware|download|download-rom|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|persist|validate|verify|restore|wipe|reinstall|rom|diagnostic|dump-partitions|dump-properties|dump-vendor|dump-logs|preflight|recon|status|workflow|resume|root|setup|wizard|help] [--goal <id>] [--mode safe|unattended|developer] [--json] [--yes] [--image <path>] [--firmware-file <url|path>] [--anonymize] [--no-reboot]\n'
  printf '%s\n' "$(L 'No args: TUI. Download/flash/restore need --yes.' 'Ohne Args: TUI. Download/Flash/Restore brauchen --yes.')"
}
CMD=""; JSON=""; YES=""; IMAGE=""; FWFILE=""; ANON=""; NOREBOOT=""; RUNMODE="safe"; GOAL=""
for a in "$@"; do
  if [ -n "${WANTVAL:-}" ]; then
    case "$WANTVAL" in
      --image) IMAGE="$a" ;; --firmware-file) FWFILE="$a" ;;
      --mode) RUNMODE_REQ="$a" ;; --goal) GOAL="$a" ;;
    esac
    WANTVAL=""
    continue
  fi
  case "$a" in
    detect|devices|analyze|firmware|download|download-rom|extract|export|patch|backup|flash|flash-system|twrp|root-methods|compat|persist|validate|verify|restore|wipe|reinstall|rom|diagnostic|dump-partitions|dump-properties|dump-vendor|dump-logs|preflight|recon|status|workflow|resume|root|setup|wizard|help) [ -z "$CMD" ] && CMD="$a" ;;
    --json) JSON=1 ;; --yes) YES=1 ;; --anonymize) ANON=1 ;; --no-reboot) NOREBOOT=1 ;;
    --mode|--goal|--image|--firmware-file) WANTVAL="$a" ;;
    *) if [ "$CMD" = "rom" ] || [ "$CMD" = "download-rom" ]; then
         if [ -z "${ROMARG:-}" ]; then ROMARG="$a"; else ROMVAL="$a"; fi
       fi ;;
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
  download-rom)
    if [ -n "$YES" ]; then DL_READY=""; download_rom "${ROMARG:-1}" --yes || exit 1; img="$DL_READY"
    else DL_READY=""; download_rom "${ROMARG:-}" || exit 1; img="$DL_READY"; fi
    if [ -n "$JSON" ]; then printf '{"ready":"%s","kind":"%s"}\n' "$img" "$(image_kind "$img")"
    else printf 'Ready: %s\n' "$img"; fi ;;
  extract)
    found="$(find "$FIRM_DIR" "$DATA_DIR" -maxdepth 3 \( -iname 'RECOVERY_RAMDISK.img' -o -iname 'RECOVERY_RAMDIS.img' -o -iname 'recovery_ramdisk.img' \) -type f 2>/dev/null | head -1)"
    [ -z "$found" ] && { printf '%s\n' "$(L 'No RECOVERY_RAMDIS(K).img found.' 'Keine RECOVERY_RAMDIS(K).img.')"; exit 3; }
    t="$(test_image "$found")"; STOCK_IMAGE="$found"; STOCK_SHA="$(printf '%s' "$t" | cut -d'|' -f2)"
    if [ -n "$JSON" ]; then printf '{"path":"%s","check":"%s"}\n' "$found" "$t"; else printf '%s: %s\n' "$found" "$t"; fi
    [ "${t%%|*}" = "PASS" ] || exit 3 ;;
  export)
    rom="${IMAGE:-$FWFILE}"
    if [ -z "$rom" ]; then rom="$(find "$ROM_DIR" -maxdepth 1 \( -iname '*.zip' -o -iname '*.img' -o -iname '*.tar' -o -iname '*.tar.gz' -o -iname '*.tgz' -o -iname '*.gz' -o -iname '*.xz' \) -type f 2>/dev/null | head -1)"; fi
    [ -z "$rom" ] && { printf '%s\n' "$(L 'No ROM package. Place .img/.img.gz/.img.xz/.zip/.tar.gz in data/roms/.' 'Kein ROM-Paket. .img/.img.gz/.img.xz/.zip/.tar.gz nach data/roms/.')"; exit 3; }
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
  wipe)
    if [ -z "$YES" ]; then printf '%s\n' "$(L 'Wipe needs --yes (erases all user data).' 'Wipe braucht --yes (loescht Nutzerdaten).')"; exit 4; fi
    guided_wipe --yes || exit 1 ;;
  reinstall)
    if [ -z "$YES" ]; then printf '%s\n' "$(L 'Reinstall needs --yes.' 'Reinstall braucht --yes.')"; exit 4; fi
    if [ -z "$IMAGE" ] || [ ! -f "$IMAGE" ]; then printf '%s\n' "$(L 'Reinstall needs --image <gsi-or-system.img> (custom); stock path is guided in TUI.' 'Reinstall braucht --image (custom); Stock-Weg in TUI.'))"; exit 3; fi
    system_flash "$IMAGE" --yes || exit 1 ;;
  persist)
    if [ -z "$YES" ]; then printf '%s\n' "$(L 'Needs live root + --yes.' 'Braucht live Root + --yes.')"; exit 4; fi
    install_persist_fixes || exit 1 ;;
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
  preflight)
    if preflight; then printf 'PREFLIGHT READY\n'; else exit 1; fi ;;
  setup)
    find_tools
    ensure_tool adb || exit 1
    ensure_tool fastboot || exit 1
    ensure_scrcpy
    save_config
    preflight || exit 1
    printf 'SETUP COMPLETE\n' ;;
  recon)
    preflight >/dev/null 2>&1 || { printf 'PREFLIGHT BLOCKED\n'; exit 1; }
    detect_mode
    [ "$MODE" = "android" ] && android_analysis
    [ "$MODE" = "fastboot" ] && fastboot_analysis || true
    if [ -n "$JSON" ]; then printf '{"mode":"%s","os":"%s","profile":"%s","storage":"%s"}\n' "$MODE" "$OS_KIND" "$PROFILE_ID" "$STORAGE"
    else printf 'mode=%s os=%s profile=%s storage=%s\n' "$MODE" "$OS_KIND" "$PROFILE_ID" "$STORAGE"; fi ;;
  status)
    if preflight >/dev/null 2>&1; then pf=true; else pf=false; fi
    g=""; [ -f "$(state_file)" ] && g="$(read_state_goal 2>/dev/null || true)"
    load_rom
    printf '{"preflight_go":%s,"adb":"%s","fastboot":"%s","mode":"%s","installed_rom":"%s","run_mode":"%s","goal":"%s"}\n' "$pf" "$ADB_STATE" "$FB_STATE" "$MODE" "$INSTALLED_ROM" "$RUNMODE" "$g" ;;
  rom)
    load_rom
    case "${ROMARG:-}" in
      list) rom_options | awk -F'|' '{printf "%d. %s [%s]\n", NR, $2, $1}' ;;
      set) if [ -n "${ROMVAL:-}" ]; then
             case "$ROMVAL" in
               *[!0-9]*) save_rom "$ROMVAL" ;;
               *) _rid="$(rom_options | sed -n "${ROMVAL}p" | cut -d'|' -f1)"
                  if [ -n "$_rid" ]; then save_rom "$_rid"; else printf 'Unknown number.\n'; exit 3; fi ;;
             esac
             printf 'Phone runs: %s\n' "$(rom_label "$INSTALLED_ROM")"
           else printf 'rom set needs a value (number from rom list, or id).\n'; exit 3; fi ;;
      clear) save_rom ""; printf 'Installed ROM cleared.\n' ;;
      *) if [ -n "$JSON" ]; then printf '{"installed_rom":"%s","label":"%s"}\n' "$INSTALLED_ROM" "$(rom_label "$INSTALLED_ROM")"
         else printf 'Phone runs: %s [%s]\n' "$(rom_label "$INSTALLED_ROM")" "$INSTALLED_ROM"; fi ;;
    esac ;;
  workflow)
    g="${GOAL:-}"
    if [ -z "$g" ] || [ -z "$(goal_steps "$g")" ]; then printf 'Unknown goal. Known: root custom_rom stock_rom root_custom_rom root_stock_rom root_custom_rom_recovery root_stock_rom_recovery restore_original full_reinstall\n'; exit 1; fi
    if [ -n "$JSON" ]; then
      printf '{"goal":"%s","steps":[' "$g"
      first=1; for s in $(goal_steps "$g"); do
        gate="pending"
        case "$s" in *flash*) gate="flash";; *verify*|*validat*) gate="verify";; *backup*) gate="backup";; *recon*|*analy*) gate="analyze";; esac
        st="pending"
        if [ "$gate" != "pending" ] && ! step_gate "$gate" >/dev/null 2>&1; then st="blocked"; fi
        [ "$first" = 1 ] || printf ','; first=0
        printf '{"step":"%s","status":"%s"}' "$s" "$st"
      done
      printf ']}\n'
    else
      printf 'Goal: %s\n' "$g"
      for s in $(goal_steps "$g"); do printf ' - %s\n' "$s"; done
    fi ;;
  resume)
    g="$(read_state_goal)" || { printf 'No saved workflow state.\n'; exit 1; }
    [ -z "$g" ] && { printf 'No saved workflow state.\n'; exit 1; }
    if [ -z "$YES" ]; then printf "Resume goal '%s'? Re-run with --yes.\n" "$g"; exit 4; fi
    run_goal "$g" || exit 1 ;;
  root)
    if [ -z "$YES" ]; then printf 'Root goal needs --yes (gates still enforced). Plan: workflow --goal root.\n'; exit 4; fi
    run_goal root || exit 1 ;;
  wizard) main_menu ;;
esac
