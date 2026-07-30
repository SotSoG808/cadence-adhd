import { invoke } from '@tauri-apps/api/core';

// -------------------------------------------------------------------
// Toast helper
// -------------------------------------------------------------------
const toastEl = Object.assign(document.createElement('div'), { id: 'toast' });
document.body.appendChild(toastEl);
function toast(msg, ms = 2400) {
  toastEl.textContent = msg;
  toastEl.classList.add('show');
  setTimeout(() => toastEl.classList.remove('show'), ms);
}

// -------------------------------------------------------------------
// Navigation
// -------------------------------------------------------------------
let currentScreen = 'today';
const appEl = document.querySelector('#app');

document.querySelectorAll('.nav-btn').forEach((btn) => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.nav-btn').forEach((b) => b.classList.remove('active'));
    btn.classList.add('active');
    currentScreen = btn.dataset.screen;
    renderScreen(currentScreen);
  });
});

// -------------------------------------------------------------------
// Focus mode
// -------------------------------------------------------------------
const modeSelect = document.getElementById('mode-select');
modeSelect.addEventListener('change', async () => {
  await invoke('set_setting', { key: 'focus_mode', value: modeSelect.value });
  toast(`Mode: ${modeSelect.value}`);
  renderScreen(currentScreen);
});

async function loadMode() {
  try {
    const settings = await invoke('get_settings');
    const entry = settings.find(([k]) => k === 'focus_mode');
    if (entry) modeSelect.value = entry[1];
  } catch (_) {}
}

// -------------------------------------------------------------------
// Consent gate
// -------------------------------------------------------------------
async function ensureConsent() {
  const settings = await invoke('get_settings').catch(() => []);
  const given = settings.find(([k]) => k === 'consent_given')?.[1];
  if (given === '1') return;
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.id = 'consent-overlay';
    overlay.innerHTML = `
      <div id="consent-box">
        <h2>Welcome to Cadence</h2>
        <p>Cadence stores your routine tasks and calendar data <strong>locally on this device only</strong>.
        Data is encrypted at rest. If you enable phone push, reminder text is sent via ntfy.sh.
        This application is for personal use only and is not a medical device.</p>
        <p>By continuing you confirm you understand and agree to these terms.</p>
        <button class="btn btn-primary" id="consent-accept">I understand &mdash; let&rsquo;s go</button>
      </div>`;
    document.body.appendChild(overlay);
    document.getElementById('consent-accept').addEventListener('click', async () => {
      await invoke('set_setting', { key: 'consent_given', value: '1' });
      overlay.remove();
      resolve();
    });
  });
}

// -------------------------------------------------------------------
// TODAY screen
// -------------------------------------------------------------------
async function renderToday() {
  const now = new Date();
  const hour = now.getHours();
  const greeting =
    hour < 12 ? 'Good morning' : hour < 17 ? 'Good afternoon' : 'Good evening';

  let tasks = [];
  let insights = { today_pts: 0, goal_pts: 30, streak: 0, level: 1, on_time_pct: 0 };
  try {
    tasks = await invoke('get_tasks');
    insights = await invoke('get_insights');
  } catch (e) {
    console.error(e);
  }

  const pending = tasks.filter((t) => t.status === 'Pending');
  const done = tasks.filter((t) => t.status === 'Done');
  const nowTasks = pending.filter((t) => {
    if (!t.scheduled_at) return false;
    const d = new Date(t.scheduled_at);
    return d <= now;
  });
  const upNext = pending.filter((t) => {
    if (!t.scheduled_at) return true;
    return new Date(t.scheduled_at) > now;
  });

  const pct = Math.min(
    100,
    Math.round((insights.today_pts / insights.goal_pts) * 100)
  );

  const taskCard = (t) => `
    <div class="card row" data-id="${t.id}">
      <div class="col">
        <strong>${esc(t.title)}</strong>
        <small style="color:var(--muted)">${esc(t.category)}
          ${t.scheduled_at ? '&middot; ' + fmtTime(t.scheduled_at) : ''}
          ${t.snoozed_until ? '<span class="pill">snoozed</span>' : ''}
          ${t.deferred_until ? '<span class="pill">deferred</span>' : ''}
        </small>
      </div>
      <div style="display:flex;gap:6px;flex-wrap:wrap">
        <button class="btn btn-primary btn-sm" onclick="completeTask('${t.id}')">Done &middot; ${t.points} pts</button>
        <button class="btn btn-ghost btn-sm" onclick="snoozeTask('${t.id}')">Snooze 10m</button>
        <button class="btn btn-ghost btn-sm" onclick="deferTask('${t.id}')">Defer</button>
      </div>
    </div>`;

  appEl.innerHTML = `
    <h2>${greeting} &#127774;</h2>
    <p style="color:var(--muted);margin-top:4px">
      Today &middot; <span class="pill">${modeSelect.value} mode</span>
    </p>

    <h3>Now</h3>
    ${nowTasks.length ? nowTasks.map(taskCard).join('') : '<p style="color:var(--muted)">Nothing due right now &#10003;</p>'}

    <h3>Up next</h3>
    ${upNext.length ? upNext.map(taskCard).join('') : '<p style="color:var(--muted)">All tasks accounted for</p>'}

    <h3>Progress</h3>
    <div class="card">
      <div class="row">
        <span><strong>${insights.today_pts}</strong> / ${insights.goal_pts} pts today</span>
        <span class="pill ${pct >= 100 ? 'green' : ''}">${pct}%</span>
      </div>
      <div class="progress-track"><div class="progress-fill" style="width:${pct}%"></div></div>
      <div style="color:var(--muted);font-size:13px;margin-top:8px">
        Streak: <strong>${insights.streak}</strong> days &middot;
        Level <strong>${insights.level}</strong>
      </div>
    </div>

    <h3>Done today &mdash; ${done.length}</h3>
    ${done.slice(-5).reverse().map((t) => `<div class="card row"><span>&#10003; ${esc(t.title)}</span><span class="pill green">+${t.points}</span></div>`).join('') || '<p style="color:var(--muted)">Nothing yet</p>'}
  `;
}

window.completeTask = async (id) => {
  try {
    const pts = await invoke('complete_task', { taskId: id, late: false });
    toast(`+${pts} points!`);
    renderToday();
  } catch (e) { toast('Error: ' + e, 4000); }
};

window.snoozeTask = async (id) => {
  try {
    await invoke('snooze_task', { taskId: id, minutes: 10 });
    toast('Snoozed 10 minutes');
    renderToday();
  } catch (e) { toast('Error: ' + e, 4000); }
};

window.deferTask = async (id) => {
  const tomorrow = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
  try {
    await invoke('defer_task', { taskId: id, untilDate: tomorrow });
    toast('Deferred to tomorrow');
    renderToday();
  } catch (e) { toast('Error: ' + e, 4000); }
};

// -------------------------------------------------------------------
// ROUTINE screen
// -------------------------------------------------------------------
async function renderRoutine() {
  appEl.innerHTML = `
    <h2>Routine tasks</h2>
    <p style="color:var(--muted)">Fixed times, flexible blocks, or chained after another task.</p>
    <div class="card">
      <div style="display:grid;gap:10px">
        <input type="text" id="rt-title" placeholder="Task name" />
        <select class="field" id="rt-category">
          ${['Medication','Meal','Care','Exercise','Work','Home','Other'].map((c) => `<option>${c}</option>`).join('')}
        </select>
        <select class="field" id="rt-schedule-type">
          <option value="fixed">Fixed time</option>
          <option value="flexible">Flexible block</option>
          <option value="chain">After another task</option>
        </select>
        <input type="time" id="rt-time" />
        <input type="text" id="rt-points" placeholder="Points (default 10)" style="width:160px" />
        <button class="btn btn-primary" id="rt-add-btn">Add task</button>
      </div>
    </div>
    <div id="routine-list"></div>`;

  document.getElementById('rt-add-btn').addEventListener('click', addRoutineTask);
  renderRoutineList();
}

async function renderRoutineList() {
  const listEl = document.getElementById('routine-list');
  if (!listEl) return;
  try {
    const tasks = await invoke('get_tasks');
    if (!tasks.length) { listEl.innerHTML = '<p style="color:var(--muted)">No tasks yet.</p>'; return; }
    listEl.innerHTML = `<h3>All tasks</h3>` + tasks.map((t) => `
      <div class="card row">
        <div class="col">
          <strong>${esc(t.title)}</strong>
          <small style="color:var(--muted)">${esc(t.category)} &middot; ${t.scheduled_at ? fmtTime(t.scheduled_at) : 'flexible'} &middot; ${t.points} pts</small>
        </div>
        <span class="pill ${t.status === 'Done' ? 'green' : ''}">${t.status}</span>
      </div>`).join('');
  } catch (e) { listEl.innerHTML = '<p>Error loading tasks.</p>'; }
}

async function addRoutineTask() {
  const title = document.getElementById('rt-title').value.trim();
  if (!title) { toast('Please enter a task name', 3000); return; }
  const category = document.getElementById('rt-category').value;
  const timeVal = document.getElementById('rt-time').value;
  const points = parseInt(document.getElementById('rt-points').value) || 10;
  const today = new Date().toISOString().slice(0, 10);
  const scheduled_at = timeVal ? `${today}T${timeVal}:00Z` : null;
  try {
    await invoke('add_task', {
      dto: {
        id: '',
        title,
        category,
        scheduled_at,
        due_by: null,
        scheduled_days: [],
        after_task: null,
        points,
        status: 'Pending',
        snoozed_until: null,
        deferred_until: null,
        quiet: false,
      },
    });
    toast('Task added');
    document.getElementById('rt-title').value = '';
    renderRoutineList();
  } catch (e) { toast('Error: ' + e, 4000); }
}

// -------------------------------------------------------------------
// INSIGHTS screen
// -------------------------------------------------------------------
async function renderInsights() {
  let ins = { today_pts: 0, goal_pts: 30, streak: 0, level: 1, on_time_pct: 0 };
  try { ins = await invoke('get_insights'); } catch (_) {}

  appEl.innerHTML = `
    <h2>Insights</h2>
    <div class="stat-grid">
      <div class="stat-card"><div class="stat-value">${ins.today_pts}</div><div class="stat-label">Points today / ${ins.goal_pts} goal</div></div>
      <div class="stat-card"><div class="stat-value">${ins.streak}</div><div class="stat-label">Day streak</div></div>
      <div class="stat-card"><div class="stat-value">${ins.level}</div><div class="stat-label">Level</div></div>
      <div class="stat-card"><div class="stat-value">${ins.on_time_pct.toFixed(0)}%</div><div class="stat-label">On-time (30 days)</div></div>
    </div>
    <div class="card" style="color:var(--muted);font-size:13px">Points &times; 250 per level. Late completions earn half points. Daily goal streak requires hitting the points goal each day.</div>`;
}

// -------------------------------------------------------------------
// CALENDARS screen
// -------------------------------------------------------------------
async function renderCalendars() {
  appEl.innerHTML = `
    <h2>Calendars</h2>
    <p style="color:var(--muted)">Import .ics files from Google Calendar, Outlook, or email invites. Re-import a file with the same name to refresh it.</p>
    <div class="card">
      <label for="ics-file" style="font-weight:600">Select .ics file</label><br><br>
      <input type="file" id="ics-file" accept=".ics" />
      <button class="btn btn-primary" style="margin-top:12px" id="ics-import-btn">Import calendar</button>
    </div>
    <h3>Event lead-time reminders</h3>
    <div class="card" style="color:var(--muted)">No calendars yet. Import an .ics file to see events here.</div>`;

  document.getElementById('ics-import-btn').addEventListener('click', () => {
    toast('Calendar import coming soon — file parsing wired in Rust engine', 4000);
  });
}

// -------------------------------------------------------------------
// SETTINGS screen
// -------------------------------------------------------------------
async function renderSettings() {
  let settings = [];
  try { settings = await invoke('get_settings'); } catch (_) {}
  const get = (k) => settings.find(([key]) => key === k)?.[1] ?? '';

  appEl.innerHTML = `
    <h2>Settings</h2>

    <div class="card">
      <h3 style="margin-top:0">Notifications</h3>
      <div class="row" style="margin-bottom:12px">
        <span>Desktop notifications</span>
        <span class="pill green">Enabled</span>
      </div>
    </div>

    <div class="card">
      <h3 style="margin-top:0">Phone &amp; watch push (ntfy)</h3>
      <p style="color:var(--muted);font-size:13px">Real pushes to your phone via the free ntfy app. A Garmin watch mirrors phone notifications automatically. Treat the topic like a password — anyone who knows it can read these reminders.</p>
      <label for="ntfy-topic">ntfy topic</label><br>
      <input type="text" id="ntfy-topic" value="${esc(get('ntfy_topic'))}" placeholder="long-random-string" style="margin:8px 0" />
      <div class="row" style="margin-top:8px">
        <label><input type="checkbox" id="ntfy-enabled" ${get('ntfy_enabled') === '1' ? 'checked' : ''} /> Enable ntfy push</label>
        <button class="btn btn-primary btn-sm" id="ntfy-save">Save</button>
      </div>
    </div>

    <div class="card">
      <h3 style="margin-top:0">Daily goal</h3>
      <label for="goal-pts">Points goal per day</label><br>
      <input type="text" id="goal-pts" value="${esc(get('goal_pts'))}" style="width:100px;margin:8px 0" />
      <button class="btn btn-ghost btn-sm" id="goal-save">Save</button>
    </div>

    <div class="card">
      <h3 style="margin-top:0">Data</h3>
      <p style="color:var(--muted);font-size:13px">Routine, calendars and history are stored locally on this device, encrypted at rest. No data is sent to any server except reminder text via ntfy.sh if you enable phone push.</p>
      <button class="btn btn-ghost btn-sm" id="check-engine">Check reminder engine</button>
      <p id="engine-status" style="font-size:13px;color:var(--muted)"></p>
    </div>`;

  document.getElementById('ntfy-save').addEventListener('click', async () => {
    const topic = document.getElementById('ntfy-topic').value.trim();
    const enabled = document.getElementById('ntfy-enabled').checked ? '1' : '0';
    await invoke('set_setting', { key: 'ntfy_topic', value: topic });
    await invoke('set_setting', { key: 'ntfy_enabled', value: enabled });
    toast('ntfy settings saved');
  });

  document.getElementById('goal-save').addEventListener('click', async () => {
    const val = document.getElementById('goal-pts').value.trim();
    if (!val || isNaN(val)) { toast('Enter a valid number', 3000); return; }
    await invoke('set_setting', { key: 'goal_pts', value: val });
    toast('Goal updated');
  });

  document.getElementById('check-engine').addEventListener('click', async () => {
    const status = await invoke('app_status');
    document.getElementById('engine-status').textContent = status;
  });
}

// -------------------------------------------------------------------
// Router
// -------------------------------------------------------------------
function renderScreen(name) {
  const map = {
    today:     renderToday,
    routine:   renderRoutine,
    insights:  renderInsights,
    calendars: renderCalendars,
    settings:  renderSettings,
  };
  (map[name] ?? renderToday)();
}

// -------------------------------------------------------------------
// Utilities
// -------------------------------------------------------------------
function esc(str) {
  return String(str ?? '').replace(/[&<>"']/g, (c) =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c])
  );
}

function fmtTime(iso) {
  try { return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }); }
  catch (_) { return iso; }
}

// -------------------------------------------------------------------
// Bootstrap
// -------------------------------------------------------------------
async function init() {
  await ensureConsent();
  await loadMode();
  renderScreen('today');
}

init();
