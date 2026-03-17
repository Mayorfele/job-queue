use actix_web::HttpResponse;

pub async fn serve() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(HTML)
}

const HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Job Queue Dashboard</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }

  body {
    background: #0f1117;
    color: #e2e8f0;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
    padding: 24px;
  }

  h1 {
    font-size: 18px;
    font-weight: 600;
    color: #f8fafc;
    margin-bottom: 24px;
    letter-spacing: 0.05em;
  }

  .stats {
    display: flex;
    gap: 12px;
    margin-bottom: 24px;
    flex-wrap: wrap;
  }

  .stat {
    background: #1e2130;
    border: 1px solid #2d3148;
    border-radius: 8px;
    padding: 12px 20px;
    min-width: 120px;
  }

  .stat-label {
    font-size: 11px;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 4px;
  }

  .stat-value {
    font-size: 24px;
    font-weight: 700;
  }

  .stat-value.total     { color: #f8fafc; }
  .stat-value.pending   { color: #94a3b8; }
  .stat-value.running   { color: #38bdf8; }
  .stat-value.retrying  { color: #fb923c; }
  .stat-value.completed { color: #4ade80; }
  .stat-value.dead      { color: #f87171; }

  table {
    width: 100%;
    border-collapse: collapse;
    background: #1e2130;
    border: 1px solid #2d3148;
    border-radius: 8px;
    overflow: hidden;
  }

  thead tr {
    background: #161824;
    border-bottom: 1px solid #2d3148;
  }

  th {
    padding: 10px 16px;
    text-align: left;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #64748b;
    font-weight: 600;
  }

  td {
    padding: 10px 16px;
    border-bottom: 1px solid #1a1d2e;
    vertical-align: middle;
  }

  tr:last-child td { border-bottom: none; }

  tr:hover td { background: #232640; }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge.pending   { background: #1e293b; color: #94a3b8; }
  .badge.running   { background: #0c2a3d; color: #38bdf8; }
  .badge.retrying  { background: #2d1f0e; color: #fb923c; }
  .badge.completed { background: #0f2d1a; color: #4ade80; }
  .badge.failed    { background: #2d1515; color: #f87171; }
  .badge.dead      { background: #2d1515; color: #f87171; }

  .type-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
  }

  .type-badge.order        { background: #1e1b4b; color: #a5b4fc; }
  .type-badge.payment      { background: #1a2e1a; color: #86efac; }
  .type-badge.notification { background: #2d1f0e; color: #fdba74; }

  .id-cell {
    font-family: monospace;
    font-size: 11px;
    color: #64748b;
  }

  .attempt-cell { color: #94a3b8; }
  .age-cell     { color: #64748b; font-size: 11px; }

  .error-cell {
    color: #f87171;
    font-size: 11px;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pulse {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #38bdf8;
    margin-right: 6px;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.3; }
  }

  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  .last-updated {
    font-size: 11px;
    color: #475569;
  }

  .empty-state {
    text-align: center;
    padding: 48px;
    color: #475569;
  }

  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
  }

  .tab {
    padding: 6px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid transparent;
    background: transparent;
    color: #64748b;
    transition: all 0.15s;
  }

  .tab:hover { color: #e2e8f0; background: #1e2130; }

  .tab.active {
    background: #1e2130;
    border-color: #2d3148;
    color: #f8fafc;
  }

  .requeue-btn {
    padding: 3px 10px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 4px;
    color: #94a3b8;
    cursor: pointer;
    font-size: 11px;
    font-family: inherit;
    transition: all 0.15s;
  }

  .requeue-btn:hover {
    background: #334155;
    color: #f8fafc;
  }
</style>
</head>
<body>

<div class="header-row">
  <h1>⚡ Job Queue Dashboard</h1>
  <span class="last-updated" id="last-updated">updating...</span>
</div>

<div class="stats">
  <div class="stat"><div class="stat-label">Total</div><div class="stat-value total"  id="s-total">—</div></div>
  <div class="stat"><div class="stat-label">Pending</div><div class="stat-value pending"   id="s-pending">—</div></div>
  <div class="stat"><div class="stat-label">Running</div><div class="stat-value running"   id="s-running">—</div></div>
  <div class="stat"><div class="stat-label">Retrying</div><div class="stat-value retrying" id="s-retrying">—</div></div>
  <div class="stat"><div class="stat-label">Completed</div><div class="stat-value completed" id="s-completed">—</div></div>
  <div class="stat"><div class="stat-label">Dead</div><div class="stat-value dead" id="s-dead">—</div></div>
</div>

<div class="tabs">
  <button class="tab active" onclick="switchTab('all')">All Jobs</button>
  <button class="tab" onclick="switchTab('dead')">Dead Letter Queue</button>
</div>

<table id="jobs-table">
  <thead>
    <tr>
      <th>Job ID</th>
      <th>Type</th>
      <th>Status</th>
      <th>Priority</th>
      <th>Attempt</th>
      <th>Age</th>
      <th>Error</th>
    </tr>
  </thead>
  <tbody id="jobs-body">
    <tr><td colspan="7" class="empty-state">loading...</td></tr>
  </tbody>
</table>

<script>
  let currentTab = 'all';

  function switchTab(tab) {
    currentTab = tab;
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    event.target.classList.add('active');

    const thead = document.querySelector('#jobs-table thead tr');
    if (tab === 'dead') {
      thead.innerHTML = `
        <th>Job ID</th><th>Type</th><th>Priority</th>
        <th>Attempts</th><th>Died</th><th>Error</th><th>Action</th>
      `;
    } else {
      thead.innerHTML = `
        <th>Job ID</th><th>Type</th><th>Status</th>
        <th>Priority</th><th>Attempt</th><th>Age</th><th>Error</th>
      `;
    }
    fetchData();
  }

  function age(isoString) {
    if (!isoString) return '—';
    const diff = Math.floor((Date.now() - new Date(isoString)) / 1000);
    if (diff < 60)   return diff + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    return Math.floor(diff / 3600) + 'h ago';
  }

  function priority(n) {
    if (n === 3) return 'high';
    if (n === 2) return 'normal';
    return 'low';
  }

  function shortId(id) {
    return id.substring(0, 8) + '...';
  }

  async function requeue(jobId) {
    await fetch('/jobs/' + jobId + '/requeue', { method: 'POST' });
    fetchData();
  }

  async function fetchData() {
    try {
      if (currentTab === 'dead') {
        const res = await fetch('/jobs/dead');
        const jobs = await res.json();
        renderDead(jobs);
      } else {
        const res = await fetch('/jobs');
        const jobs = await res.json();
        renderJobs(jobs);
        updateStats(jobs);
      }
      document.getElementById('last-updated').textContent =
        'updated ' + new Date().toLocaleTimeString();
    } catch (e) {
      console.error('fetch error', e);
    }
  }

  function updateStats(jobs) {
    const counts = { pending:0, running:0, retrying:0, completed:0, dead:0 };
    jobs.forEach(j => { if (counts[j.status] !== undefined) counts[j.status]++; });
    document.getElementById('s-total').textContent     = jobs.length;
    document.getElementById('s-pending').textContent   = counts.pending;
    document.getElementById('s-running').textContent   = counts.running;
    document.getElementById('s-retrying').textContent  = counts.retrying;
    document.getElementById('s-completed').textContent = counts.completed;
    document.getElementById('s-dead').textContent      = counts.dead;
  }

  function renderJobs(jobs) {
    const tbody = document.getElementById('jobs-body');
    if (!jobs.length) {
      tbody.innerHTML = '<tr><td colspan="7" class="empty-state">no jobs yet — enqueue something</td></tr>';
      return;
    }
    tbody.innerHTML = jobs.map(j => `
      <tr>
        <td class="id-cell" title="${j.id}">
          ${j.status === 'running' ? '<span class="pulse"></span>' : ''}
          ${shortId(j.id)}
        </td>
        <td><span class="type-badge ${j.job_type}">${j.job_type}</span></td>
        <td><span class="badge ${j.status}">${j.status}</span></td>
        <td>${priority(j.priority)}</td>
        <td class="attempt-cell">${j.attempt}/${j.max_attempts}</td>
        <td class="age-cell">${age(j.created_at)}</td>
        <td class="error-cell" title="${j.error || ''}">${j.error || '—'}</td>
      </tr>
    `).join('');
  }

  function renderDead(jobs) {
    const tbody = document.getElementById('jobs-body');
    if (!jobs.length) {
      tbody.innerHTML = '<tr><td colspan="7" class="empty-state">dead letter queue is empty</td></tr>';
      return;
    }
    tbody.innerHTML = jobs.map(j => `
      <tr>
        <td class="id-cell" title="${j.job_id}">${shortId(j.job_id)}</td>
        <td><span class="type-badge ${j.job_type}">${j.job_type}</span></td>
        <td>${priority(j.priority)}</td>
        <td class="attempt-cell">${j.total_attempts}</td>
        <td class="age-cell">${age(j.died_at)}</td>
        <td class="error-cell" title="${j.last_error || ''}">${j.last_error || '—'}</td>
        <td><button class="requeue-btn" onclick="requeue('${j.job_id}')">requeue</button></td>
      </tr>
    `).join('');
  }

  fetchData();
  setInterval(fetchData, 2000);
</script>
</body>
</html>"#;