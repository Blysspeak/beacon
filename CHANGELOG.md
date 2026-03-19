# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-19

### Added

- `beacon install` — auto-setup Claude Code PostToolUse hooks
- `beacon uninstall` — remove Claude Code hooks
- Interactive `install.sh` wizard with ASCII logo banner
  - Auto-detects platform (Linux/macOS, x86_64/ARM64, glibc/musl)
  - Downloads latest release from GitHub or builds from source
  - Auto-adds to PATH with shell detection (bash/zsh/fish)
  - Claude Code hook setup with Y/n prompt
  - Telegram connection with token validation and test
- Claude Code hook: auto-monitors deploy after `git push`, warns on failures
- GitHub Actions CI workflow for multi-platform release builds
- Production API URL: `beacon.blysspeak.space`

### Changed

- Default API URL changed from placeholder to `https://beacon.blysspeak.space`
- Token validation: empty tokens rejected with helpful error
- Install binary strategy: always fetch latest from GitHub first, fallback to source build

## [0.1.0] - 2026-03-19

### Added

- `beacon watch` — monitor deploy status in foreground with live spinner
- `beacon watch --daemon` — background monitoring with PID file and log output
- `beacon push [args]` — wraps `git push` + auto-starts deploy monitoring
- `beacon status` — read last deploy result from local mailbox (`~/.beacon/last_deploy.json`)
- `beacon status --json` — machine-readable JSON output
- `beacon remote connect <TOKEN>` — connect to Beacon Telegram bot for notifications
- `beacon remote test` — send test notification to verify connection
- `beacon remote disconnect` — remove Telegram integration
- GitHub Actions provider with automatic token resolution (`GITHUB_TOKEN` env / `gh auth token`)
- Git remote auto-detection (SSH and HTTPS formats)
- URL encoding for branch names with special characters (e.g. `feature/foo`)
- Adaptive polling: 5s for first 2 min, then 15s, max 30 min timeout
- Atomic file writes for mailbox and config (write-to-tmp + rename)
- HTTP request timeouts (15s for GitHub API, 10s for Beacon API)
- Colored terminal output with status badges
- Detailed error messages for GitHub API failures (401, 403, 404)
