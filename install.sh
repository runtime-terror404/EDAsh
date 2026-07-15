#!/usr/bin/env bash
set -euo pipefail

# ── edash install.sh ──
# Bootstrap script: curl | sh first-install; delegates to `edash update` on re-run.
# https://github.com/runtime-terror404/edash

REPO="runtime-terror404/edash"
BIN_NAME="edash"
DEFAULT_BIN_DIR="${HOME}/.local/bin"

# ── helpers ──
need() { command -v "$1" >/dev/null 2>&1 || { echo "missing required: $1 — install it first"; exit 1; }; }
log()  { echo "  edash: $*"; }
err()  { echo "  edash error: $*" >&2; exit 1; }

# ── platform detection ──
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  ASSET_ARCH="x86_64-unknown-linux-gnu" ;;
    aarch64) ASSET_ARCH="aarch64-unknown-linux-gnu" ;;
    *)       err "unsupported architecture: $ARCH (expected x86_64 or aarch64)" ;;
esac

OS=$(uname -s)
if [ "$OS" != "Linux" ]; then
    err "unsupported OS: $OS (Linux only)"
fi

# ── args ──
SYSTEM=0
while [ $# -gt 0 ]; do
    case "$1" in
        --system) SYSTEM=1 ;;
        --help|-h)
            echo "Usage: curl ... | bash [-s -- --system]"
            echo "  --system  install to /usr/local (requires root)"
            exit 0
            ;;
        *) err "unknown flag: $1" ;;
    esac
    shift
done

# ── root guard ──
if [ "$(id -u)" = "0" ] && [ "$SYSTEM" != "1" ]; then
    err "refusing to run as root. Use --system for lab/CI installs, or run as a normal user."
fi

# ── dependency check ──
need curl
need tar
need mktemp

# ── if edash exists, delegate to update ──
if command -v edash >/dev/null 2>&1; then
    log "edash found on PATH — delegating to 'edash update'"
    exec edash update
fi

# ── set install paths ──
if [ "$SYSTEM" = "1" ]; then
    BIN_DIR="/usr/local/bin"
    INSTALL_METHOD="system"
else
    BIN_DIR="$DEFAULT_BIN_DIR"
    INSTALL_METHOD="user"
    mkdir -p "$BIN_DIR"
fi

# ── get latest release info ──
log "fetching latest release info..."
RELEASE_JSON=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest")
TAG=$(echo "$RELEASE_JSON" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "\(.*\)".*/\1/')

if [ -z "$TAG" ] || [ "$TAG" = "null" ]; then
    err "could not determine latest release tag"
fi

log "latest release: $TAG"

# ── download ──
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

BIN_URL="https://github.com/${REPO}/releases/download/${TAG}/edash-${ASSET_ARCH}"
CATALOG_URL="https://github.com/${REPO}/releases/download/${TAG}/catalog.tar.gz"

log "downloading binary..."
curl -sSL -o "$TMPDIR/edash" "$BIN_URL" || err "binary download failed"

log "downloading catalog..."
curl -sSL -o "$TMPDIR/catalog.tar.gz" "$CATALOG_URL" || err "catalog download failed"

# ── sanity-check binary ──
chmod +x "$TMPDIR/edash"
if ! "$TMPDIR/edash" --version >/dev/null 2>&1; then
    err "downloaded binary failed --version sanity check"
fi
log "binary version: $("$TMPDIR/edash" --version)"

# ── install catalog ──
log "extracting catalog..."
mkdir -p "$TMPDIR/catalog"
tar -xzf "$TMPDIR/catalog.tar.gz" -C "$TMPDIR/catalog" || err "catalog extraction failed"

# Clean up stale staging dir from previous interrupted installs
rm -rf "${HOME}/.local/share/edash/catalog/base.new"

if [ -d "${HOME}/.local/share/edash/catalog/base" ]; then
    # Update: stage catalog via hidden subcommand, then swap atomically
    log "updating catalog..."
    EDASH_INSTALLER=1 "$TMPDIR/edash" __internal stage-catalog \
        "$TMPDIR/catalog" "$TMPDIR/catalog/manifest.yaml" 2>/dev/null || true
    if [ -d "${HOME}/.local/share/edash/catalog/base.new" ]; then
        rm -rf "${HOME}/.local/share/edash/catalog/base.old" 2>/dev/null || true
        mv "${HOME}/.local/share/edash/catalog/base" "${HOME}/.local/share/edash/catalog/base.old" 2>/dev/null || true
        mv "${HOME}/.local/share/edash/catalog/base.new" "${HOME}/.local/share/edash/catalog/base" 2>/dev/null || true
        rm -rf "${HOME}/.local/share/edash/catalog/base.old" 2>/dev/null || true
    fi
else
    # First install: copy directly, no staging needed
    log "first install — setting up catalog..."
    mkdir -p "${HOME}/.local/share/edash/catalog/base"
    cp -a "$TMPDIR/catalog/." "${HOME}/.local/share/edash/catalog/base/"
fi

# ── install binary ──
log "installing to $BIN_DIR/$BIN_NAME"
mv "$TMPDIR/edash" "$BIN_DIR/$BIN_NAME"

# ── self-test ──
if ! "$BIN_DIR/$BIN_NAME" --version >/dev/null 2>&1; then
    err "installed binary failed self-test"
fi

# ── write installation.yaml ──
mkdir -p "${HOME}/.local/share/edash"
cat > "${HOME}/.local/share/edash/installation.yaml" <<YAML
install_method: $INSTALL_METHOD
installed_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
binary_version: $TAG
catalog_version: $TAG
installer_version: 3
YAML

# ── done ──
echo ""
echo "  ✓ edash $TAG installed to $BIN_DIR/$BIN_NAME"
echo ""

# ── PATH check ──
case ":$PATH:" in
    *:"$BIN_DIR":*) ;;
    *)
        echo "  Note: $BIN_DIR is not in your PATH."
        echo "  Add this to your shell config:"
        echo ""
        echo "    export PATH=\"$BIN_DIR:\$PATH\""
        echo ""
        ;;
esac

echo "  Run 'edash' to launch the dashboard, or 'edash --help' for commands."
