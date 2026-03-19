#!/usr/bin/env python3
"""Waybar module: Beacon deploy status. Refreshed via SIGRTMIN+8 (no polling)."""
import json, os, sys, time
from datetime import datetime, timezone

STATUS_FILE = os.path.expanduser("~/.beacon/last_deploy.json")


def main():
    if not os.path.exists(STATUS_FILE):
        print(json.dumps({
            "text": "⟐ No deploys",
            "tooltip": "Beacon: no data yet\nRun beacon push to start",
            "class": "idle",
        }))
        return

    try:
        with open(STATUS_FILE) as f:
            d = json.load(f)
    except Exception:
        print(json.dumps({"text": "⟐ Error", "tooltip": "Failed to read status", "class": "idle"}))
        return

    status = d.get("status", "unknown")
    repo = d.get("repo", "")
    branch = d.get("branch", "")
    commit = d.get("commit", "")[:7]
    workflow = d.get("workflow_name") or ""
    url = d.get("url") or ""
    timestamp = d.get("timestamp") or ""
    failed_jobs = d.get("failed_jobs") or []
    short_repo = repo.split("/")[-1] if "/" in repo else repo
    label = workflow if workflow else branch

    text_map = {
        "success": (f"✓ {short_repo}/{label}", "success"),
        "failed": (f"✗ {short_repo}/{label}", "failed"),
        "in_progress": (f"◉ {short_repo}/{label}", "progress"),
        "not_found": (f"⟐ {short_repo}..." if short_repo else "⟐ Waiting...", "waiting"),
    }
    text, css = text_map.get(status, ("⟐ --", "idle"))

    # Tooltip
    lines = ["Beacon — Deploy Status", "─" * 21]
    status_labels = {
        "success": "✅ SUCCESS",
        "failed": "❌ FAILED",
        "in_progress": "⏳ IN PROGRESS",
        "not_found": "🔍 Waiting...",
    }
    lines.append(status_labels.get(status, f"Status: {status}"))

    if repo:
        lines.append(f"Repo: {repo}")
    if branch:
        lines.append(f"Branch: {branch}")
    if commit:
        lines.append(f"Commit: {commit}")
    if workflow:
        lines.append(f"Workflow: {workflow}")

    if timestamp:
        try:
            ts = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
            diff = int(time.time() - ts.timestamp())
            if diff < 60:
                ago = f"{diff}s ago"
            elif diff < 3600:
                ago = f"{diff // 60}m ago"
            elif diff < 86400:
                ago = f"{diff // 3600}h ago"
            else:
                ago = f"{diff // 86400}d ago"
            lines.append(f"Time: {ago}")
        except Exception:
            pass

    if failed_jobs and status == "failed":
        lines.append("─" * 21)
        lines.append("Failed jobs:")
        for job in failed_jobs[:5]:
            lines.append(f"  × {job}")

    if url:
        lines.append("─" * 21)
        lines.append("Click to open in browser")

    print(json.dumps({"text": text, "tooltip": "\n".join(lines), "class": css}))


if __name__ == "__main__":
    main()
