'use strict';

// Presentation only. No audio/device API, network access, IPC or persistence.
const root = document.getElementById('linx');
const stateControl = document.getElementById('preview-state');
const apps = [
  { id: 'game', name: 'Game', symbol: 'G', detail: 'Game audio', channel: 1, volume: 85, muted: false, level: 71 },
  { id: 'discord', name: 'Discord', symbol: 'D', detail: 'Voice chat', channel: 2, volume: 100, muted: false, level: 48 },
  { id: 'firefox', name: 'Firefox', symbol: 'F', detail: 'Browser audio', channel: 3, volume: 65, muted: false, level: 61 },
];
const stateLabels = { connected: 'Connected', waiting: 'Waiting for Studio', connecting: 'Connecting', paused: 'Paused', attention: 'Needs attention', private: 'Private test' };
let state = 'connected';
let resumeState = 'connected';
let view = 'overview';
let channelFilter = null;
const channelName = n => n === 1 ? 'Link In' : `Link ${n} In`;
const active = () => state === 'connected' || state === 'private';
const icon = name => `<svg class="icon" aria-hidden="true"><use href="#${name}"/></svg>`;
const announce = message => { document.getElementById('announcement').textContent = message; };

function renderChannels() {
  document.getElementById('channels').innerHTML = [1, 2, 3, 4].map(n => {
    const assigned = active() ? apps.filter(a => a.channel === n) : [];
    const name = assigned.length ? assigned.map(a => a.name).join(', ') : (active() ? 'Unassigned' : '—');
    const level = Math.max(0, ...assigned.map(a => a.muted ? 0 : a.level * a.volume / 100));
    return `<button class="channel" data-channel="${n}" aria-pressed="${channelFilter === n}" aria-label="Filter applications for ${channelName(n)}"><span class="channel-heading"><b>0${n}</b> ${channelName(n)}</span><span class="channel-name${assigned.length ? '' : ' empty'}">${name}</span><span class="channel-subtitle">${assigned.length ? `${assigned.length} application` : (active() ? 'Ready for an app' : 'No active assignment')}</span><span class="channel-meter" aria-label="Illustrative stereo level, not live audio"><span class="meter-track"><span class="meter-fill" style="display:block;width:${level}%"></span></span><span class="meter-track"><span class="meter-fill" style="display:block;width:${Math.max(0, level - 6)}%"></span></span><small>Demo level</small></span></button>`;
  }).join('');
  document.querySelectorAll('[data-channel]').forEach(button => button.addEventListener('click', () => {
    const n = Number(button.dataset.channel);
    channelFilter = channelFilter === n ? null : n;
    renderChannels(); renderApps();
    document.querySelector(`[data-channel="${n}"]`).focus();
    announce(channelFilter ? `Showing ${channelName(n)} applications` : 'Showing all applications');
  }));
}

function renderApps() {
  const shown = channelFilter === null ? apps : (active() ? apps.filter(a => a.channel === channelFilter) : []);
  document.getElementById('clear-filter').hidden = channelFilter === null;
  document.getElementById('empty-channel').hidden = shown.length !== 0;
  document.getElementById('apps-title').innerHTML = `${channelFilter ? channelName(channelFilter) : 'Shared applications'} <span>${shown.length}</span>`;
  document.getElementById('app-list').innerHTML = shown.map(a => `<div class="app-row${a.muted ? ' is-muted' : ''}" data-app="${a.id}"><span class="app-symbol" aria-hidden="true">${a.symbol}</span><div><span class="app-name">${a.name}</span><span class="app-subtitle">${a.detail}</span></div><span class="route">${icon('arrow')}${active() ? channelName(a.channel) : 'Not linked'}</span><div class="app-controls"><label class="app-volume"><span class="sr-only">${a.name} Linux send volume, percent</span><input type="range" min="0" max="100" value="${a.volume}" data-volume="${a.id}" style="--fill:${a.volume}%"${active() ? '' : ' disabled'}><output for="volume-${a.id}">${a.volume}%</output></label><button class="mute-button" data-mute="${a.id}" aria-label="${a.name} mute" aria-pressed="${a.muted}"${active() ? '' : ' disabled'}>${icon(a.muted ? 'muted' : 'speaker')}</button></div></div>`).join('');
  document.querySelectorAll('[data-volume]').forEach(input => {
    input.id = `volume-${input.dataset.volume}`;
    input.addEventListener('input', () => {
      const app = apps.find(a => a.id === input.dataset.volume);
      app.volume = Math.max(0, Math.min(100, Number(input.value)));
      input.style.setProperty('--fill', `${app.volume}%`);
      input.nextElementSibling.value = `${app.volume}%`;
      renderChannels();
    });
    input.addEventListener('change', () => announce(`${apps.find(a => a.id === input.dataset.volume).name} demo volume ${input.value} percent`));
  });
  document.querySelectorAll('[data-mute]').forEach(button => button.addEventListener('click', () => {
    const app = apps.find(a => a.id === button.dataset.mute);
    app.muted = !app.muted;
    renderChannels(); renderApps();
    document.querySelector(`[data-mute="${app.id}"]`).focus();
    announce(`${app.name} ${app.muted ? 'muted' : 'unmuted'} in preview`);
  }));
}

function renderState() {
  root.dataset.state = state;
  stateControl.value = state;
  document.getElementById('status-text').textContent = stateLabels[state];
  document.getElementById('connection-description').textContent = ({ connected: 'Three apps shared from this PC', private: 'Isolated session · no physical output', waiting: 'LinX will wait here for your device', connecting: 'Establishing the link', paused: 'Automatic connection is held', attention: 'Session stopped · connection needs review' })[state];
  document.getElementById('footer-text').textContent = active() ? (state === 'private' ? 'Private test session' : '3 apps shared') : stateLabels[state];
  document.getElementById('apps-caption').textContent = state === 'private' ? 'Assignments from the synthetic peer · no hardware.' : active() ? 'Assignments received from the streaming PC.' : 'Selected apps are not currently linked.';
  document.getElementById('attention-message').hidden = state !== 'attention';
  const session = document.getElementById('session');
  session.disabled = state === 'connecting' || state === 'attention';
  session.innerHTML = icon(active() ? 'pause' : 'play') + `<span>${active() ? 'Pause' : 'Connect'}</span>`;
  renderChannels(); renderApps();
}

function setView(next) {
  view = next;
  root.dataset.view = view;
  for (const n of ['overview', 'mixer']) {
    const tab = document.getElementById(`${n}-tab`);
    tab.setAttribute('aria-selected', String(n === view));
    tab.tabIndex = n === view ? 0 : -1;
    tab.classList.toggle('active', n === view);
  }
  document.getElementById('workspace').setAttribute('aria-labelledby', `${view}-tab`);
}

stateControl.addEventListener('change', () => { state = stateControl.value; renderState(); announce(`Preview: ${stateLabels[state]}`); });
document.getElementById('session').addEventListener('click', () => {
  if (active()) { resumeState = state; state = 'paused'; }
  else { state = resumeState; }
  renderState(); announce(`Preview: ${stateLabels[state]}`);
});
document.getElementById('settings-toggle').addEventListener('click', event => {
  const panel = document.getElementById('settings-panel');
  panel.hidden = !panel.hidden;
  event.currentTarget.setAttribute('aria-expanded', String(!panel.hidden));
});
document.getElementById('auto-connect').addEventListener('change', event => announce(`Demo automatic connection ${event.target.checked ? 'on' : 'off'}. No preference saved.`));
document.getElementById('clear-filter').addEventListener('click', () => { channelFilter = null; renderChannels(); renderApps(); document.getElementById(`${view}-tab`).focus(); });
for (const n of ['overview', 'mixer']) {
  document.getElementById(`${n}-tab`).addEventListener('click', () => setView(n));
  document.getElementById(`${n}-tab`).addEventListener('keydown', event => {
    if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      const next = event.key === 'Home' ? 'overview' : event.key === 'End' ? 'mixer' : (n === 'overview' ? 'mixer' : 'overview');
      setView(next); document.getElementById(`${next}-tab`).focus();
    }
  });
}
setView('overview');
renderState();
