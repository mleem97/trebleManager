#!/usr/bin/env bash
# Unit tests for treble-toolkit.sh pure logic (simulated outputs only; never production).
# Run: bash tests/test-parsers.sh
set -u
PASS=0; FAIL=0
ok() { PASS=$((PASS+1)); printf '[PASS] %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '[FAIL] %s\n' "$1"; }

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
for fn in valid_url boot_magic_ver firmware_compat os_classify profile_verified profile_variant test_system_image root_method_ids root_method_name compat_file compat_broken_markers compat_roms vendor_advice resolve_mode; do import_fn "$fn"; done

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

# ---- 12. Immutable release: single version everywhere ----
ROOT_D="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VER_FILE="$(tr -d ' \n\r' < "$ROOT_D/VERSION")"
SH_VER="$(grep -m1 '^TTVERSION=' "$ROOT_D/scripts/treble-toolkit.sh" | cut -d'"' -f2)"
[ "$SH_VER" = "$VER_FILE" ] && ok "bash == VERSION ($VER_FILE)" || bad "bash ($SH_VER) != VERSION ($VER_FILE)"
grep -q "Version-$VER_FILE" "$ROOT_D/README.md" && ok "readme badge" || bad "readme badge"
grep -q "## v$VER_FILE" "$ROOT_D/CHANGELOG.md" && ok "changelog entry" || bad "changelog entry"

printf '\nResult: %s PASS, %s FAIL\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
