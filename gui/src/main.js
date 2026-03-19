const REFRESH_INTERVAL = 2000;

let deploys = [];
let selectedIndex = -1;

async function loadHistory() {
  try {
    let text;

    if (window.__TAURI_INTERNALS__) {
      // Tauri v2 — read file via plugin
      const { readTextFile, BaseDirectory } = await import("@tauri-apps/plugin-fs");
      text = await readTextFile('.beacon/history.jsonl', { baseDir: BaseDirectory.Home });
    } else {
      // Dev mode — use beacon CLI via subprocess
      try {
        const resp = await fetch('/api/history');
        if (resp.ok) {
          deploys = await resp.json();
          render();
          return;
        }
      } catch {}
      return;
    }

    deploys = text
      .trim()
      .split('\n')
      .filter(Boolean)
      .map(line => {
        try { return JSON.parse(line); }
        catch { return null; }
      })
      .filter(Boolean)
      .reverse()
      .slice(0, 100);

    render();
  } catch (e) {
    // File might not exist yet
    if (!e.toString().includes('not found') && !e.toString().includes('No such file')) {
      console.error('Failed to load history:', e);
    }
  }
}

function timeAgo(ts) {
  const diff = Math.floor((Date.now() - new Date(ts).getTime()) / 1000);
  if (diff < 0) return 'now';
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

function statusIcon(status) {
  const icons = { success: '✓', failed: '✗', in_progress: '◉', not_found: '?' };
  return icons[status] || '?';
}

function shortRepo(repo) {
  return repo.includes('/') ? repo.split('/').pop() : repo;
}

function render() {
  const list = document.getElementById('deploys-list');
  const total = deploys.length;
  const success = deploys.filter(d => d.status === 'success').length;
  const failed = deploys.filter(d => d.status === 'failed').length;

  document.getElementById('stat-total').textContent = `${total} deploys`;
  document.getElementById('stat-success').textContent = `✓ ${success}`;
  document.getElementById('stat-failed').textContent = `✗ ${failed}`;

  if (deploys.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">⟐</div>
        <div class="empty-text">No deploys yet</div>
        <div class="empty-sub">Push some code to get started</div>
      </div>`;
    return;
  }

  list.innerHTML = deploys.map((d, i) => `
    <div class="deploy-item ${d.status === 'failed' ? 'failed' : ''} ${i === selectedIndex ? 'selected' : ''}"
         data-index="${i}">
      <div class="deploy-icon ${d.status}">${statusIcon(d.status)}</div>
      <div class="deploy-repo">${shortRepo(d.repo)}</div>
      <div class="deploy-workflow">${d.workflow_name || '-'}</div>
      <div class="deploy-branch">${d.branch}</div>
      <div class="deploy-commit">${(d.commit || '').slice(0, 7)}</div>
      <div class="deploy-time">${timeAgo(d.timestamp)}</div>
    </div>
  `).join('');

  // Attach click handlers
  list.querySelectorAll('.deploy-item').forEach(el => {
    el.addEventListener('click', () => selectDeploy(parseInt(el.dataset.index)));
  });
}

function selectDeploy(index) {
  selectedIndex = index;
  const d = deploys[index];
  if (!d) return;

  const panel = document.getElementById('details-panel');
  panel.classList.remove('hidden');

  const badge = document.getElementById('details-status');
  badge.textContent = d.status.toUpperCase().replace('_', ' ');
  badge.className = `status-badge ${d.status}`;

  document.getElementById('details-repo').textContent = d.repo;
  document.getElementById('details-workflow').textContent = d.workflow_name || '-';
  document.getElementById('details-branch').textContent = d.branch;
  document.getElementById('details-commit').textContent = (d.commit || '').slice(0, 12);
  document.getElementById('details-time').textContent = d.timestamp ? new Date(d.timestamp).toLocaleString() : '-';

  const failedEl = document.getElementById('details-failed');
  const failedList = document.getElementById('details-failed-list');
  if (d.failed_jobs?.length) {
    failedEl.classList.remove('hidden');
    failedList.innerHTML = d.failed_jobs.map(j => `<div class="details-failed-item">× ${j}</div>`).join('');
  } else {
    failedEl.classList.add('hidden');
  }

  const urlEl = document.getElementById('details-url');
  if (d.url) {
    urlEl.href = d.url;
    urlEl.style.display = 'inline-block';
  } else {
    urlEl.style.display = 'none';
  }

  render();
}

document.getElementById('btn-close-details').addEventListener('click', () => {
  document.getElementById('details-panel').classList.add('hidden');
  selectedIndex = -1;
  render();
});

document.getElementById('btn-refresh').addEventListener('click', loadHistory);

// Keyboard navigation
document.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowDown' || e.key === 'j') {
    e.preventDefault();
    selectDeploy(Math.min((selectedIndex < 0 ? -1 : selectedIndex) + 1, deploys.length - 1));
  } else if (e.key === 'ArrowUp' || e.key === 'k') {
    e.preventDefault();
    selectDeploy(Math.max(selectedIndex - 1, 0));
  } else if ((e.key === 'o' || e.key === 'Enter') && selectedIndex >= 0) {
    const d = deploys[selectedIndex];
    if (d?.url) window.open(d.url, '_blank');
  } else if (e.key === 'r') {
    loadHistory();
  }
});

// Initial load + auto-refresh
loadHistory();
setInterval(loadHistory, REFRESH_INTERVAL);
