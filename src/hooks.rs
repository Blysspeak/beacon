use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;

const HOOK_SCRIPT: &str = r#"#!/bin/sh
# Beacon deploy monitor hook for Claude Code
# PostToolUse on Bash: checks mailbox + starts monitoring after git push

HOOK_INPUT=$(cat)
TOOL_INPUT=$(echo "$HOOK_INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

command -v beacon >/dev/null 2>&1 || exit 0

# --- Mailbox check: report completed deploys ---
STATUS_JSON=$(beacon status --json 2>/dev/null)
if [ -n "$STATUS_JSON" ] && [ "$STATUS_JSON" != "null" ]; then
    STATUS=$(echo "$STATUS_JSON" | jq -r '.status // empty' 2>/dev/null)
    REPO=$(echo "$STATUS_JSON" | jq -r '.repo // empty' 2>/dev/null)
    BRANCH=$(echo "$STATUS_JSON" | jq -r '.branch // empty' 2>/dev/null)

    case "$STATUS" in
        failed)
            echo "DEPLOY FAILED: $REPO ($BRANCH). Run 'beacon status' for details. Fix the issue before continuing."
            ;;
    esac
fi

# --- Start monitoring after git push ---
case "$TOOL_INPUT" in
    git\ push*|git\ push)
        git remote get-url origin >/dev/null 2>&1 || exit 0
        beacon watch --daemon 2>/dev/null || true
        echo "Beacon: deploy monitoring started in background"
        ;;
esac

exit 0
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

    // Write hook script
    let hook_path = hooks_dir.join("beacon-deploy-check.sh");
    fs::write(&hook_path, HOOK_SCRIPT).context("failed to write hook script")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("  Hook script: {}", hook_path.display());

    // Update settings.json
    let settings_path = claude_dir.join("settings.json");
    update_settings(&settings_path, &hook_path)?;

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

    // Check if beacon hook already exists
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

    // Build new hook entry
    let hook_entry = serde_json::json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": hook_cmd,
            "timeout": 10
        }]
    });

    // Ensure path exists: hooks.PostToolUse[]
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

    // Atomic write
    let tmp = settings_path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&settings)?;
    fs::write(&tmp, &json).context("failed to write settings")?;
    fs::rename(&tmp, settings_path).context("failed to save settings")?;

    println!("  Hook added to settings.json");

    Ok(())
}

pub fn uninstall_claude_hook() -> Result<()> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let claude_dir = PathBuf::from(&home).join(".claude");
    let settings_path = claude_dir.join("settings.json");
    let hook_path = claude_dir.join("hooks/beacon-deploy-check.sh");

    // Remove hook script
    if hook_path.exists() {
        fs::remove_file(&hook_path)?;
        println!("  Removed hook script");
    }

    // Remove from settings.json
    if settings_path.exists() {
        let data = fs::read_to_string(&settings_path)?;
        let mut settings: serde_json::Value = serde_json::from_str(&data)?;

        if let Some(hooks) = settings.get_mut("hooks") {
            if let Some(post) = hooks.get_mut("PostToolUse") {
                if let Some(arr) = post.as_array_mut() {
                    arr.retain(|entry| {
                        let is_beacon = entry
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
                            .unwrap_or(false);
                        !is_beacon
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
