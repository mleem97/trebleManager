#!/usr/bin/env bash
# Unit tests for treble-toolkit.sh pure logic (simulated outputs only; never production).
# Run: bash tests/test-parsers.sh
set -u
PASS=0; FAIL=0
ok() { PASS=$((PASS+1)); printf '[PASS] %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '[FAIL] %s\n' "$1"; }

# Online run (curl|bash): without repo layout fetch the FULL release ZIP
# (same trust root) and run the suite from it. Guard prevents loops.
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
    _ttag="$(curl -fsSL https://api.github.com/repos/mleem97/trebleManager/releases/latest 2>/dev/null | grep -m1 '"tag_name"' | cut -d'"' -f4)"
    _tdone=""
    if [ -n "$_ttag" ]; then
      _tdest="$_tbase/$_ttag"
      if [ ! -f "$_tdest/tests/test-parsers.sh" ]; then
        _tzip="$_tbase/trebleManager-$_ttag.zip"
        if curl -fsSL -o "$_tzip" "https://github.com/mleem97/trebleManager/releases/download/$_ttag/trebleManager-$_ttag.zip" 2>/dev/null; then
          if curl -fsSL -o "$_tzip.sha256" "https://github.com/mleem97/trebleManager/releases/download/$_ttag/trebleManager-$_ttag.zip.sha256" 2>/dev/null; then
            _texp="$(awk '{print $1}' "$_tzip.sha256" 2>/dev/null | tr 'a-z' 'A-Z')"
            _tact="$(sha256sum "$_tzip" 2>/dev/null | awk '{print $1}' | tr 'a-z' 'A-Z')"
            if [ -z "$_tact" ]; then _tact="$(shasum -a 256 "$_tzip" 2>/dev/null | awk '{print $1}' | tr 'a-z' 'A-Z')"; fi
            if [ -n "$_texp" ] && [ "$_texp" != "$_tact" ]; then echo "ERROR: SHA256 MISMATCH - deleted, aborting."; rm -f "$_tzip"; _texp="MISMATCH"; fi
          fi
          if [ "${_texp:-}" != "MISMATCH" ]; then
            mkdir -p "$_tdest" 2>/dev/null
            if command -v unzip >/dev/null 2>&1; then unzip -qo "$_tzip" -d "$_tdest" 2>/dev/null
            else python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "$_tzip" "$_tdest" 2>/dev/null; fi
          fi
        fi
      fi
      if [ -f "$_tdest/tests/test-parsers.sh" ]; then
        export TT_TEST_BOOTSTRAPPED=1
        bash "$_tdest/tests/test-parsers.sh"
        exit $?
      fi
    fi
    echo "ERROR: test bootstrap failed (offline?) - cannot run without layout."
    exit 1
  fi
  unset _BSSRC0 _TROOT _tbase _ttag _tdest _tzip _texp _tact _tdone
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
for fn in valid_url boot_magic_ver firmware_compat os_classify profile_verified profile_variant test_system_image root_method_ids root_method_name compat_file compat_broken_markers compat_roms vendor_advice resolve_mode goal_steps step_gate device_states platform_tools_url install_base_dir flash_verdict; do import_fn "$fn"; done

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

# ---- 13. Flash verdict: FAILED vetoes, progress words are not success ----
printf '%s\n' "Sending 'system' (1126400 KB)              OKAY [ 28.1s]" "Writing 'system'                                 OKAY [ 41.2s]" "Finished. Total time: 70.003s" | flash_verdict >/dev/null 2>&1
[ "$?" = 0 ] && ok "verdict OK on full success" || bad "verdict OK on full success"
printf '%s\n' "Sending 'system' (1126400 KB)              OKAY [ 28.1s]" "Writing 'system'          FAILED (remote: 'Command not allowed')" "Finished. Total time: 0.010s" | flash_verdict >/dev/null 2>&1
[ "$?" = 1 ] && ok "verdict FAILED despite Writing+OKAY" || bad "verdict FAILED despite Writing+OKAY"
printf '%s\n' "Erasing 'userdata' ..." | flash_verdict >/dev/null 2>&1
[ "$?" = 2 ] && ok "verdict UNCLEAR on progress only" || bad "verdict UNCLEAR on progress only"
printf '%s\n' "Erasing 'userdata'                                 OKAY [  2.1s]" "Finished. Total time: 2.150s" | flash_verdict >/dev/null 2>&1
[ "$?" = 0 ] && ok "verdict OK on erase success" || bad "verdict OK on erase success"

# ---- 14. Launchers: one central entry, online starters bootstrap full ZIP ----[ -f "$ROOT_D/Start-TrebleToolkit.bat" ] && ok "central starter present" || bad "central starter present"
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
grep -q "TT_BOOTSTRAPPED" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash self-bootstrap" || bad "bash self-bootstrap"
grep -q "TT_BOOTSTRAPPED" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 self-bootstrap" || bad "ps1 self-bootstrap"
grep -q "SHA256 MISMATCH" "$ROOT_D/scripts/Treble-Toolkit.ps1" && ok "ps1 bootstrap aborts on mismatch" || bad "ps1 bootstrap aborts on mismatch"
grep -q "SHA256 MISMATCH" "$ROOT_D/scripts/treble-toolkit.sh" && ok "bash bootstrap aborts on mismatch" || bad "bash bootstrap aborts on mismatch"

printf '\nResult: %s PASS, %s FAIL\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
