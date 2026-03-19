#!/usr/bin/env python3
"""Waybar module: Beacon multi-repo deploy status. Refreshed via SIGRTMIN+8."""
import json, os, sys, time
from datetime import datetime, timezone

HISTORY_FILE = os.path.expanduser("~/.beacon/history.jsonl")
STATUS_FILE = os.path.expanduser("~/.beacon/last_deploy.json")
RECENCY_MINUTES = 30
MAX_REPOS = 5


def read_history_by_repo():
    """Read recent deploys, return latest per repo."""
    if not os.path.exists(HISTORY_FILE):
        return []

    cutoff = time.time() - (RECENCY_MINUTES * 60)
    entries = []

    try:
        with open(HISTORY_FILE) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    d = json.loads(line)
                    ts_str = d.get("timestamp", "")
                    if ts_str:
                        ts = datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
                        if ts.timestamp() > cutoff:
                            entries.append(d)
                except (json.JSONDecodeError, ValueError):
                    continue
    except Exception:
        return []

    # Latest per repo
    entries.reverse()
    seen = {}
    for e in entries:
        repo = e.get("repo", "")
        if repo and repo not in seen:
            seen[repo] = e
        if len(seen) >= MAX_REPOS:
            break

    return list(seen.values())


def read_fallback():
    """Fall back to last_deploy.json if no history."""
    if not os.path.exists(STATUS_FILE):
        return []
    try:
        with open(STATUS_FILE) as f:
            return [json.load(f)]
    except Exception:
        return []


def format_entry(d):
    """Format single deploy for waybar text."""
    status = d.get("status", "unknown")
    repo = d.get("repo", "")
    workflow = d.get("workflow_name") or d.get("branch", "")
    short_repo = repo.split("/")[-1] if "/" in repo else repo

    # Shorten workflow name (Deploy Backend → Back, Deploy Frontend → Front)
    short_wf = workflow
    if workflow.startswith("Deploy "):
        short_wf = workflow[7:]

    icons = {
        "success": "✓",
        "failed": "✗",
        "in_progress": "◉",
        "not_found": "⟐",
    }
    icon = icons.get(status, "?")

    return f"{icon} {short_repo}/{short_wf}"


def worst_class(entries):
    """Return CSS class based on worst status."""
    statuses = {e.get("status") for e in entries}
    if "failed" in statuses:
        return "failed"
    if "in_progress" in statuses:
        return "progress"
    if "success" in statuses:
        return "success"
    return "idle"


def build_tooltip(entries):
    """Build detailed tooltip."""
    lines = ["Beacon — Deploy Status", "─" * 28]

    for d in entries:
        status = d.get("status", "unknown")
        repo = d.get("repo", "")
        branch = d.get("branch", "")
        commit = d.get("commit", "")[:7]
        workflow = d.get("workflow_name") or ""
        url = d.get("url") or ""
        timestamp = d.get("timestamp") or ""
        failed_jobs = d.get("failed_jobs") or []

        status_map = {
            "success": "✅",
            "failed": "❌",
            "in_progress": "⏳",
            "not_found": "🔍",
        }
        icon = status_map.get(status, "?")

        lines.append(f"{icon} {repo}")
        if branch:
            lines.append(f"   Branch: {branch}")
        if workflow:
            lines.append(f"   Workflow: {workflow}")
        if commit:
            lines.append(f"   Commit: {commit}")

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
                lines.append(f"   Time: {ago}")
            except Exception:
                pass

        if failed_jobs and status == "failed":
            for job in failed_jobs[:3]:
                lines.append(f"   × {job}")

        lines.append("")

    # Remove trailing empty line
    if lines and lines[-1] == "":
        lines.pop()

    return "\n".join(lines)


def main():
    entries = read_history_by_repo()
    if not entries:
        entries = read_fallback()

    if not entries:
        print(json.dumps({
            "text": "⟐ No deploys",
            "tooltip": "Beacon: no deploy data yet\nRun beacon push to start",
            "class": "idle",
        }))
        return

    # Filter out not_found
    visible = [e for e in entries if e.get("status") != "not_found"]
    if not visible:
        visible = entries

    # Build text: multi-repo
    parts = [format_entry(e) for e in visible]
    text = "  ".join(parts)

    css = worst_class(visible)
    tooltip = build_tooltip(visible)

    print(json.dumps({"text": text, "tooltip": tooltip, "class": css}))


if __name__ == "__main__":
    main()
