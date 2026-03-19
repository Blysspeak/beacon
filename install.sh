#!/bin/sh
# Beacon — CI/CD deploy monitor installer
# https://github.com/Blysspeak/beacon
set -e

# --- Colors ---

setup_colors() {
    if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
        BOLD=$(tput bold 2>/dev/null || echo '')
        GREEN=$(tput setaf 2 2>/dev/null || echo '')
        LGREEN=$(tput setaf 10 2>/dev/null || echo '')
        YELLOW=$(tput setaf 3 2>/dev/null || echo '')
        RED=$(tput setaf 1 2>/dev/null || echo '')
        CYAN=$(tput setaf 6 2>/dev/null || echo '')
        DIM=$(tput setaf 8 2>/dev/null || echo '')
        WHITE=$(tput setaf 15 2>/dev/null || echo '')
        RESET=$(tput sgr0 2>/dev/null || echo '')
        BG_GREEN=$(tput setab 2 2>/dev/null || echo '')
        BG_RED=$(tput setab 1 2>/dev/null || echo '')
        BG_YELLOW=$(tput setab 3 2>/dev/null || echo '')
    else
        BOLD='' GREEN='' LGREEN='' YELLOW='' RED='' CYAN='' DIM='' WHITE='' RESET=''
        BG_GREEN='' BG_RED='' BG_YELLOW=''
    fi
}

# --- Output helpers ---

info()      { echo "  ${GREEN}>${RESET} $*"; }
success()   { echo "  ${LGREEN}${BOLD}✓${RESET} $*"; }
warn()      { echo "  ${YELLOW}${BOLD}!${RESET} $*"; }
error()     { echo "  ${RED}${BOLD}✗${RESET} $*" >&2; }
die()       { error "$@"; exit 1; }
dim()       { echo "  ${DIM}$*${RESET}"; }

step() {
    STEP_NUM="$1"
    STEP_TITLE="$2"
    echo ""
    echo "  ${DIM}─────────────────────────────────────────${RESET}"
    echo "  ${GREEN}${BOLD}[$STEP_NUM]${RESET} ${BOLD}$STEP_TITLE${RESET}"
    echo ""
}

ask_yn() {
    PROMPT="$1"
    DEFAULT="${2:-y}"
    if [ "$DEFAULT" = "y" ]; then HINT="${GREEN}Y${RESET}/n"; else HINT="y/${GREEN}N${RESET}"; fi
    printf "  ${GREEN}?${RESET} %s [%b] " "$PROMPT" "$HINT"
    read -r REPLY < /dev/tty
    case "${REPLY:-$DEFAULT}" in
        [yY]*) return 0 ;;
        *)     return 1 ;;
    esac
}

ask_input() {
    PROMPT="$1"
    printf "  ${GREEN}?${RESET} %s " "$PROMPT"
    read -r REPLY < /dev/tty
    echo "$REPLY"
}

show_banner() {
    echo ""
    echo "${GREEN}${BOLD}"
    echo "                  ╱▔╲"
    echo "                ╱    ╲"
    echo "       ╲      ╱      ╱"
    echo "        ╲╲  ╱╱     ╱╱"
    echo "         ╲╲╱╱    ╱╱"
    echo "          ╲╱   ╱╱"
    echo "          ╱  ╱╱"
    echo "        ╱╱ ╱╱"
    echo "       ╱╱╱╱"
    echo "      ╱╱${RESET}"
    echo ""
    echo "    ${BOLD}${WHITE}B E A C O N${RESET}  ${DIM}v${VERSION}${RESET}"
    echo "    ${DIM}Smart radar for your deployments${RESET}"
    echo ""
    echo "    ${DIM}github.com/Blysspeak/beacon${RESET}"
    echo ""
}

show_complete() {
    echo ""
    echo "  ${GREEN}${BOLD}╔══════════════════════════════════════╗${RESET}"
    echo "  ${GREEN}${BOLD}║${RESET}                                      ${GREEN}${BOLD}║${RESET}"
    echo "  ${GREEN}${BOLD}║${RESET}   ${LGREEN}${BOLD}Installation complete!${RESET}              ${GREEN}${BOLD}║${RESET}"
    echo "  ${GREEN}${BOLD}║${RESET}                                      ${GREEN}${BOLD}║${RESET}"
    echo "  ${GREEN}${BOLD}╚══════════════════════════════════════╝${RESET}"
    echo ""
    echo "  ${BOLD}Quick start:${RESET}"
    echo ""
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon push${RESET}              ${DIM}git push + monitor deploy${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon watch${RESET}             ${DIM}monitor current deploy${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon status${RESET}            ${DIM}last deploy result${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon remote connect${RESET}    ${DIM}Telegram alerts${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon install${RESET}           ${DIM}re-setup Claude Code hooks${RESET}"
    echo ""
    echo "  ${DIM}Docs: https://github.com/Blysspeak/beacon${RESET}"
    echo ""
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
        linux*)  OS_DISPLAY="Linux";  OS="unknown-linux-gnu" ;;
        darwin*) OS_DISPLAY="macOS";  OS="apple-darwin" ;;
        *)       die "Unsupported OS: $(uname -s)" ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH_DISPLAY="x86_64";  ARCH="x86_64" ;;
        aarch64|arm64)   ARCH_DISPLAY="ARM64";   ARCH="aarch64" ;;
        armv7*)          ARCH_DISPLAY="ARMv7";   ARCH="armv7" ;;
        *)               die "Unsupported architecture: $ARCH" ;;
    esac

    if [ "$OS" = "unknown-linux-gnu" ]; then
        if command -v ldd >/dev/null 2>&1; then
            case "$(ldd --version 2>&1)" in
                *musl*) OS="unknown-linux-musl"; OS_DISPLAY="Linux (musl)" ;;
            esac
        fi
    fi

    TARGET="${ARCH}-${OS}"
    info "Platform:  ${BOLD}${OS_DISPLAY} ${ARCH_DISPLAY}${RESET}"
}

# --- Download tool ---

download() {
    URL="$1"; OUTPUT="$2"
    if command -v curl >/dev/null 2>&1; then
        if [ "$OUTPUT" = "-" ]; then curl -fsSL "$URL"; else curl -fsSL "$URL" -o "$OUTPUT"; fi
    elif command -v wget >/dev/null 2>&1; then
        if [ "$OUTPUT" = "-" ]; then wget -qO- "$URL"; else wget -q "$URL" -O "$OUTPUT"; fi
    else
        die "Neither curl nor wget found"
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
    info "Directory: ${BOLD}${INSTALL_DIR}${RESET}"
}

try_exec() {
    if [ -w "$INSTALL_DIR" ] || [ ! -d "$INSTALL_DIR" ]; then "$@"
    elif command -v sudo >/dev/null 2>&1; then sudo "$@"
    elif command -v doas >/dev/null 2>&1; then doas "$@"
    else die "Cannot write to ${INSTALL_DIR}. Set BEACON_INSTALL_DIR."; fi
}

# --- Build from source ---

build_from_source() {
    if ! command -v cargo >/dev/null 2>&1; then
        die "Rust not found. Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi

    info "Building from source..."
    echo ""

    TOTAL_CRATES=$(grep -c '^\[' Cargo.lock 2>/dev/null || echo "?")

    cargo build --release 2>&1 | while IFS= read -r line; do
        case "$line" in
            *Compiling*)
                CRATE=$(echo "$line" | sed 's/.*Compiling \([^ ]*\).*/\1/')
                printf "\r  ${DIM}  Compiling %-30s${RESET}" "$CRATE"
                ;;
            *Finished*)
                printf "\r%-60s\r" " "
                ;;
        esac
    done

    if [ ! -f "target/release/$BINARY" ]; then
        die "Build failed"
    fi

    success "Build complete"
}

# --- Install binary ---

install_binary() {
    mkdir -p "$INSTALL_DIR"

    if [ -f "Cargo.toml" ] && [ -f "target/release/$BINARY" ]; then
        BIN_SOURCE="target/release/$BINARY"
    elif [ -f "Cargo.toml" ]; then
        build_from_source
        BIN_SOURCE="target/release/$BINARY"
    else
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
            || die "Download failed for ${TARGET}"

        tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"
        BIN_SOURCE=$(find "$TMP_DIR" -name "$BINARY" -type f | head -1)
        [ -z "$BIN_SOURCE" ] && die "Binary not found in archive"
    fi

    chmod +x "$BIN_SOURCE"
    try_exec /usr/bin/cp "$BIN_SOURCE" "$INSTALL_DIR/$BINARY"

    if "$INSTALL_DIR/$BINARY" --version >/dev/null 2>&1; then
        INSTALLED_VERSION=$("$INSTALL_DIR/$BINARY" --version 2>/dev/null | head -1)
        success "${BOLD}${INSTALLED_VERSION}${RESET} installed"
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
        warn "Claude Code not detected"
        dim "Run ${BOLD}beacon install${RESET} after installing Claude Code"
        return 1
    fi

    if ! command -v jq >/dev/null 2>&1; then
        warn "jq is required for hook setup"
        dim "Install jq, then run: beacon install"
        return 1
    fi

    HOOK_DIR="$CLAUDE_DIR/hooks"
    HOOK_PATH="$HOOK_DIR/beacon-deploy-check.sh"
    mkdir -p "$HOOK_DIR"
    echo "$HOOK_SCRIPT_CONTENT" > "$HOOK_PATH"
    chmod +x "$HOOK_PATH"

    SETTINGS="$CLAUDE_DIR/settings.json"

    if [ -f "$SETTINGS" ] && grep -q "beacon-deploy-check" "$SETTINGS" 2>/dev/null; then
        success "Hooks already configured"
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

    success "Claude Code hooks installed"
    dim "  After git push → auto-monitor deploy"
    dim "  If deploy fails → Claude gets warned"
    return 0
}

# --- Telegram setup ---

setup_telegram() {
    echo ""
    echo "  ${GREEN}│${RESET} Open ${BOLD}@BeaconCIBot${RESET} in Telegram"
    echo "  ${GREEN}│${RESET} Press ${BOLD}/start${RESET} to get your token"
    echo "  ${GREEN}│${RESET}"
    echo ""

    TOKEN=$(ask_input "Paste your token:")

    if [ -z "$TOKEN" ]; then
        warn "Skipped"
        dim "Connect later: beacon remote connect <TOKEN>"
        return
    fi

    # connect with local API for testing, or remote
    if [ -n "$BEACON_API_URL" ]; then
        "$INSTALL_DIR/$BINARY" remote connect "$TOKEN" --api-url "$BEACON_API_URL" 2>/dev/null \
            && success "Connected to Telegram" \
            || warn "Connection failed"
    else
        "$INSTALL_DIR/$BINARY" remote connect "$TOKEN" 2>/dev/null \
            && success "Connected to Telegram" \
            || warn "Connection failed"
    fi

    echo ""
    if ask_yn "Send test notification?" "y"; then
        "$INSTALL_DIR/$BINARY" remote test 2>/dev/null \
            && success "Test sent — check Telegram!" \
            || warn "Test failed. Verify token and bot."
    fi
}

# --- PATH check ---

check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) return ;;
    esac

    echo ""
    warn "${INSTALL_DIR} is not in your PATH"

    SHELL_NAME=$(basename "${SHELL:-/bin/sh}")
    case "$SHELL_NAME" in
        zsh)  RC_FILE="$HOME/.zshrc" ; RC_DISPLAY="~/.zshrc" ;;
        fish) RC_FILE="$HOME/.config/fish/config.fish"; RC_DISPLAY="~/.config/fish/config.fish" ;;
        *)    RC_FILE="$HOME/.bashrc"; RC_DISPLAY="~/.bashrc" ;;
    esac

    echo ""
    if ask_yn "Add to PATH? (${RC_DISPLAY})" "y"; then
        if [ "$SHELL_NAME" = "fish" ]; then
            echo "" >> "$RC_FILE"
            echo "# Beacon CLI" >> "$RC_FILE"
            echo "fish_add_path ${INSTALL_DIR}" >> "$RC_FILE"
        else
            echo "" >> "$RC_FILE"
            echo "# Beacon CLI" >> "$RC_FILE"
            echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "$RC_FILE"
        fi
        success "Added to ${RC_DISPLAY}"
        export PATH="${INSTALL_DIR}:$PATH"
    else
        echo ""
        dim "Add manually:"
        dim "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi
}

# --- Main ---

main() {
    setup_colors
    show_banner

    # ── Step 1: Install ──
    step "1/4" "Install binary"

    detect_platform
    detect_install_dir
    echo ""
    install_binary

    # ── PATH ──
    check_path

    # ── Step 2: Claude Code ──
    step "2/4" "Claude Code integration"

    if ask_yn "Set up Claude Code hooks?" "y"; then
        setup_claude_hooks || true
    else
        dim "Skip. Run later: beacon install"
    fi

    # ── Step 3: Telegram ──
    step "3/4" "Telegram notifications"

    if ask_yn "Connect Telegram?" "y"; then
        setup_telegram
    else
        dim "Skip. Run later: beacon remote connect <TOKEN>"
    fi

    # ── Done ──
    step "4/4" "Complete"
    show_complete
}

main "$@"
