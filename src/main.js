import { invoke } from '@tauri-apps/api/core';

const tasks = [
  { name: 'Morning medication', cat: 'Medication', time: '09:00', pts: 10 },
  { name: 'Breakfast', cat: 'Meal', time: '09:15', pts: 10 },
  { name: 'Reply to one email', cat: 'Work', time: '10:00', pts: 5 },
];

let screen = 'today';
const app = document.querySelector('#app');

function render() {
  if (screen === 'today') {
    app.innerHTML = `
      <h2>Good afternoon</h2>
      <p>Your gentle plan for today. <span class="pill">Normal mode</span></p>
      <h3>Now</h3>
      ${tasks
        .map(
          (t, i) => `
            <div class="card row">
              <div><b>${t.name}</b><br><small>${t.cat} · ${t.time}</small></div>
              <button onclick="done(${i})">Done · ${t.pts} pts</button>
            </div>
          `,
        )
        .join('')}
      <h3>Up next</h3>
      <div class="card">Today: 0 / 30 points · Goal streak: 0 days</div>
      <button onclick="screen='routine';render()">Quick add</button>
    `;
  } else if (screen === 'routine') {
    app.innerHTML = `
      <h2>Routine tasks</h2>
      <p>Fixed times, flexible blocks, or chained after another task.</p>
      <div class="card">
        <input placeholder="Task name" />
        <select>
          <option>Fixed time</option>
          <option>Flexible block</option>
          <option>After another task</option>
        </select>
        <button>Add task</button>
      </div>
      <div class="card">
        Focus modes filter what nags you. <b>Essentials</b> keeps medication, meals and care alerting during hyperfocus.
      </div>
    `;
  } else if (screen === 'insights') {
    app.innerHTML = `
      <h2>Insights</h2>
      <div class="card">Today: 0 / 30 pts</div>
      <div class="card">Goal streak: 0 days · Best run: 0</div>
      <div class="card">Level 1 · On-time rate: —</div>
      <h3>Recent completions</h3>
      <div class="card">No completions yet.</div>
    `;
  } else if (screen === 'calendars') {
    app.innerHTML = `
      <h2>Calendars</h2>
      <p>Import .ics files from Google Calendar, Outlook, or email invites. Re-import a file with the same name to refresh it.</p>
      <div class="card"><input type="file" accept=".ics" /><button>Import calendar</button></div>
      <h3>Event lead-time reminders</h3>
      <div class="card">No calendars yet.</div>
    `;
  } else {
    app.innerHTML = `
      <h2>Settings</h2>
      <div class="card">Desktop notifications: enabled</div>
      <div class="card">Phone & watch push via ntfy. Treat the topic as a password; reminder text travels via ntfy.sh only when enabled.</div>
      <button onclick="status()">Check reminder engine</button>
      <p id="status"></p>
    `;
  }
}

window.done = (i) => {
  tasks.splice(i, 1);
  render();
};

window.status = async () => {
  document.querySelector('#status').textContent = await invoke('app_status');
};

document.querySelectorAll('[data-screen]').forEach((x) => {
  x.onclick = () => {
    screen = x.dataset.screen;
    render();
  };
});

render();
