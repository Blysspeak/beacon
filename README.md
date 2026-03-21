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
  <em>Persistent daemon that auto-discovers your repos and monitors every CI/CD deploy.<br>
  GitHub polling · Telegram alerts · Waybar widget · Claude Code integration · Zero config.</em>
</p>

---

### The Problem

You `git push`, switch tasks, and the deploy **silently fails**. You keep coding on broken code. Hours later — a cascade of errors.

**With AI agents** (Claude Code, Cursor, Copilot) it's worse: they keep generating code on top of a failed deploy without knowing.

### The Fix

```
Push from anywhere → Beacon daemon detects it → polls GitHub Actions →
  ✅ Success → Telegram + Waybar green → keep working
  ❌ Failed  → Telegram + Waybar red → Claude Code stops and warns you
```

No hooks to install per-repo. No config. Beacon auto-discovers repos from your deploy history and polls GitHub every 60s.

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
                    ┌──────────────────────────────┐
                    │    beacon daemon (systemd)    │
                    │    always running              │
                    └──────────┬───────────────────┘
                               │
           ┌───────────────────┼───────────────────┐
           ↓                   ↓                   ↓
     GitHub Poller       Queue Watcher        Notifications
     (every 60s)         (hooks, instant)          │
           │                   │          ┌────────┼────────┐
           └─────────┬─────────┘          ↓        ↓        ↓
                     ↓               📱 Telegram  📊 Waybar  🤖 Claude
              Track workflow                                   Code
```

**Two detection modes — zero config:**

| Mode | Latency | How it works |
|------|---------|-------------|
| **GitHub Poller** | ~60s | Daemon polls GitHub Actions for all auto-discovered repos |
| **Hook (Claude Code)** | instant | PostToolUse hook catches `git push` in Claude sessions |

| Step | What happens |
|------|-------------|
| Push from anywhere | Poller detects new workflow run within 60s |
| Push via Claude Code | Hook enqueues immediately (< 10ms) |
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

### GitHub Polling (Auto-Discovery)

Beacon automatically discovers repos from your deploy history and polls GitHub Actions for new workflow runs. Push from terminal, IDE, GitHub web — Beacon catches it.

```bash
beacon poll list                  # see watched repos
beacon poll add owner/repo        # add repo manually
beacon poll remove owner/repo     # remove repo
beacon poll interval 30           # poll every 30s (default 60s)
```

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
| `beacon status` | Last deploy result |
| `beacon status --json` | Machine-readable output |
| `beacon log` | Deploy history |
| `beacon tui` | Interactive deploy dashboard |
| `beacon poll list` | Show watched repos (configured + auto-discovered) |
| `beacon poll add <repo>` | Add repo to watch list |
| `beacon poll remove <repo>` | Remove repo from watch list |
| `beacon poll interval <sec>` | Set poll interval (min 10s) |
| `beacon remote connect` | Connect Telegram |
| `beacon remote test` | Test Telegram |
| `beacon watch` | Manual foreground monitor |
| `beacon notify` | Enqueue current repo (used by hooks) |
| `beacon daemon` | Run daemon (managed by systemd) |
| `beacon install` | Setup hooks + daemon |
| `beacon uninstall` | Remove everything |

## Configuration

```
~/.beacon/
├── config.json        # Telegram + poll config
├── last_deploy.json   # Last deploy status (mailbox)
├── history.jsonl      # Full deploy history
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
- [x] `beacon log` — deploy history
- [x] Interactive TUI dashboard
- [x] GitHub polling with auto-discovery (no per-repo setup)
- [ ] Railway / Vercel / Fly.io providers
- [ ] Webhook mode (replace polling)

## Contributing

```bash
git clone https://github.com/Blysspeak/beacon
cd beacon
cargo build
cargo test
```

## License

[MIT](LICENSE) — use it however you want.
