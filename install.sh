#!/bin/sh
# ┌─────────────────────────────────────────────┐
# │  Beacon — CI/CD deploy monitor installer    │
# │  https://github.com/Blysspeak/beacon        │
# └─────────────────────────────────────────────┘
#
# Usage:
#   git clone https://github.com/Blysspeak/beacon && cd beacon && bash install.sh
#
# Or one-liner (downloads pre-built binary):
#   curl -fsSL https://raw.githubusercontent.com/Blysspeak/beacon/main/install.sh | sh
#
set -e

# --- Colors & output ---

setup_colors() {
    if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
        BOLD=$(tput bold 2>/dev/null || echo '')
        GREEN=$(tput setaf 2 2>/dev/null || echo '')
        YELLOW=$(tput setaf 3 2>/dev/null || echo '')
        RED=$(tput setaf 1 2>/dev/null || echo '')
        CYAN=$(tput setaf 6 2>/dev/null || echo '')
        DIM=$(tput setaf 8 2>/dev/null || echo '')
        RESET=$(tput sgr0 2>/dev/null || echo '')
    else
        BOLD='' GREEN='' YELLOW='' RED='' CYAN='' DIM='' RESET=''
    fi
}

header()    { echo ""; echo "  ${BOLD}${CYAN}$*${RESET}"; echo ""; }
info()      { echo "  ${CYAN}>${RESET} $*"; }
success()   { echo "  ${GREEN}✓${RESET} $*"; }
warn()      { echo "  ${YELLOW}!${RESET} $*"; }
error()     { echo "  ${RED}✗${RESET} $*" >&2; }
die()       { error "$@"; exit 1; }
dim()       { echo "  ${DIM}$*${RESET}"; }

ask_yn() {
    PROMPT="$1"
    DEFAULT="${2:-y}"

    if [ "$DEFAULT" = "y" ]; then
        HINT="[Y/n]"
    else
        HINT="[y/N]"
    fi

    printf "  ${BOLD}?${RESET} %s %s " "$PROMPT" "$HINT"
    read -r REPLY < /dev/tty

    case "${REPLY:-$DEFAULT}" in
        [yY]*) return 0 ;;
        *)     return 1 ;;
    esac
}

ask_input() {
    PROMPT="$1"
    printf "  ${BOLD}?${RESET} %s: " "$PROMPT"
    read -r REPLY < /dev/tty
    echo "$REPLY"
}

# --- Config ---

REPO="Blysspeak/beacon"
BINARY="beacon"
VERSION="${BEACON_VERSION:-latest}"

# --- Platform detection ---

detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$OS" in
        linux*)  OS="unknown-linux-gnu" ;;
        darwin*) OS="apple-darwin" ;;
        *)       die "Unsupported OS: $(uname -s). Only Linux and macOS are supported." ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)   ARCH="aarch64" ;;
        armv7*)          ARCH="armv7" ;;
        *)               die "Unsupported architecture: $ARCH" ;;
    esac

    # musl detection
    if [ "$OS" = "unknown-linux-gnu" ]; then
        if command -v ldd >/dev/null 2>&1; then
            case "$(ldd --version 2>&1)" in
                *musl*) OS="unknown-linux-musl" ;;
            esac
        fi
    fi

    TARGET="${ARCH}-${OS}"
}

# --- Download tool ---

download() {
    URL="$1"
    OUTPUT="$2"

    if command -v curl >/dev/null 2>&1; then
        if [ "$OUTPUT" = "-" ]; then
            curl -fsSL "$URL"
        else
            curl -fsSL "$URL" -o "$OUTPUT"
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ "$OUTPUT" = "-" ]; then
            wget -qO- "$URL"
        else
            wget -q "$URL" -O "$OUTPUT"
        fi
    else
        die "Neither curl nor wget found. Install one and retry."
    fi
}

# --- Install location ---

detect_install_dir() {
    if [ -n "$BEACON_INSTALL_DIR" ]; then
        INSTALL_DIR="$BEACON_INSTALL_DIR"
    elif [ -d "$HOME/.cargo/bin" ]; then
        INSTALL_DIR="$HOME/.cargo/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi
}

try_exec() {
    if [ -w "$INSTALL_DIR" ] || [ ! -d "$INSTALL_DIR" ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    elif command -v doas >/dev/null 2>&1; then
        doas "$@"
    else
        die "Cannot write to ${INSTALL_DIR}. Set BEACON_INSTALL_DIR to a writable path."
    fi
}

# --- Build from source ---

build_from_source() {
    if ! command -v cargo >/dev/null 2>&1; then
        die "Rust toolchain not found. Install it: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi

    info "Building from source (release mode)..."
    cargo build --release 2>&1 | while read -r line; do
        printf "\r  ${DIM}%s${RESET}                          " "$line"
    done
    echo ""

    if [ ! -f "target/release/$BINARY" ]; then
        die "Build failed — target/release/$BINARY not found"
    fi

    success "Build complete"
}

# --- Install binary ---

install_binary() {
    mkdir -p "$INSTALL_DIR"

    if [ -f "Cargo.toml" ] && [ -f "target/release/$BINARY" ]; then
        # Local build
        BIN_SOURCE="target/release/$BINARY"
    elif [ -f "Cargo.toml" ]; then
        # In repo but not built yet — build first
        build_from_source
        BIN_SOURCE="target/release/$BINARY"
    else
        # Download pre-built binary
        info "Downloading pre-built binary..."

        if [ "$VERSION" = "latest" ]; then
            VERSION=$(download "https://api.github.com/repos/${REPO}/releases/latest" - 2>/dev/null \
                | grep '"tag_name"' | head -1 \
                | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
            [ -z "$VERSION" ] && die "Failed to fetch latest version"
        fi

        ARCHIVE="${BINARY}-${VERSION}-${TARGET}.tar.gz"
        URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

        TMP_DIR=$(mktemp -d)
        trap 'rm -rf "$TMP_DIR"' EXIT

        download "$URL" "$TMP_DIR/$ARCHIVE" \
            || die "Download failed. Version ${VERSION} may not have a release for ${TARGET}"

        tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"
        BIN_SOURCE=$(find "$TMP_DIR" -name "$BINARY" -type f | head -1)
        [ -z "$BIN_SOURCE" ] && die "Binary not found in archive"
    fi

    chmod +x "$BIN_SOURCE"
    try_exec /usr/bin/cp "$BIN_SOURCE" "$INSTALL_DIR/$BINARY"

    if "$INSTALL_DIR/$BINARY" --version >/dev/null 2>&1; then
        INSTALLED_VERSION=$("$INSTALL_DIR/$BINARY" --version 2>/dev/null | head -1)
        success "Installed ${BOLD}${INSTALLED_VERSION}${RESET} to ${INSTALL_DIR}/"
    else
        success "Installed to ${INSTALL_DIR}/${BINARY}"
    fi
}

# --- Claude Code hooks ---

HOOK_SCRIPT_CONTENT='#!/bin/sh
# Beacon deploy monitor hook for Claude Code
HOOK_INPUT=$(cat)
TOOL_INPUT=$(echo "$HOOK_INPUT" | jq -r '"'"'.tool_input.command // empty'"'"' 2>/dev/null)
command -v beacon >/dev/null 2>&1 || exit 0

STATUS_JSON=$(beacon status --json 2>/dev/null)
if [ -n "$STATUS_JSON" ] && [ "$STATUS_JSON" != "null" ]; then
    STATUS=$(echo "$STATUS_JSON" | jq -r '"'"'.status // empty'"'"' 2>/dev/null)
    REPO_NAME=$(echo "$STATUS_JSON" | jq -r '"'"'.repo // empty'"'"' 2>/dev/null)
    BRANCH=$(echo "$STATUS_JSON" | jq -r '"'"'.branch // empty'"'"' 2>/dev/null)
    case "$STATUS" in
        failed) echo "DEPLOY FAILED: $REPO_NAME ($BRANCH). Run beacon status for details." ;;
    esac
fi

case "$TOOL_INPUT" in
    git\ push*) beacon watch --daemon 2>/dev/null && echo "Beacon: deploy monitoring started" || true ;;
esac
exit 0'

setup_claude_hooks() {
    CLAUDE_DIR="$HOME/.claude"

    if [ ! -d "$CLAUDE_DIR" ]; then
        warn "Claude Code not detected (~/.claude not found)"
        dim "Install Claude Code first, then run: beacon install"
        return 1
    fi

    if ! command -v jq >/dev/null 2>&1; then
        warn "jq is required for hook setup"
        dim "Install jq, then run: beacon install"
        return 1
    fi

    # Write hook script
    HOOK_DIR="$CLAUDE_DIR/hooks"
    HOOK_PATH="$HOOK_DIR/beacon-deploy-check.sh"
    mkdir -p "$HOOK_DIR"
    echo "$HOOK_SCRIPT_CONTENT" > "$HOOK_PATH"
    chmod +x "$HOOK_PATH"
    success "Hook script created"

    # Update settings.json
    SETTINGS="$CLAUDE_DIR/settings.json"

    if [ -f "$SETTINGS" ] && grep -q "beacon-deploy-check" "$SETTINGS" 2>/dev/null; then
        success "Hook already in settings.json"
        return 0
    fi

    HOOK_ENTRY="{\"matcher\":\"Bash\",\"hooks\":[{\"type\":\"command\",\"command\":\"$HOOK_PATH\",\"timeout\":10}]}"

    if [ -f "$SETTINGS" ]; then
        TMP=$(mktemp)
        jq --argjson hook "$HOOK_ENTRY" '
            .hooks //= {} |
            .hooks.PostToolUse //= [] |
            .hooks.PostToolUse += [$hook]
        ' "$SETTINGS" > "$TMP" && mv "$TMP" "$SETTINGS"
    else
        cat > "$SETTINGS" << EOF
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "$HOOK_PATH",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
EOF
    fi

    success "Hook added to settings.json"
    return 0
}

# --- Telegram setup ---

setup_telegram() {
    echo ""
    dim "Open @BeaconCIBot in Telegram and press /start to get your token."
    echo ""

    TOKEN=$(ask_input "Paste your token from the bot")

    if [ -z "$TOKEN" ]; then
        warn "Skipped — no token provided"
        dim "Connect later: beacon remote connect <TOKEN>"
        return
    fi

    "$INSTALL_DIR/$BINARY" remote connect "$TOKEN" 2>/dev/null \
        && success "Telegram connected" \
        || warn "Failed to connect. Run manually: beacon remote connect $TOKEN"

    if ask_yn "Send a test notification?" "y"; then
        "$INSTALL_DIR/$BINARY" remote test 2>/dev/null \
            && success "Test message sent — check your Telegram" \
            || warn "Test failed. Check your token and bot status."
    fi
}

# --- PATH check ---

check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) return ;;
    esac

    warn "${INSTALL_DIR} is not in your PATH"
    echo ""

    SHELL_NAME=$(basename "${SHELL:-/bin/sh}")
    case "$SHELL_NAME" in
        zsh)
            RC_FILE="~/.zshrc"
            dim "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc && source ~/.zshrc"
            ;;
        fish)
            RC_FILE="~/.config/fish/config.fish"
            dim "  fish_add_path ${INSTALL_DIR}"
            ;;
        *)
            RC_FILE="~/.bashrc"
            dim "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc && source ~/.bashrc"
            ;;
    esac

    echo ""

    if ask_yn "Add to PATH automatically? (${RC_FILE})" "y"; then
        REAL_RC=$(eval echo "$RC_FILE")
        echo "" >> "$REAL_RC"
        echo "# Beacon CLI" >> "$REAL_RC"
        echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "$REAL_RC"
        success "Added to ${RC_FILE}"
        export PATH="${INSTALL_DIR}:$PATH"
    fi
}

# --- Main ---

main() {
    setup_colors

    echo ""
    echo "  ${BOLD}┌──────────────────────────────────────┐${RESET}"
    echo "  ${BOLD}│${RESET}  ${CYAN}${BOLD}Beacon${RESET} — CI/CD deploy monitor      ${BOLD}│${RESET}"
    echo "  ${BOLD}│${RESET}  Smart radar for your deployments    ${BOLD}│${RESET}"
    echo "  ${BOLD}└──────────────────────────────────────┘${RESET}"

    # Step 1: Platform & install dir
    header "Step 1/4 — Install binary"

    detect_platform
    detect_install_dir
    info "Platform: ${BOLD}${TARGET}${RESET}"
    info "Install to: ${BOLD}${INSTALL_DIR}${RESET}"
    echo ""

    install_binary

    # Step 2: PATH
    check_path

    # Step 3: Claude Code
    header "Step 2/4 — Claude Code integration"

    if ask_yn "Set up Claude Code hooks? (auto-monitor deploys after git push)" "y"; then
        if setup_claude_hooks; then
            dim "After git push, Claude will auto-start deploy monitoring"
            dim "If deploy fails, Claude gets notified before the next action"
        fi
    else
        dim "Skip. Run later: beacon install"
    fi

    # Step 4: Telegram
    header "Step 3/4 — Telegram notifications"

    if ask_yn "Connect Telegram notifications?" "y"; then
        setup_telegram
    else
        dim "Skip. Connect later: beacon remote connect <TOKEN>"
    fi

    # Done
    header "Step 4/4 — Done!"

    echo "  ${GREEN}${BOLD}Beacon is ready!${RESET}"
    echo ""
    echo "  ${BOLD}Quick start:${RESET}"
    echo ""
    dim "  \$ beacon push              # git push + monitor deploy"
    dim "  \$ beacon watch             # monitor current deploy"
    dim "  \$ beacon status            # last deploy result"
    dim "  \$ beacon remote connect    # Telegram alerts"
    dim "  \$ beacon install           # re-setup Claude Code hooks"
    echo ""
    dim "  Docs: https://github.com/${REPO}"
    echo ""
}

main "$@"
