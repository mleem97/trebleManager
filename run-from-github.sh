#!/usr/bin/env bash
# Central online entry point (Linux/macOS): downloads the FULL release ZIP
# (scripts/ + data/registry + tests) and launches the toolkit from it, so an
# online run is 100% identical to a ZIP run. Never exits silently.
set -u
REPO="mleem97/trebleManager"
API="https://api.github.com/repos/$REPO/releases/latest"
BASE="${XDG_DATA_HOME:-$HOME/.local/share}/trebleManager"
mkdir -p "$BASE" || { echo "ERROR: cannot create $BASE"; read -r -p "Enter=close " _; exit 1; }

echo "Resolving latest release (raw VERSION first, API fallback) ..."
TAG="$(curl -fsSL https://raw.githubusercontent.com/mleem97/trebleManager/main/VERSION 2>/dev/null | tr -d ' \r\n')"
[ -n "$TAG" ] && TAG="v$TAG"
if [ -z "$TAG" ]; then
  TAG="$(curl -fsSL "$API" 2>/dev/null | grep -m1 '"tag_name"' | cut -d'"' -f4)"
fi
if [ -z "$TAG" ]; then
  echo "ERROR: GitHub API unreachable (network/proxy?)."
  echo "Fallback: download a release ZIP manually, extract, run ./scripts/treble-toolkit.sh"
  read -r -p "Enter=close " _
  exit 1
fi
echo "Version: $TAG"
DEST="$BASE/$TAG"
if [ ! -x "$DEST/scripts/treble-toolkit.sh" ]; then
  ZIP="$BASE/trebleManager-$TAG.zip"
  echo "Downloading: $REPO releases/download/$TAG/$(basename "$ZIP")"
  curl -fsSL -o "$ZIP" "https://github.com/$REPO/releases/download/$TAG/$(basename "$ZIP")" \
    || { echo "ERROR: download failed."; read -r -p "Enter=close " _; exit 1; }
  if curl -fsSL -o "$ZIP.sha256" "https://github.com/$REPO/releases/download/$TAG/$(basename "$ZIP").sha256" 2>/dev/null; then
    EXP="$(awk '{print $1}' "$ZIP.sha256")"
    ACT="$(sha256sum "$ZIP" 2>/dev/null | awk '{print $1}')"
    if [ -z "$ACT" ]; then ACT="$(shasum -a 256 "$ZIP" 2>/dev/null | awk '{print $1}')"; fi
    if [ -n "$EXP" ] && [ -n "$ACT" ]; then
      if [ "$EXP" = "$ACT" ]; then echo "SHA256 OK."
      else echo "ERROR: SHA256 MISMATCH - deleted, aborting."; rm -f "$ZIP"; read -r -p "Enter=close " _; exit 1; fi
    else echo "WARN: could not verify SHA256 (no tool?), continuing."; fi
  else echo "WARN: no .sha256 asset, skipping verify."; fi
  mkdir -p "$DEST" || exit 1
  if command -v unzip >/dev/null 2>&1; then
    unzip -qo "$ZIP" -d "$DEST" || { echo "ERROR: extract failed."; read -r -p "Enter=close " _; exit 1; }
  else
    python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "$ZIP" "$DEST" \
      || { echo "ERROR: extract failed (need unzip or python3)."; read -r -p "Enter=close " _; exit 1; }
  fi
fi
if [ ! -x "$DEST/scripts/treble-toolkit.sh" ]; then
  echo "ERROR: toolkit script missing in: $DEST/scripts/"
  read -r -p "Enter=close " _
  exit 1
fi
echo "Launching: $DEST/scripts/treble-toolkit.sh"
chmod +x "$DEST/scripts/treble-toolkit.sh"
exec "$DEST/scripts/treble-toolkit.sh" "$@"
