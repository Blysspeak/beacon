<p align="center">
  <img src="logo.png" alt="Beacon" width="200" />
</p>

<h1 align="center">Beacon</h1>

<p align="center">
  <strong>Stop shipping broken code. Know your deploy status instantly.</strong>
</p>

<p align="center">
  <a href="https://github.com/Blysspeak/beacon/releases"><img src="https://img.shields.io/github/v/tag/Blysspeak/beacon?label=version&color=a6e3a1&style=flat-square" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/Rust-000?logo=rust&logoColor=white&style=flat-square" alt="Rust">
  <img src="https://img.shields.io/badge/GitHub%20Actions-2088FF?logo=githubactions&logoColor=white&style=flat-square" alt="GitHub Actions">
  <img src="https://img.shields.io/badge/Telegram-26A5E4?logo=telegram&logoColor=white&style=flat-square" alt="Telegram">
  <img src="https://img.shields.io/badge/Waybar-333?style=flat-square" alt="Waybar">
</p>

<p align="center">
  <em>Persistent daemon that monitors CI/CD after every <code>git push</code>.<br>
  Telegram alerts · Waybar widget · Claude Code integration · Zero config.</em>
</p>

---

### The Problem

You `git push`, switch tasks, and the deploy **silently fails**. You keep coding on broken code. Hours later — a cascade of errors.

**With AI agents** (Claude Code, Cursor, Copilot) it's worse: they keep generating code on top of a failed deploy without knowing.

### The Fix

```
git push → Beacon daemon catches it → polls GitHub Actions →
  ✅ Success → Telegram + Waybar green → keep working
  ❌ Failed  → Telegram + Waybar red → Claude Code stops and warns you
```

---

## Quick Start

```bash
git clone https://github.com/Blysspeak/beacon && cd beacon && bash install.sh
```

<details>
<summary><strong>What the installer does (6 steps)</strong></summary>

1. **Builds/downloads** the binary for your platform
2. **Starts systemd daemon** — persistent, auto-restarts
3. **Installs Claude Code hooks** — PreToolUse (warn on fail) + PostToolUse (enqueue push)
4. **Connects Telegram** — token from [@beacon_github_bot](https://t.me/beacon_github_bot)
5. **Waybar widget** (optional) — real-time status bar
6. **Done** — zero config after install

</details>

Or install manually:
```bash
cargo install beacon && beacon install
```

---

## How It Works

```
                    ┌──────────────────────────┐
                    │   beacon daemon (systemd) │
                    │   always running           │
                    └─────────┬────────────────┘
                              │
       ┌──────────────────────┼──────────────────────┐
       ↓                      ↓                      ↓
  📁 Mailbox            📱 Telegram           📊 Waybar
  last_deploy.json      instant alert         SIGRTMIN+8
       ↓
  🤖 Claude Code
  "DEPLOY FAILED — fix it
   before continuing"
```

**Push → Queue → Daemon → Track → Notify.** That's it.

| Step | What happens |
|------|-------------|
| You push | Hook writes `~/.beacon/queue/123.json` (< 10ms) |
| Daemon sees it | Starts polling GitHub Actions for that commit |
| Deploy completes | Writes result, sends Telegram, updates Waybar |
| Deploy **failed** | Claude Code sees warning before next action |

---

## Features

### Telegram Notifications

```bash
beacon remote connect <TOKEN>   # get token from /start in @beacon_github_bot
beacon remote test              # verify connection
```

<table><tr><td>✅ <strong>Deploy SUCCESS</strong><br>Repo: myapp<br>Branch: main<br>Workflow: Deploy Backend</td><td>❌ <strong>Deploy FAILED</strong><br>Repo: myapp<br>Branch: main<br>Failed: deploy (failure)</td></tr></table>

### Claude Code Integration

When a deploy fails, Claude sees this **before every action**:

```
⚠️  DEPLOY FAILED — owner/myapp (main)
   Workflow: Deploy Backend
   Failed: deploy (failure)
   Fix the deploy issue before continuing.
```

Claude stops and helps you fix it instead of piling code on a broken build.

### Waybar Widget

Real-time in your status bar. No polling — instant signal.

```
✓ myapp/Deploy Backend      ← green
✗ myapp/Deploy Frontend     ← red
◉ myapp/Deploy Backend      ← yellow (in progress)
```

Click → opens GitHub Actions run. Tooltip shows full details.

### Persistent Daemon

Runs as a systemd user service. Always on, auto-restarts.

```bash
systemctl --user status beacon     # status
journalctl --user -u beacon -f     # live logs
```

---

## Commands

| Command | Description |
|---------|-------------|
| `beacon push [args]` | `git push` + queue for monitoring |
| `beacon notify` | Enqueue current repo for daemon (used by hooks) |
| `beacon watch` | Manual foreground monitor with spinner |
| `beacon status` | Last deploy result |
| `beacon status --json` | Machine-readable output |
| `beacon daemon` | Run daemon (managed by systemd) |
| `beacon remote connect` | Connect Telegram |
| `beacon remote test` | Test Telegram |
| `beacon install` | Setup hooks + daemon |
| `beacon uninstall` | Remove everything |

## Configuration

```
~/.beacon/
├── config.json        # Telegram token + API URL
├── last_deploy.json   # Last deploy status
├── queue/             # Push events for daemon (file-based IPC)
└── daemon.log         # Daemon logs (if not using systemd)
```

| Env Variable | Description |
|-------------|-------------|
| `GITHUB_TOKEN` | GitHub API token (falls back to `gh auth token`) |
| `BEACON_API_URL` | Custom Beacon Bot API URL |

## Roadmap

- [x] GitHub Actions provider
- [x] Telegram notifications
- [x] Claude Code hooks (PreToolUse + PostToolUse)
- [x] Waybar widget with instant signal refresh
- [x] Persistent systemd daemon
- [x] Interactive installer with ASCII banner
- [ ] Railway / Vercel / Fly.io providers
- [ ] Multi-repo dashboard
- [ ] Webhook mode (replace polling)
- [ ] `beacon log` — deploy history

## Contributing

```bash
git clone https://github.com/Blysspeak/beacon
cd beacon
cargo build
cargo test
```

## License

[MIT](LICENSE) — use it however you want.
