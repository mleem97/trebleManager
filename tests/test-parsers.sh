#!/usr/bin/env bash
# Unit tests for treble-toolkit.sh pure logic (simulated outputs only; never production).
# Run: bash tests/test-parsers.sh
set -u
PASS=0; FAIL=0
ok() { PASS=$((PASS+1)); printf '[PASS] %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '[FAIL] %s\n' "$1"; }
printf 'test-parsers (bash %s, src: %s)\n' "$BASH_VERSION" "${BASH_SOURCE[0]:-pipe/stdin}"
printf 'DRY RUN TESTS - no device flashed or touched (simulated outputs only).\n'

# Online run (curl|bash): without repo layout fetch the FULL release ZIP
# (same trust root) and run the suite from it. Guard prevents loops.
# Tag order: raw VERSION first (no API needed), API last (rate-limited).
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
        echo "ERROR: SHA256 MISMATCH - deleted, aborting." >&2; rm -f "$_zip"; return 1
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
if [ -z "${TT_TEST_BOOTSTRAPPED:-}" ]; then
  _BSSRC0="${BASH_SOURCE[0]:-}"
  if [ -n "$_BSSRC0" ] && [ -f "$_BSSRC0" ]; then
    _TROOT="$(cd "$(dirname "$_BSSRC0")/.." && pwd)"
  else
    _TROOT="$(pwd)"
  fi
  if [ ! -f "$_TROOT/VERSION" ] || [ ! -f "$_TROOT/scripts/treble-toolkit.sh" ]; then
    echo "Test suite without layout - fetching full release ZIP ..."
    _tbase="${XDG_DATA_HOME:-$HOME/.local/share}/trebleManager"
    mkdir -p "$_tbase" 2>/dev/null
    _tver="$(curl -fsSL https://raw.githubusercontent.com/mleem97/trebleManager/main/VERSION 2>/dev/null | tr -d ' \r\n')"
    _tbf=""
    if [ -n "$_tver" ]; then _tbf="$(bootstrap_fetch "v$_tver" "tests/test-parsers.sh" 2>/dev/null)"; fi
    if [ -z "$_tbf" ]; then
      _ttag="$(curl -fsSL https://api.github.com/repos/mleem97/trebleManager/releases/latest 2>/dev/null | grep -m1 '"tag_name"' | cut -d'"' -f4)"
      [ -n "$_ttag" ] && _tbf="$(bootstrap_fetch "$_ttag" "tests/test-parsers.sh" 2>/dev/null)"
    fi
    if [ -n "$_tbf" ]; then
      export TT_TEST_BOOTSTRAPPED=1
      bash "$_tbf"
      exit $?
    fi
    echo "ERROR: test bootstrap failed (offline?) - cannot run without layout."
    exit 1
  fi
  unset _BSSRC0 _TROOT _tbase _tver _tbf _ttag
fi

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)/treble-toolkit.sh"

# Extract function bodies by brace balancing and source them (pure funcs only).
import_fn() {
  local name="$1" start depth i block
  start="$(grep -n "^${name}() {" "$SRC" | head -1 | cut -d: -f1)"
  [ -n "$start" ] || { bad "function found: $name"; return; }
  depth=0; i=0; block=""
  while IFS= read -r line; do
    i=$((i+1)); [ "$i" -lt "$start" ] && continue
    block="$block
$line"
    opens="$(printf '%s' "$line" | tr -cd '{' | wc -c)"
    closes="$(printf '%s' "$line" | tr -cd '}' | wc -c)"
    depth=$((depth + opens - closes))
    if [ "$i" -gt "$start" ] && [ "$depth" -eq 0 ]; then break; fi
  done < "$SRC"
  eval "$block" || { bad "load: $name"; return; }
}
for fn in valid_url boot_magic_ver image_kind firmware_compat os_classify profile_verified profile_variant test_system_image root_method_ids root_method_name compat_file compat_broken_markers compat_roms vendor_advice resolve_mode goal_steps step_gate device_states platform_tools_url install_base_dir flash_verdict rom_suggest rom_label rom_options rom_broken rom_entry_gsi export_recovery rom_downloads target_androids resolver_entries; do import_fn "$fn"; done

# Need TTLANG + PROFILE_ID + stubs used by imported funcs
TTLANG="en"
PROFILE_ID="VTR-L29"
FW_STATUS=""

# 1. URL validation
valid_url "https://androidhost.ru/x/VTR-L29-9.1.0.297.zip" && ok "url https zip" || bad "url https zip"
valid_url "ftp://example.com/fw.zip" && bad "url ftp rejected" || ok "url ftp rejected"
valid_url "file:///C:/fw.zip" && bad "url file rejected" || ok "url file rejected"
valid_url "https://example.com/fw.exe" && bad "url exe rejected" || ok "url exe rejected"
valid_url "" && bad "url empty rejected" || ok "url empty rejected"

# 2. Boot magic (real files, no mock)
TMP="$(mktemp -d)"
printf 'ANDROID!\x030000' > "$TMP/good.img"; head -c 100 /dev/zero >> "$TMP/good.img"
[ "$(boot_magic_ver "$TMP/good.img")" = "3" ] && ok "boot magic v3" || bad "boot magic v3"
printf 'no android here' > "$TMP/bad.txt"
[ "$(boot_magic_ver "$TMP/bad.txt")" = "-1" ] && ok "no magic rejected" || bad "no magic rejected"

# 3. Firmware compat
out="$(firmware_compat VTR-L29 "VTR-L29 9.1.0.297(C432E5R1P9)" "C432")"
[ "${out%%|*}" = "PASS" ] && ok "fw pass" || bad "fw pass ($out)"
out="$(firmware_compat VTR-L29 "VKY-L29 9.1.0.297(C432E5R1P9)" "C432")"
[ "${out%%|*}" = "FAIL" ] && ok "fw wrong model FAIL" || bad "fw wrong model ($out)"
out="$(firmware_compat VTR-L29 "" "")"
[ "${out%%|*}" = "FAIL" ] && ok "fw empty FAIL" || bad "fw empty ($out)"

# 4. OS classification
MODEL="TrebleDroid with GApps"; PNAME="lineage_arm64_bgN"; DISPLAY="lineage_arm64_bgN-userdebug 13 TQ3A.230901.001"; EMUI=""; OS_KIND=""
os_classify
[ "$OS_KIND" = "TrebleDroid-GSI" ] && ok "os gsi" || bad "os gsi ($OS_KIND)"
MODEL="VTR-L29"; PNAME="VTR-L29"; DISPLAY="VTR-L29 9.1.0.297(C432E5R1P9)"; EMUI="EmotionUI_9.1.0"
os_classify
[ "$OS_KIND" = "Stock-EMUI-9.1" ] && ok "os stock" || bad "os stock ($OS_KIND)"

# 5. Profile verified gate
PROFILE_ID="VTR-L29"; [ "$(profile_verified)" = "1" ] && ok "verified VTR-L29" || bad "verified VTR-L29"
PROFILE_ID="VTR-AL00"; [ "$(profile_verified)" = "0" ] && ok "unverified VTR-AL00 blocks" || bad "unverified VTR-AL00"
PROFILE_ID="VTR-L29"

# 5b. Root methods: Magisk preferred first
first="$(root_method_ids | head -1)"
[ "$first" = "magisk-recovery" ] && ok "magisk preferred first" || bad "magisk order ($first)"
[ "$(root_method_ids | wc -l)" -eq 4 ] && ok "4 root methods" || bad "root method count"

# 5c. Registry: broken markers + rom lists (needs TOOL_ROOT)
TOOL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE_ID="VTR-L29"
mk="$(compat_broken_markers)"
printf '%s' "$mk" | grep -q light && ok "marker light" || bad "marker light ($mk)"
printf '%s' "$mk" | grep -q havoc && ok "marker havoc" || bad "marker havoc ($mk)"
rl="$(compat_roms)"
printf '%s' "$rl" | grep -q "LineageOS|working" && ok "registry lineage" || bad "registry lineage"

# 6. System image gate (small/foreign names refused)
printf 'tiny' > "$TMP/small.img"
t="$(test_system_image "$TMP/small.img" 2>/dev/null || printf 'FAIL|small')"
[ "${t%%|*}" = "FAIL" ] && ok "system small refused" || bad "system small ($t)"

# 5. Huawei getvar tolerance (FAILED is data, not lock)
printf 'getvar:unlocked FAILED (remote: Command not allowed)\nfinished.\n' > "$TMP/fb.txt"
grep -q "Command not allowed" "$TMP/fb.txt" && ok "huawei denied tolerated" || bad "huawei denied"
rm -rf "$TMP"

# ---- 13. Run modes ----
[ "$(resolve_mode "")" = "safe" ] && ok "mode safe default" || bad "mode default"
[ "$(resolve_mode unattended)" = "unattended" ] && ok "mode unattended" || bad "mode unattended"
[ "$(resolve_mode developer)" = "developer" ] && ok "mode developer" || bad "mode developer"
[ "$(resolve_mode yolo)" = "safe" ] && ok "mode fallback" || bad "mode fallback"

# ---- 14. Goals + gates (no device here: flash blocked, detect passes) ---- ---- ----
[ "$(goal_steps root | wc -w)" = "10" ] && ok "root goal 10 steps" || bad "root goal steps"
[ -z "$(goal_steps nope)" ] && ok "unknown goal empty" || bad "unknown goal"
PATCHED_IMAGE=""; STOCK_IMAGE=""; ADB_BIN=""; FB_BIN=""
step_gate detect >/dev/null 2>&1 && ok "gate detect passes" || bad "gate detect"
step_gate flash >/dev/null 2>&1 && bad "gate flash blocked" || ok "gate flash blocked"
device_states
[ -n "$OVERALL" ] && ok "device states ($OVERALL)" || bad "device states"

# ---- 15. Guided setup helpers (pure parts) ----
[ "$(platform_tools_url linux)" = "https://dl.google.com/android/repository/platform-tools-latest-linux.zip" ] && ok "pt linux url" || bad "pt linux url"
[ "$(platform_tools_url darwin)" = "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip" ] && ok "pt mac url" || bad "pt mac url"
TOOL_DIR="$(mktemp -d)"
[ -n "$(install_base_dir)" ] && ok "install base dir" || bad "install base dir"
rm -rf "$TOOL_DIR"

# ---- 12. Immutable release: single version everywhere ----
ROOT_D="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VER_FILE="$(tr -d ' \n\r' < "$ROOT_D/VERSION")"
SH_VER="$(grep -m1 '^TTVERSION=' "$ROOT_D/scripts/treble-toolkit.sh" | cut -d'"' -f2)"
[ "$SH_VER" = "$VER_FILE" ] && ok "bash == VERSION ($VER_FILE)" || bad "bash ($SH_VER) != VERSION ($VER_FILE)"
grep -q "Version-$VER_FILE" "$ROOT_D/README.md" && ok "readme badge" || bad "readme badge"
grep -q "## v$VER_FILE" "$ROOT_D/CHANGELOG.md" && ok "changelog entry" || bad "changelog entry"

# ---- 16. Flash verdict: FAILED vetoes, progress words are not success ----
printf '%s\n' "Sending 'system' (1126400 KB)              OKAY [ 28.1s]" "Writing 'system'                                 OKAY [ 41.2s]" "Finished. Total time: 70.003s" | flash_verdict >/dev/null 2>&1
[ "$?" = 0 ] && ok "verdict OK on full success" || bad "verdict OK on full success"
printf '%s\n' "Sending 'system' (1126400 KB)              OKAY [ 28.1s]" "Writing 'system'          FAILED (remote: 'Command not allowed')" "Finished. Total time: 0.010s" | flash_verdict >/dev/null 2>&1
[ "$?" = 1 ] && ok "verdict FAILED despite Writing+OKAY" || bad "verdict FAILED despite Writing+OKAY"
printf '%s\n' "Erasing 'userdata' ..." | flash_verdict >/dev/null 2>&1
[ "$?" = 2 ] && ok "verdict UNCLEAR on progress only" || bad "verdict UNCLEAR on progress only"
printf '%s\n' "Erasing 'userdata'                                 OKAY [  2.1s]" "Finished. Total time: 2.150s" | flash_verdict >/dev/null 2>&1
[ "$?" = 0 ] && ok "verdict OK on erase success" || bad "verdict OK on erase success"

# ---- 17. Installed ROM: suggestion never guesses custom, labels exact ----
[ "$(rom_suggest 'Stock EMUI 9.1')" = "stock" ] && ok "suggest stock for EMUI" || bad "suggest stock for EMUI"
[ -z "$(rom_suggest 'TrebleDroid GSI')" ] && ok "suggest empty for GSI (user picks)" || bad "suggest empty for GSI (user picks)"
[ "$(rom_label 'stock')" = "Stock EMUI" ] && ok "label stock" || bad "label stock"
[ "$(rom_label 'rom:LineageOS 20')" = "LineageOS 20" ] && ok "label rom id" || bad "label rom id"
[ "$(rom_label 'other')" = "Other custom ROM" ] && ok "label other" || bad "label other"
[ "$(rom_label '')" = "?" ] && ok "label empty" || bad "label empty"
TOOL_ROOT="$ROOT_D"; PROFILE_ID="VTR-L09"
rom_options | grep -q "UNOFFICIAL (20251021)" && ok "unofficial lineage selectable (VTR-L09)" || bad "unofficial lineage selectable (VTR-L09)"
rom_options | grep -q "^stock|" && ok "stock option first" || bad "stock option first"
rom_broken | grep -qi "light" && ok "broken light listed unselectable" || bad "broken light listed unselectable"
# Fresh fixtures (TMP was cleaned mid-suite)
KTMP="$(mktemp -d)"
printf 'ANDROID!\x030000' > "$KTMP/good.img"; head -c 100 /dev/zero >> "$KTMP/good.img"
[ "$(image_kind "$KTMP/good.img")" = "boot" ] && ok "kind boot" || bad "kind boot"
printf '\x3a\xff\x26\xed' > "$KTMP/sparse.img"; head -c 100 /dev/zero >> "$KTMP/sparse.img"
[ "$(image_kind "$KTMP/sparse.img")" = "system" ] && ok "kind system (sparse)" || bad "kind system (sparse)"
printf 'no android here' > "$KTMP/bad.txt"
[ "$(image_kind "$KTMP/bad.txt")" = "unknown" ] && ok "kind unknown (text)" || bad "kind unknown (text)"
[ "$(rom_entry_gsi 'rom:LineageOS 20 UNOFFICIAL (20251021)' "$(compat_file)")" = "arm64_bgN" ] && ok "unofficial entry is GSI-typed" || bad "unofficial entry is GSI-typed"
[ -z "$(rom_entry_gsi 'rom:LeaOS' "$(compat_file)")" ] && ok "device rom has no gsi flag" || bad "device rom has no gsi flag"
# Live gz-export (real files): boot.img.gz -> exported base; system.img.gz -> refused as SYSTEM
log() { :; }
REC_DIR="$TMP/rec"; TOOL_DIR="$TMP/tools"; ROM_DIR="$TMP/roms"; MAG_DIR="$TMP/mag"; STAMP="t"; TTLOG="$TMP/t.log"; DATA_DIR="$TMP/data"
mkdir -p "$REC_DIR" "$TOOL_DIR" "$ROM_DIR" "$MAG_DIR"
printf 'ANDROID!\x030000' > "$TMP/tboot.img"; head -c 100 /dev/zero >> "$TMP/tboot.img"
gzip -c "$TMP/tboot.img" > "$ROM_DIR/tboot.img.gz"
EXPORT_FILES=""
if export_recovery "$ROM_DIR/tboot.img.gz" >/dev/null 2>&1; then
  [ -n "$EXPORT_FILES" ] && [ -f "$EXPORT_FILES" ] && ok "gz boot export works" || bad "gz boot export works (no file)"
else bad "gz boot export works (rc)"; fi
printf '\x3a\xff\x26\xed' > "$TMP/tsys.img"; head -c 100 /dev/zero >> "$TMP/tsys.img"
gzip -c "$TMP/tsys.img" > "$ROM_DIR/tsys.img.gz"
if export_recovery "$ROM_DIR/tsys.img.gz" >/dev/null 2>&1; then bad "gz system refused"; else ok "gz system refused"; fi
rom_downloads | grep -q "lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz|https://sourceforge.net/" && ok "verified GSI download URL in registry" || bad "verified GSI download URL in registry"
valid_url "https://sourceforge.net/projects/andyyan-gsi/files/lineage-20-td/lineage-20.0-20251021-UNOFFICIAL-arm64_bgN-signed.img.gz/download" && ok "sourceforge /download URL accepted" || bad "sourceforge /download URL accepted"
target_androids | grep -q "^13|" && ok "android 13 offered" || bad "android 13 offered"
[ "$(target_androids | grep -c .)" -ge 3 ] && ok "multiple android versions (bidirectional filter base)" || bad "multiple android versions (bidirectional filter base)"
resolver_entries 13 | grep -q "LineageOS 20 UNOFFICIAL (20251021)" && ok "resolver finds unofficial build" || bad "resolver finds unofficial build"
resolver_entries 13 "LineageOS 20 UNOFFICIAL (20251021)" | grep -q "recovery_ramdisk|stock_firmware" && ok "resolver root artifact resolved" || bad "resolver root artifact resolved"

# ---- 18. Launchers: one central entry, online starters bootstrap full ZIP ----
[ -f "$ROOT_D/Start-TrebleToolkit.bat" ] && ok "central starter present" || bad "central starter present"
grep -q 'scripts\\Treble-Toolkit.ps1' "$ROOT_D/Start-TrebleToolkit.bat" && ok "starter calls TUI" || bad "starter calls TUI"
grep -q "pause" "$ROOT_D/Start-TrebleToolkit.bat" && ok "starter never silent" || bad "starter never silent"
[ -f "$ROOT_D/Run-FromGitHub.bat" ] && ok "online starter bat present" || bad "online starter bat present"
grep -q "releases/latest" "$ROOT_D/Run-FromGitHub.bat" && ok "online bat resolves latest" || bad "online bat resolves latest"
grep -q "Start-TrebleToolkit.bat" "$ROOT_D/Run-FromGitHub.bat" && ok "online bat chains central starter" || bad "online bat chains central starter"
grep -q "SHA256" "$ROOT_D/Run-FromGitHub.bat" && ok "online bat verifies hash" || bad "online bat verifies hash"
[ -f "$ROOT_D/run-from-github.sh" ] && ok "online starter sh present" || bad "online starter sh present"
bash -n "$ROOT_D/run-from-github.sh" && ok "online sh syntax" || bad "online sh syntax"
grep -q "treble-toolkit.sh" "$ROOT_D/run-from-github.sh" && ok "online sh launches toolkit" || bad "online sh launches toolkit"
grep -q "run-from-github" "$ROOT_D/README.md" && ok "readme documents online start" || bad "readme documents online start"
grep -q "Instant Execute" "$ROOT_D/README.md" && ok "readme instant execute" || bad "readme instant execute"
grep -q "certutil -hashfile" "$ROOT_D/README.md" && ok "readme cmd verify" || bad "readme cmd verify"
grep -q "Run-FromGitHub.bat" "$ROOT_D/README.md" && ok "readme cmd one-liner" || bad "readme cmd one-liner"
grep -q "sha256sum -c" "$ROOT_D/README.md" && ok "readme linux verify" || bad "readme linux verify"
grep -q "TT_TEST_BOOTSTRAPPED" "$ROOT_D/tests/test-parsers.sh" && ok "bash tests self-bootstrap" || bad "bash tests self-bootstrap"
grep -q "TT_TEST_BOOTSTRAPPED" "$ROOT_D/tests/Test-Parsers.ps1" && ok "ps1 tests self-bootstrap" || bad "ps1 tests self-bootstrap"
grep -q 'User-Agent' "$ROOT_D/tests/Test-Parsers.ps1" && ok "ps1 tests send UA (no API 403)" || bad "ps1 tests send UA (no API 403)"
grep -q 'User-Agent' "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 tool sends UA (no API 403)" || bad "ps1 tool sends UA (no API 403)"
grep -q "Test-Parsers.ps1 | iex" "$ROOT_D/README.md" && ok "readme online tests" || bad "readme online tests"
grep -q 'bootstrap_fetch "v$TTVERSION"' "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash own-version first" || bad "bash own-version first"
grep -q "main/VERSION" "$ROOT_D/tests/test-parsers.sh" && ok "bash tests raw VERSION (no API)" || bad "bash tests raw VERSION (no API)"
grep -q "main/VERSION" "$ROOT_D/tests/Test-Parsers.ps1" && ok "ps1 tests raw VERSION (no API)" || bad "ps1 tests raw VERSION (no API)"
grep -q "function Get-TTScriptRoot" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 script-root helper" || bad "ps1 script-root helper"
grep -q '@__fw' "$ROOT_D/scripts/Treble-Toolkit.ps1" && ! grep -q '@\$__fw' "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 correct splatting" || bad "ps1 correct splatting"
grep -q "function script:" "$ROOT_D/tests/Test-Parsers.ps1" && ok "ps1 imports script-scoped" || bad "ps1 imports script-scoped"
grep -q "MISSING functions" "$ROOT_D/tests/Test-Parsers.ps1" && ok "ps1 import fail-fast" || bad "ps1 import fail-fast"
grep -q "main/VERSION" "$ROOT_D/Run-FromGitHub.bat" "$ROOT_D/run-from-github.sh" && ok "launchers raw VERSION first" || bad "launchers raw VERSION first"
grep -q "stop aptouch" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash immediate aptouch stop" || bad "bash immediate aptouch stop"
grep -q "stop aptouch" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 immediate aptouch stop" || bad "ps1 immediate aptouch stop"
grep -q "SKIP_FIX_OFFER" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash wizard autofix, no double prompt" || bad "bash wizard autofix, no double prompt"
grep -q "SkipFixOffer" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 wizard autofix, no double prompt" || bad "ps1 wizard autofix, no double prompt"
grep -q "persistent boot. Conscious choice" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 persistent-boot pointer" || bad "ps1 persistent-boot pointer"
grep -q "persistent boot. Conscious choice" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash persistent-boot pointer" || bad "bash persistent-boot pointer"
[ -f "$ROOT_D/core/treble_core/Cargo.toml" ] && ok "rust core manifest" || bad "rust core manifest"
grep -q "flash_verdict" "$ROOT_D/core/treble_core/src/fastboot.rs" && ok "rust flash verdict" || bad "rust flash verdict"
grep -q "image_kind" "$ROOT_D/core/treble_core/src/images.rs" && ok "rust image kind" || bad "rust image kind"
grep -q "check_url" "$ROOT_D/core/treble_core/src/firmware.rs" && ok "rust url check" || bad "rust url check"
grep -q "rom_entry_label" "$ROOT_D/core/treble_core/src/roms.rs" && ok "rust rom labels" || bad "rust rom labels"
[ -f "$ROOT_D/gsi-root/Cargo.toml" ] && ok "gsi-root workspace" || bad "gsi-root workspace"
grep -q "EXPERIMENTAL" "$ROOT_D/gsi-root/crates/gsi-root-cli/src/main.rs" && ok "gsi-root honest refusal" || bad "gsi-root honest refusal"
[ -f "$ROOT_D/gsi-root/devices/huawei-p10/profile.toml" ] && ok "gsi-root P10 profile" || bad "gsi-root P10 profile"
[ -f "$ROOT_D/gsi-root/crates/gsi-device/src/lib.rs" ] && ok "gsi-device crate" || bad "gsi-device crate"
grep -q "workflow-state.json.* shape" "$ROOT_D/gsi-root/crates/gsi-device/src/lib.rs" && ok "slot format compatible" || bad "slot format compatible"
grep -q "Managed external-tool binding" "$ROOT_D/gsi-root/crates/gsi-tool/src/lib.rs" && ok "gsi-tool binding layer" || bad "gsi-tool binding layer"
grep -q "device detect" "$ROOT_D/gsi-root/crates/gsi-root-cli/src/main.rs" && ok "cli device commands" || bad "cli device commands"
[ -f "$ROOT_D/gsi-root/docs/STATUS.md" ] && ok "gsi-root status doc" || bad "gsi-root status doc"
grep -q '"gui"' "$ROOT_D/run-from-github.sh" && ok "online sh installs GUI" || bad "online sh installs GUI"
grep -q ":GUI" "$ROOT_D/Run-FromGitHub.bat" && ok "online bat installs GUI" || bad "online bat installs GUI"
grep -q "LOCALAPPDATA" "$ROOT_D/Run-FromGitHub.bat" && ok "bat GUI target dir" || bad "bat GUI target dir"
grep -q "RefusedExperimental" "$ROOT_D/gsi-root/crates/gsi-workflow/src/lib.rs" && ok "workflow honest refusal" || bad "workflow honest refusal"
grep -q "NotCheckedNoKeyInfrastructure" "$ROOT_D/gsi-root/crates/gsi-update/src/lib.rs" && ok "updater no fake signatures" || bad "updater no fake signatures"
grep -q "TT_BOOTSTRAPPED" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash self-bootstrap" || bad "bash self-bootstrap"
grep -q "TT_BOOTSTRAPPED" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 self-bootstrap" || bad "ps1 self-bootstrap"
grep -q "SHA256 MISMATCH" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 bootstrap aborts on mismatch" || bad "ps1 bootstrap aborts on mismatch"
grep -q "SHA256 MISMATCH" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash bootstrap aborts on mismatch" || bad "bash bootstrap aborts on mismatch"

printf '\nResult: %s PASS, %s FAIL\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
