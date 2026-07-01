#!/bin/sh
# Installs the headless claudeometer-service binary for Linux or macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/Antoinenz/Claudeometer/main/scripts/install.sh | sh
#
# Then:
#   claudeometer-service login <your-session-key>
#   claudeometer-service install
#
# Or, for a true one-liner on a box you already trust with the key:
#   CLAUDEOMETER_SESSION_KEY=sk-... curl -fsSL .../install.sh | sh
# which signs in and installs the background service automatically.
#
# Windows: no installer here — download claudeometer-service-windows-x86_64.exe
# from the Releases page and run it directly; see docs/SERVICE.md.

set -eu

REPO="Antoinenz/Claudeometer"
INSTALL_DIR="${CLAUDEOMETER_INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="claudeometer-service"

fail() {
    echo "error: $1" >&2
    exit 1
}

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Linux) os_tag="linux" ;;
    Darwin) os_tag="macos" ;;
    *) fail "unsupported OS '$os' — see docs/SERVICE.md for manual install" ;;
esac

case "$arch" in
    x86_64|amd64) arch_tag="x86_64" ;;
    aarch64|arm64) arch_tag="aarch64" ;;
    *) fail "unsupported architecture '$arch'" ;;
esac

asset="${BIN_NAME}-${os_tag}-${arch_tag}"

# The service is released independently from the GUI app (tagged
# `service-v*`, not `v*`), so "the latest release" of the repo as a whole
# isn't necessarily the one with this asset — e.g. a GUI-only `v*` release
# could be published more recently. Search the releases list (newest first)
# for the first one that actually has our asset, instead of assuming
# /releases/latest does.
echo "Looking up the most recent claudeometer-service release of ${REPO}..."
download_url="$(
    curl -fsSL "https://api.github.com/repos/${REPO}/releases" \
        | grep "\"browser_download_url\".*${asset}\"" \
        | head -n1 \
        | sed -E 's/.*"(https[^"]+)".*/\1/'
)"

[ -n "$download_url" ] || fail "couldn't find a release asset named '${asset}' — see docs/SERVICE.md"

mkdir -p "$INSTALL_DIR"
echo "Downloading ${asset}..."
curl -fsSL -o "${INSTALL_DIR}/${BIN_NAME}" "$download_url"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo "Installed to ${INSTALL_DIR}/${BIN_NAME}"

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "Note: ${INSTALL_DIR} isn't on your PATH — add it, or call the binary by its full path." ;;
esac

if [ -n "${CLAUDEOMETER_SESSION_KEY:-}" ]; then
    echo "CLAUDEOMETER_SESSION_KEY is set — signing in and installing the background service..."
    "${INSTALL_DIR}/${BIN_NAME}" login "$CLAUDEOMETER_SESSION_KEY"
    "${INSTALL_DIR}/${BIN_NAME}" install
else
    echo
    echo "Next steps:"
    echo "  ${INSTALL_DIR}/${BIN_NAME} login <your-session-key>   # from claude.ai's sessionKey cookie"
    echo "  ${INSTALL_DIR}/${BIN_NAME} install                    # run it in the background from now on"
fi
