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
    echo "  ${LGREEN}${BOLD}  ██████╗ ███████╗ █████╗  ██████╗ ██████╗ ███╗   ██╗${RESET}"
    echo "  ${GREEN}${BOLD}  ██╔══██╗██╔════╝██╔══██╗██╔════╝██╔═══██╗████╗  ██║${RESET}"
    echo "  ${GREEN}${BOLD}  ██████╔╝█████╗  ███████║██║     ██║   ██║██╔██╗ ██║${RESET}"
    echo "  ${GREEN}${BOLD}  ██╔══██╗██╔══╝  ██╔══██║██║     ██║   ██║██║╚██╗██║${RESET}"
    echo "  ${GREEN}${BOLD}  ██████╔╝███████╗██║  ██║╚██████╗╚██████╔╝██║ ╚████║${RESET}"
    echo "  ${DIM}  ╚═════╝ ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═══╝${RESET}"
    echo ""
    echo "  ${DIM}  Smart radar for your deployments${RESET}     ${DIM}v${VERSION}${RESET}"
    echo "  ${DIM}  github.com/Blysspeak/beacon${RESET}"
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
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon push${RESET}              ${DIM}git push + auto-monitor${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon status${RESET}            ${DIM}last deploy result${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon watch${RESET}             ${DIM}manual foreground monitor${RESET}"
    echo "    ${GREEN}\$${RESET} ${BOLD}beacon remote connect${RESET}    ${DIM}Telegram alerts${RESET}"
    echo ""
    echo "  ${DIM}Daemon: systemctl --user status beacon${RESET}"
    echo "  ${DIM}Logs:   journalctl --user -u beacon -f${RESET}"
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

    info "Building from source (this may take 1-2 minutes)..."
    echo ""

    # Build and show progress — pipe to subshell for live output
    if cargo build --release 2>&1 | while IFS= read -r line; do
        case "$line" in
            *Compiling*)
                CRATE=$(echo "$line" | sed 's/.*Compiling \([^ ]*\).*/\1/')
                printf "\r  ${DIM}  Compiling %-40s${RESET}" "$CRATE"
                ;;
            *Finished*)
                printf "\r%-60s\r" " "
                ;;
            *error*)
                printf "\r%-60s\r" " "
                echo "  ${RED}$line${RESET}"
                ;;
        esac
    done; then
        : # success
    else
        echo ""
        die "Build failed. Check error output above."
    fi

    if [ ! -f "target/release/$BINARY" ]; then
        die "Build failed — binary not produced"
    fi

    success "Build complete"
}

# --- Install binary ---

install_binary() {
    try_exec mkdir -p "$INSTALL_DIR" || die "Cannot create ${INSTALL_DIR}"

    # Check if already installed
    if command -v "$BINARY" >/dev/null 2>&1; then
        EXISTING=$("$BINARY" --version 2>/dev/null | head -1)
        info "Found existing: ${DIM}${EXISTING}${RESET}"
    fi

    BIN_SOURCE=""

    # Strategy 1: Try downloading latest release from GitHub
    info "Fetching latest release from GitHub..."

    if [ "$VERSION" = "latest" ]; then
        VERSION=$(download "https://api.github.com/repos/${REPO}/releases/latest" - 2>/dev/null \
            | grep '"tag_name"' | head -1 \
            | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
    fi

    if [ -n "$VERSION" ]; then
        ARCHIVE="${BINARY}-${VERSION}-${TARGET}.tar.gz"
        URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

        TMP_DIR=$(mktemp -d)
        trap 'rm -rf "$TMP_DIR"' EXIT

        if download "$URL" "$TMP_DIR/$ARCHIVE" 2>/dev/null; then
            tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR" 2>/dev/null
            BIN_SOURCE=$(find "$TMP_DIR" -name "$BINARY" -type f | head -1)

            if [ -n "$BIN_SOURCE" ]; then
                success "Downloaded ${BOLD}${VERSION}${RESET}"
            else
                warn "Archive downloaded but binary not found inside"
                BIN_SOURCE=""
            fi
        else
            warn "No pre-built binary for ${BOLD}${TARGET}${RESET}"
        fi
    else
        warn "No releases found on GitHub"
    fi

    # Strategy 2: Build from source (fallback)
    if [ -z "$BIN_SOURCE" ]; then
        if [ -f "Cargo.toml" ]; then
            info "Falling back to build from source..."
            build_from_source
            BIN_SOURCE="target/release/$BINARY"
        else
            die "No binary available and not in source directory. Clone the repo first:\n  git clone https://github.com/${REPO} && cd beacon && bash install.sh"
        fi
    fi

    # Install
    chmod +x "$BIN_SOURCE"
    try_exec /usr/bin/cp "$BIN_SOURCE" "$INSTALL_DIR/$BINARY"

    if "$INSTALL_DIR/$BINARY" --version >/dev/null 2>&1; then
        INSTALLED_VERSION=$("$INSTALL_DIR/$BINARY" --version 2>/dev/null | head -1)
        success "${BOLD}${INSTALLED_VERSION}${RESET} installed to ${INSTALL_DIR}/"
    else
        success "Installed to ${INSTALL_DIR}/${BINARY}"
    fi
}

# --- Daemon (systemd) ---

setup_daemon() {
    # Check if systemd user services are available
    if ! command -v systemctl >/dev/null 2>&1; then
        warn "systemctl not found"
        dim "Run daemon manually: beacon daemon &"
        return
    fi

    if ! systemctl --user status >/dev/null 2>&1; then
        warn "systemd user services not available"
        dim "Run daemon manually: beacon daemon &"
        return
    fi

    SERVICE_DIR="$HOME/.config/systemd/user"
    SERVICE_PATH="$SERVICE_DIR/beacon.service"
    mkdir -p "$SERVICE_DIR"

    # Write service file
    cat > "$SERVICE_PATH" << SVCEOF
[Unit]
Description=Beacon — CI/CD deploy monitor daemon
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/beacon daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
SVCEOF

    # Enable and start
    systemctl --user daemon-reload 2>/dev/null
    systemctl --user enable --now beacon.service 2>/dev/null

    if systemctl --user is-active beacon.service >/dev/null 2>&1; then
        success "Daemon running (systemd)"
        dim "  Persistent — auto-restarts on failure"
        dim "  Logs: journalctl --user -u beacon -f"
    else
        warn "Daemon failed to start"
        dim "Check: systemctl --user status beacon"
        dim "Or run manually: beacon daemon"
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
    *git\ push*)
        WORK_DIR=$(echo "$TOOL_INPUT" | sed -n '"'"'s/.*cd \([^ &;]*\).*/\1/p'"'"' | head -1)
        if [ -n "$WORK_DIR" ] && [ -d "$WORK_DIR" ]; then
            (cd "$WORK_DIR" && beacon watch --daemon 2>/dev/null) && echo "Beacon: monitoring deploy" || true
        else
            beacon watch --daemon 2>/dev/null && echo "Beacon: monitoring deploy" || true
        fi ;;
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
        # Validate existing settings.json is valid JSON
        if ! jq empty "$SETTINGS" 2>/dev/null; then
            warn "settings.json is corrupted, creating backup"
            /usr/bin/cp "$SETTINGS" "$SETTINGS.bak"
        fi

        TMP=$(mktemp)
        if jq --argjson hook "$HOOK_ENTRY" '
            .hooks //= {} |
            .hooks.PostToolUse //= [] |
            .hooks.PostToolUse += [$hook]
        ' "$SETTINGS" > "$TMP" 2>/dev/null; then
            mv "$TMP" "$SETTINGS"
        else
            rm -f "$TMP"
            warn "Failed to update settings.json"
            dim "Run ${BOLD}beacon install${RESET} to retry"
            return 1
        fi
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
    echo "  ${GREEN}│${RESET} Open ${BOLD}@beacon_github_bot${RESET} in Telegram"
    echo "  ${GREEN}│${RESET} Press ${BOLD}/start${RESET} to get your token"
    echo "  ${GREEN}│${RESET} It looks like: ${DIM}xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx${RESET}"
    echo ""

    # Loop until valid token or user skips
    ATTEMPTS=0
    while true; do
        TOKEN=$(ask_input "Paste your token (or 'skip'):")

        # User wants to skip
        case "$TOKEN" in
            skip|SKIP|Skip|s|S|"")
                warn "Skipped"
                dim "Connect later: beacon remote connect <TOKEN>"
                return
                ;;
        esac

        # Basic validation: non-empty, contains dashes (looks like UUID)
        case "$TOKEN" in
            *-*-*-*-*)
                break
                ;;
            *)
                ATTEMPTS=$((ATTEMPTS + 1))
                if [ "$ATTEMPTS" -ge 3 ]; then
                    warn "Too many attempts"
                    dim "Connect later: beacon remote connect <TOKEN>"
                    return
                fi
                error "That doesn't look like a token from the bot"
                dim "Get it from /start in @beacon_github_bot"
                echo ""
                ;;
        esac
    done

    # Connect
    CONNECT_ARGS="$TOKEN"
    [ -n "$BEACON_API_URL" ] && CONNECT_ARGS="$TOKEN --api-url $BEACON_API_URL"

    if "$INSTALL_DIR/$BINARY" remote connect $CONNECT_ARGS 2>/dev/null; then
        success "Connected to Telegram"
    else
        warn "Connection failed"
        dim "Try manually: beacon remote connect $TOKEN"
        return
    fi

    # Test
    echo ""
    if ask_yn "Send test notification?" "y"; then
        if "$INSTALL_DIR/$BINARY" remote test 2>/dev/null; then
            success "Test sent — check Telegram!"
        else
            warn "Test failed. Check bot status and try: beacon remote test"
        fi
    fi
}

# --- Waybar widget ---

setup_waybar() {
    WAYBAR_DIR="$HOME/.config/waybar"
    MODULE_DIR="$WAYBAR_DIR/modules"
    MODULE_PATH="$MODULE_DIR/beacon.py"
    CONFIG_PATH="$WAYBAR_DIR/config"
    STYLE_PATH="$WAYBAR_DIR/style.css"

    mkdir -p "$MODULE_DIR"

    # Copy module script from contrib/ (if in repo) or download
    if [ -f "contrib/waybar/beacon.py" ]; then
        /usr/bin/cp contrib/waybar/beacon.py "$MODULE_PATH"
    else
        download "https://raw.githubusercontent.com/${REPO}/main/contrib/waybar/beacon.py" "$MODULE_PATH" \
            || { warn "Failed to download waybar module"; return 1; }
    fi
    chmod +x "$MODULE_PATH"
    success "Module script installed"

    # Add to waybar config if not already present
    if [ -f "$CONFIG_PATH" ]; then
        if grep -q "custom/beacon" "$CONFIG_PATH" 2>/dev/null; then
            success "Already in waybar config"
        else
            if command -v jq >/dev/null 2>&1 && jq empty "$CONFIG_PATH" 2>/dev/null; then
                TMP=$(mktemp)
                jq '
                    .["modules-left"] += ["custom/beacon"] |
                    .["custom/beacon"] = {
                        "format": "{}",
                        "return-type": "json",
                        "exec": "~/.config/waybar/modules/beacon.py",
                        "exec-on-event": false,
                        "interval": "once",
                        "signal": 8,
                        "on-click": "xdg-open $(jq -r '.url // empty' ~/.beacon/last_deploy.json 2>/dev/null) 2>/dev/null || true"
                    }
                ' "$CONFIG_PATH" > "$TMP" && mv "$TMP" "$CONFIG_PATH"
                success "Added to waybar config"
            else
                warn "Could not parse waybar config (not JSON or jq missing)"
                dim "Add \"custom/beacon\" to modules-left manually"
            fi
        fi
    else
        warn "waybar config not found at $CONFIG_PATH"
    fi

    # Add styles if not already present
    if [ -f "$STYLE_PATH" ]; then
        if grep -q "custom-beacon" "$STYLE_PATH" 2>/dev/null; then
            success "Styles already present"
        else
            if [ -f "contrib/waybar/style.css" ]; then
                printf '\n' >> "$STYLE_PATH"
                cat contrib/waybar/style.css >> "$STYLE_PATH"
            else
                STYLE_URL="https://raw.githubusercontent.com/${REPO}/main/contrib/waybar/style.css"
                printf '\n' >> "$STYLE_PATH"
                download "$STYLE_URL" - >> "$STYLE_PATH" 2>/dev/null
            fi
            success "Styles added to style.css"
        fi
    fi

    dim "  Widget refreshes instantly via signal (no polling)"
    dim "  Restart waybar to apply: pkill waybar && waybar &"
}

# --- PATH check ---

check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            success "Already in PATH"
            return
            ;;
    esac

    echo ""
    warn "${INSTALL_DIR} is not in your PATH"

    SHELL_NAME=$(basename "${SHELL:-/bin/sh}")
    case "$SHELL_NAME" in
        zsh)  RC_FILE="$HOME/.zshrc" ; RC_DISPLAY="~/.zshrc" ;;
        fish) RC_FILE="$HOME/.config/fish/config.fish"; RC_DISPLAY="~/.config/fish/config.fish" ;;
        *)    RC_FILE="$HOME/.bashrc"; RC_DISPLAY="~/.bashrc" ;;
    esac

    REAL_RC=$(eval echo "$RC_FILE")

    echo ""
    if ask_yn "Add to PATH? (${RC_DISPLAY})" "y"; then
        # Don't add duplicate entry
        if grep -q "# Beacon CLI" "$REAL_RC" 2>/dev/null; then
            success "Already in ${RC_DISPLAY}"
        else
            [ ! -f "$REAL_RC" ] && touch "$REAL_RC"
            if [ "$SHELL_NAME" = "fish" ]; then
                printf '\n# Beacon CLI\nfish_add_path %s\n' "$INSTALL_DIR" >> "$REAL_RC"
            else
                printf '\n# Beacon CLI\nexport PATH="%s:$PATH"\n' "$INSTALL_DIR" >> "$REAL_RC"
            fi
            success "Added to ${RC_DISPLAY}"
        fi
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
    step "1/6" "Install binary"

    detect_platform
    detect_install_dir
    echo ""
    install_binary

    # ── PATH ──
    check_path

    # ── Step 2: Daemon ──
    step "2/6" "Background daemon"

    setup_daemon

    # ── Step 3: Claude Code ──
    step "3/6" "Claude Code integration"

    if ask_yn "Set up Claude Code hooks?" "y"; then
        setup_claude_hooks || true
        dim "  PreToolUse: warns Claude if deploy is broken"
        dim "  PostToolUse: auto-enqueues push for monitoring"
    else
        dim "Skip. Run later: beacon install"
    fi

    # ── Step 4: Telegram ──
    step "4/6" "Telegram notifications"

    if ask_yn "Connect Telegram?" "y"; then
        setup_telegram
    else
        dim "Skip. Run later: beacon remote connect <TOKEN>"
    fi

    # ── Step 5: Waybar (optional) ──
    step "5/6" "Waybar widget (optional)"

    if command -v waybar >/dev/null 2>&1 && [ -d "$HOME/.config/waybar" ]; then
        info "Waybar detected"
        if ask_yn "Install Beacon status widget for Waybar?" "n"; then
            setup_waybar
        else
            dim "Skip. You can set it up later manually."
        fi
    else
        dim "Waybar not detected — skipping"
    fi

    # ── Done ──
    step "6/6" "Complete"
    show_complete
}

main "$@"
