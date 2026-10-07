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
if [ "${1:-}" = "gui" ]; then
  # GUI installieren + starten (gsi-root, Prebuilt-Binary aus Release).
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"; ARCH="$(uname -m)"
  case "$OS-$ARCH" in
    linux-x86_64) PLAT="linux-x86_64" ;;
    *) echo "ERROR: no prebuilt GUI binary for $OS-$ARCH."; echo "Fallback: TUI via run-from-github.sh (without gui)."; read -r -p "Enter=close " _; exit 1 ;;
  esac
  GBIN="$HOME/.local/bin/gsi-root"
  if [ ! -x "$GBIN" ]; then
    GZ="gsi-root-$PLAT.tar.gz"
    echo "Downloading GUI: $REPO releases/download/$TAG/$GZ"
    curl -fsSL -o "$BASE/$GZ" "https://github.com/$REPO/releases/download/$TAG/$GZ" \
      || { echo "ERROR: GUI download failed."; read -r -p "Enter=close " _; exit 1; }
    if curl -fsSL -o "$BASE/$GZ.sha256" "https://github.com/$REPO/releases/download/$TAG/$GZ.sha256" 2>/dev/null; then
      EXP="$(awk '{print $1}' "$BASE/$GZ.sha256")"
      ACT="$(sha256sum "$BASE/$GZ" 2>/dev/null | awk '{print $1}')"
      if [ -n "$EXP" ] && [ -n "$ACT" ] && [ "$EXP" != "$ACT" ]; then
        echo "ERROR: SHA256 MISMATCH - deleted, aborting."; rm -f "$BASE/$GZ"; read -r -p "Enter=close " _; exit 1
      fi
      echo "SHA256 OK."
    else echo "WARN: no .sha256 asset, skipping verify."; fi
    mkdir -p "$HOME/.local/bin"
    tar -xzf "$BASE/$GZ" -C "$HOME/.local/bin" || { echo "ERROR: extract failed."; read -r -p "Enter=close " _; exit 1; }
    chmod +x "$GBIN"
    case ":$PATH:" in *":$HOME/.local/bin:"*) echo "PATH already set." ;; *)
      printf '\n# gsi-root (idempotent)\ncase ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) export PATH="$PATH:$HOME/.local/bin" ;; esac\n' >> "$HOME/.profile"
      echo "PATH persisted in ~/.profile (restart terminal)." ;; esac
  else echo "Already installed: $GBIN"; fi
  echo "Launching GUI: $GBIN"
  exec "$GBIN"
fi
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
