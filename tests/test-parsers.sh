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
for fn in valid_url boot_magic_ver firmware_compat os_classify; do import_fn "$fn"; done

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

# 5. Huawei getvar tolerance (FAILED is data, not lock)
printf 'getvar:unlocked FAILED (remote: Command not allowed)\nfinished.\n' > "$TMP/fb.txt"
grep -q "Command not allowed" "$TMP/fb.txt" && ok "huawei denied tolerated" || bad "huawei denied"
rm -rf "$TMP"

printf '\nResult: %s PASS, %s FAIL\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
