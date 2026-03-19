use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;

const HOOK_SCRIPT: &str = r#"#!/bin/sh
# Beacon deploy monitor hook for Claude Code
# Enqueues push events for the persistent beacon daemon
HOOK_INPUT=$(cat)
TOOL_INPUT=$(echo "$HOOK_INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)
command -v beacon >/dev/null 2>&1 || exit 0

# Check mailbox for failed deploys
STATUS_JSON=$(beacon status --json 2>/dev/null)
if [ -n "$STATUS_JSON" ] && [ "$STATUS_JSON" != "null" ]; then
    STATUS=$(echo "$STATUS_JSON" | jq -r '.status // empty' 2>/dev/null)
    REPO_NAME=$(echo "$STATUS_JSON" | jq -r '.repo // empty' 2>/dev/null)
    BRANCH=$(echo "$STATUS_JSON" | jq -r '.branch // empty' 2>/dev/null)
    case "$STATUS" in
        failed) echo "DEPLOY FAILED: $REPO_NAME ($BRANCH). Run beacon status for details." ;;
    esac
fi

# Enqueue push event for daemon (instant — just writes a file)
case "$TOOL_INPUT" in
    *git\ push*)
        WORK_DIR=$(echo "$TOOL_INPUT" | sed -n 's/.*cd \([^ &;]*\).*/\1/p' | head -1)
        TARGET="${WORK_DIR:-.}"
        if [ -d "$TARGET" ]; then
            GIT_ROOT=$(cd "$TARGET" && git rev-parse --show-toplevel 2>/dev/null)
            if [ -n "$GIT_ROOT" ]; then
                (cd "$GIT_ROOT" && beacon notify 2>/dev/null) &
            fi
        fi
        ;;
esac
exit 0
"#;

const SYSTEMD_SERVICE: &str = r#"[Unit]
Description=Beacon — CI/CD deploy monitor daemon
After=network.target

[Service]
Type=simple
ExecStart=BEACON_BIN daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#;

pub fn install_claude_hook() -> Result<()> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let claude_dir = PathBuf::from(&home).join(".claude");

    if !claude_dir.exists() {
        bail!(
            "Claude Code directory not found (~/.claude).\n  \
             Install Claude Code first, then run `beacon install` again."
        );
    }

    let hooks_dir = claude_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).context("failed to create hooks directory")?;

    let hook_path = hooks_dir.join("beacon-deploy-check.sh");
    fs::write(&hook_path, HOOK_SCRIPT).context("failed to write hook script")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("  Hook script: {}", hook_path.display());
    update_settings(&claude_dir.join("settings.json"), &hook_path)?;

    Ok(())
}

pub fn install_systemd_service() -> Result<()> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let service_dir = PathBuf::from(&home).join(".config/systemd/user");
    fs::create_dir_all(&service_dir)?;

    let service_path = service_dir.join("beacon.service");

    // Find beacon binary path
    let beacon_bin = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from(format!("{home}/.cargo/bin/beacon")));

    let service_content = SYSTEMD_SERVICE.replace("BEACON_BIN", &beacon_bin.to_string_lossy());
    fs::write(&service_path, service_content)?;

    println!("  Systemd service: {}", service_path.display());

    // Enable and start
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "beacon.service"])
        .output();

    // Check status
    let status = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "beacon.service"])
        .output();

    match status {
        Ok(out) if out.status.success() => {
            println!("  Daemon: running");
        }
        _ => {
            println!("  Daemon: failed to start (check: systemctl --user status beacon)");
        }
    }

    Ok(())
}

pub fn uninstall_claude_hook() -> Result<()> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let claude_dir = PathBuf::from(&home).join(".claude");
    let settings_path = claude_dir.join("settings.json");
    let hook_path = claude_dir.join("hooks/beacon-deploy-check.sh");

    if hook_path.exists() {
        fs::remove_file(&hook_path)?;
        println!("  Removed hook script");
    }

    if settings_path.exists() {
        let data = fs::read_to_string(&settings_path)?;
        let mut settings: serde_json::Value = serde_json::from_str(&data)?;

        if let Some(hooks) = settings.get_mut("hooks") {
            if let Some(post) = hooks.get_mut("PostToolUse") {
                if let Some(arr) = post.as_array_mut() {
                    arr.retain(|entry| {
                        !entry
                            .get("hooks")
                            .and_then(|h| h.as_array())
                            .map(|arr| {
                                arr.iter().any(|h| {
                                    h.get("command")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.contains("beacon-deploy-check"))
                                        .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false)
                    });
                }
            }
        }

        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&settings_path, json)?;
        println!("  Removed hook from settings.json");
    }

    Ok(())
}

pub fn uninstall_systemd_service() -> Result<()> {
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "disable", "--now", "beacon.service"])
        .output();

    let home = std::env::var("HOME").context("HOME not set")?;
    let service_path = PathBuf::from(&home).join(".config/systemd/user/beacon.service");
    if service_path.exists() {
        fs::remove_file(&service_path)?;
        println!("  Removed systemd service");
    }

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    Ok(())
}

fn update_settings(settings_path: &PathBuf, hook_path: &PathBuf) -> Result<()> {
    let hook_cmd = hook_path.to_string_lossy().to_string();

    let mut settings: serde_json::Value = if settings_path.exists() {
        let data = fs::read_to_string(settings_path).context("failed to read settings.json")?;
        serde_json::from_str(&data).context("failed to parse settings.json")?
    } else {
        serde_json::json!({})
    };

    if let Some(hooks) = settings.get("hooks") {
        if let Some(post) = hooks.get("PostToolUse") {
            if let Some(arr) = post.as_array() {
                for entry in arr {
                    if let Some(inner) = entry.get("hooks") {
                        if let Some(inner_arr) = inner.as_array() {
                            for h in inner_arr {
                                if let Some(cmd) = h.get("command") {
                                    if cmd.as_str().unwrap_or("").contains("beacon-deploy-check") {
                                        println!("  Hook already configured in settings.json");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let hook_entry = serde_json::json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": hook_cmd,
            "timeout": 10
        }]
    });

    if settings.get("hooks").is_none() {
        settings["hooks"] = serde_json::json!({});
    }
    if settings["hooks"].get("PostToolUse").is_none() {
        settings["hooks"]["PostToolUse"] = serde_json::json!([]);
    }

    settings["hooks"]["PostToolUse"]
        .as_array_mut()
        .unwrap()
        .push(hook_entry);

    let tmp = settings_path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&settings)?;
    fs::write(&tmp, &json).context("failed to write settings")?;
    fs::rename(&tmp, settings_path).context("failed to save settings")?;

    println!("  Hook added to settings.json");

    Ok(())
}
