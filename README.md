<p align="center">
  <img src="logo.png" alt="Beacon" width="180" />
</p>

<h1 align="center">Beacon</h1>

<p align="center">
  <strong>Smart radar for your deployments</strong><br>
  Monitors CI/CD status after <code>git push</code> and alerts you instantly via terminal & Telegram.
</p>

<p align="center">
  <a href="https://github.com/Blysspeak/beacon/releases"><img src="https://img.shields.io/github/v/tag/Blysspeak/beacon?label=version&color=green" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License"></a>
  <img src="https://img.shields.io/badge/language-Rust-orange" alt="Rust">
  <img src="https://img.shields.io/badge/provider-GitHub%20Actions-purple" alt="GitHub Actions">
</p>

---

## The Problem

You run `git push`, switch to the next task, and the deployment **silently fails** in the background. You keep coding on top of a broken build. Hours later you discover a cascade of errors — all because nobody checked the deploy.

This is especially critical when working with **AI coding agents** (Claude Code, Cursor, Copilot) that keep generating code on top of a failed deploy without realizing it.

## The Solution

Beacon watches your CI/CD pipeline after every push and **comes back with the result** — like a boomerang.

```
git push → Beacon starts monitoring → polls GitHub Actions →
  ✅ Deploy succeeded → you keep working
  ❌ Deploy failed → you (and your AI agent) know immediately
```

## Quick Start

```bash
# One-line install (interactive wizard: binary + Claude Code hooks + Telegram)
git clone https://github.com/Blysspeak/beacon && cd beacon && bash install.sh
```

Or install from pre-built binary:
```bash
curl -fsSL https://raw.githubusercontent.com/Blysspeak/beacon/main/install.sh | sh
```

Or via Cargo:
```bash
cargo install beacon
beacon install    # set up Claude Code hooks
```

Then just use it:
```bash
beacon push       # git push + auto-monitor deploy
beacon watch      # monitor current deploy
beacon status     # last deploy result
```

## Commands

| Command | Description |
|---------|-------------|
| `beacon push [args]` | `git push` + automatic deploy monitoring |
| `beacon watch` | Monitor current deploy in foreground with live spinner |
| `beacon watch --daemon` | Monitor in background, logs to `~/.beacon/daemon.log` |
| `beacon status` | Show last deploy result from local mailbox |
| `beacon status --json` | Machine-readable output for integrations |
| `beacon remote connect <TOKEN>` | Connect Telegram notifications |
| `beacon remote test` | Verify Telegram connection |
| `beacon remote disconnect` | Remove Telegram integration |
| `beacon install` | Set up Claude Code hooks (auto-monitor after push) |
| `beacon uninstall` | Remove Claude Code hooks |

## Telegram Notifications

Get deploy results in Telegram — never miss a failed build.

```bash
# 1. Start @BeaconCIBot in Telegram → get your token
# 2. Connect
beacon remote connect <TOKEN>

# 3. Verify
beacon remote test
```

After this, every `beacon push` or `beacon watch` will send you a Telegram message when the deploy completes.

## Claude Code Integration

The installer sets up hooks automatically. Or do it manually:

```bash
beacon install
```

This adds a PostToolUse hook that:
- **After `git push`** — starts background deploy monitoring
- **Before every action** — checks if last deploy failed and warns Claude

If a deploy fails, Claude sees the error before generating more code on top of a broken build.

## Waybar Widget

Real-time deploy status in your status bar — no polling, instant updates via signal.

```
✓ myapp/Deploy Backend      ← green, success
✗ myapp/Deploy Frontend     ← red, failed
◉ myapp/Deploy Backend      ← yellow, in progress
```

The installer can set it up automatically, or install manually:

```bash
# Copy module
cp contrib/waybar/beacon.py ~/.config/waybar/modules/
chmod +x ~/.config/waybar/modules/beacon.py

# Add to waybar config (modules-left):
#   "custom/beacon"
# And add module config:
#   "custom/beacon": {
#     "format": "{}",
#     "return-type": "json",
#     "exec": "~/.config/waybar/modules/beacon.py",
#     "signal": 8,
#     "interval": "once"
#   }

# Append styles
cat contrib/waybar/style.css >> ~/.config/waybar/style.css
```

Click the widget to open the GitHub Actions run in browser.

## Architecture

Beacon runs as a **persistent systemd daemon** (like TimeForged). One process, always running.

```
                     ┌──────────────────────────┐
                     │   beacon daemon (systemd) │
                     │   polls ~/.beacon/queue/   │
                     └─────────┬────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        ↓                      ↓                      ↓
   mailbox::write()    telegram::send()    pkill SIGRTMIN+8
   ~/.beacon/          POST /notify        waybar refresh
   last_deploy.json    → Telegram 📱       instant update
```

**Push sources** (all write to `~/.beacon/queue/`):
- Claude Code hook → `beacon notify` (< 10ms, just writes a file)
- `beacon push` → `git push` + `beacon notify`
- Manual → `beacon notify` from any terminal

**Deploy tracking:**
1. Daemon picks up queue event → polls GitHub Actions
2. On completion → writes mailbox + sends Telegram + signals waybar
3. If deploy **failed** → Claude Code sees warning before next action (PreToolUse hook)

```bash
# Daemon management
systemctl --user status beacon     # check status
journalctl --user -u beacon -f     # live logs
systemctl --user restart beacon    # restart
```

## Configuration

Beacon stores its data in `~/.beacon/`:

```
~/.beacon/
├── config.json        # Remote connection settings
├── last_deploy.json   # Last deploy status (mailbox)
├── watcher.pid        # Daemon PID (when using --daemon)
└── daemon.log         # Daemon logs
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `GITHUB_TOKEN` | GitHub API token (falls back to `gh auth token`) |
| `BEACON_API_URL` | Custom Beacon Bot API URL |

## Roadmap

- [ ] Railway provider
- [ ] Vercel provider
- [ ] Fly.io provider
- [x] `beacon install` — auto-setup Claude Code hooks
- [x] Waybar widget with real-time updates
- [ ] Webhook mode (instead of polling)
- [ ] Multi-repo dashboard

## License

[MIT](LICENSE)
