'use strict';

// Design scenarios only. No audio, USB, IPC, requests, timers or storage.
const channels = [
  { id: 1, title: 'Game', app: 'Game', letter: 'G', color: '#a6d5bd', gain: -6, muted: false, level: 76 },
  { id: 2, title: 'Voice', app: 'Discord', letter: 'D', color: '#a6bfe7', gain: -3, muted: false, level: 64 },
  { id: 3, title: 'Music', app: 'Spotify', letter: 'S', color: '#c1a9d8', gain: -12, muted: false, level: 83 },
  { id: 4, title: 'Browser', app: 'Firefox', letter: 'F', color: '#d9b68b', gain: -9, muted: false, level: 69 },
];
const descriptions = {
  console: 'Default: a compact mixing surface with each channel’s fader, meter and applications together.',
  matrix: 'An alternative signal map: source rows, destination columns, one focused gain control.',
  mini: 'A small desktop utility: four immediate controls, with the full console one click away.',
};
const labels = { connected: 'Studio connected', waiting: 'Waiting for Studio', connecting: 'Connecting', paused: 'Paused', attention: 'Needs attention', private: 'Private test' };
const instrument = document.getElementById('instrument');
const parameters = new URLSearchParams(location.search);
let direction = parameters.get('view') || 'console';
if (!Object.hasOwn(descriptions, direction)) direction = 'console';
let theme = parameters.get('theme') === 'light' ? 'light' : 'dark';
const palettes = {
  mint: { dark: '#a6d5bd', light: '#31735d', channels: ['#a6d5bd', '#a6bfe7', '#c1a9d8', '#d9b68b'], lightChannels: ['#39765f', '#426ca1', '#795794', '#946831'] },
  blue: { dark: '#9dc3ef', light: '#345f9a', channels: ['#9dc3ef', '#8bc1da', '#abb9e8', '#95d2cc'], lightChannels: ['#345f9a', '#32728b', '#5c63a4', '#35776d'] },
  violet: { dark: '#bea9e3', light: '#7551a5', channels: ['#bea9e3', '#b1b8e8', '#d1afd6', '#a9bfd5'], lightChannels: ['#7551a5', '#62669f', '#985b9d', '#4e7495'] },
  amber: { dark: '#ddb98f', light: '#8e612a', channels: ['#ddb98f', '#d8ca9c', '#cca89b', '#c2be9c'], lightChannels: ['#8e612a', '#80702d', '#95614d', '#727033'] },
  rose: { dark: '#dca8b6', light: '#9b4b67', channels: ['#dca8b6', '#c2a9ce', '#d3b9bd', '#d1b3a0'], lightChannels: ['#9b4b67', '#815a94', '#956573', '#8c6549'] },
};
let accent = Object.hasOwn(palettes, parameters.get('accent')) ? parameters.get('accent') : 'mint';
let state = 'connected';
let resumeState = 'connected';
let selected = 1;
let settingsOpen = false;
let autoConnect = true;
const active = () => state === 'connected' || state === 'private';
const route = id => id === 1 ? 'Link In' : `Link ${id} In`;
const gainText = gain => (gain < 0 ? '−' : '') + Math.abs(gain).toFixed(1);
const icon = id => `<svg class="icon" aria-hidden="true"><use href="#${id}"/></svg>`;
const announce = text => { document.getElementById('announcement').textContent = text; };

function sessionHeader(extra = '') {
  return `<header class="instrument-header"><div class="wordmark"><svg class="lynx" aria-hidden="true"><use href="#lynx"/></svg><strong>LinX</strong><span>${extra || 'STUDIO'}</span></div><div class="instrument-header-right"><span class="session-state"><i></i>${labels[state]}</span><button class="tool-button" data-session aria-label="${active() ? 'Pause session' : 'Connect session'}"${state === 'connecting' || state === 'attention' ? ' disabled' : ''}>${icon(active() ? 'pause' : 'play')}<span class="button-label">${active() ? 'Pause' : 'Connect'}</span></button>${direction === 'mini' ? '' : `<button class="tool-button icon-button" data-settings aria-label="Connection settings" aria-expanded="${settingsOpen}">${icon('sliders')}</button>`}</div></header>`;
}
function notice() {
  if (active()) return '';
  return `<div class="session-notice">${({waiting: 'Leave LinX open. It will wait for Studio.', connecting: 'Establishing the link. Controls are temporarily unavailable.', paused: 'Session paused. Automatic connection is held until you reconnect.', attention: 'Session stopped. Review the connection before reconnecting.'})[state]}</div>`;
}
function settings() {
  if (!settingsOpen) return '';
  return `<div class="connection-settings"><label>Connect automatically while LinX is open<input id="auto-connect" type="checkbox" role="switch"${autoConnect ? ' checked' : ''}></label></div>`;
}
function footer(label, compact = true) {
  return `<footer class="instrument-footer"><span class="signal-path">${label}</span><button class="small-button" data-switch="${compact ? 'mini' : 'console'}">${icon('expand')}${compact ? 'Mini view' : 'Open console'}</button></footer>`;
}
function mutedButton(c, mini = false) {
  return `<button class="${mini ? 'mini-mute' : 'strip-mute'}" data-mute="${c.id}" aria-label="${c.app} send mute" aria-pressed="${c.muted}"${active() ? '' : ' disabled'}>${icon(c.muted ? 'mute' : 'speaker')}${mini ? '' : `<span>${c.muted ? 'Muted' : 'Mute'}</span>`}</button>`;
}
function gainInput(c, vertical = false) {
  return `<input class="${vertical ? 'vertical-fader' : 'horizontal-fader'}" type="range" min="-60" max="0" step="1" value="${c.gain}" data-gain="${c.id}" aria-label="${c.app} Linux send gain in decibels" style="--fill:${(c.gain + 60) / 60 * 100}%"${active() ? '' : ' disabled'}>`;
}
function consoleView() {
  return `${sessionHeader()}<div class="console-subheader"><strong>LINUX SENDS</strong><span>4 stereo channels · demo levels</span></div>${notice()}<div class="channel-bank">${channels.map(c => {
    const level = active() && !c.muted ? Math.max(0, c.level + c.gain * 1.2) : 0;
    return `<section class="strip${c.muted ? ' is-muted' : ''}" data-channel="${c.id}" style="--channel:${c.color}"><div class="strip-top"><strong>${c.title}</strong><span class="channel-index">0${c.id}</span></div><span class="strip-route">${active() ? route(c.id) : 'Not linked'}</span><div class="fader-block"><div class="fader-scale" aria-hidden="true"><span>0</span><span>−12</span><span>−24</span><span>−36</span><span>−48</span><span>−60</span></div><div class="fader">${gainInput(c, true)}</div><div class="stereo-meter" aria-label="Illustrative stereo level"><span class="meter-rail"><i style="--level:${level}%"></i></span><span class="meter-rail"><i style="--level:${Math.max(0, level - 7)}%"></i></span></div></div><div class="gain-value"><output data-readout="${c.id}">${gainText(c.gain)}</output><span>dB</span></div>${mutedButton(c)}<div class="source-well"><span>APPLICATION</span><div class="source-chip"><b aria-hidden="true">${c.letter}</b>${c.app}</div></div></section>`;
  }).join('')}</div>${footer(`THIS PC ${icon('arrow')} STUDIO ${icon('arrow')} STREAMING PC`)}${settings()}`;
}
function inspector() {
  const c = channels.find(c => c.id === selected);
  return `<div class="inspector-heading"><span class="overline">LINUX SEND</span><h3>${c.app}</h3><span class="source-route">${active() ? `→ ${route(c.id)}` : 'Not linked'}</span></div><div class="inspector-gain"><output data-readout="${c.id}">${gainText(c.gain)}</output><span>dB</span></div><div class="inspector-controls">${gainInput(c)}${mutedButton(c)}</div><p>Changes this Linux source before Studio. Every downstream mix receiving it is affected.</p>`;
}
function matrixView() {
  return `${sessionHeader('SIGNAL MAP')}<div class="map-title"><h2>Signal map</h2><span>04 sources / 04 destinations</span></div>${notice()}<div class="map-body"><div><table class="routing-table" aria-label="Illustrative assignments received from the streaming PC"><thead><tr><th scope="col">SOURCE</th>${channels.map(c => `<th scope="col">${route(c.id)}</th>`).join('')}</tr></thead><tbody>${channels.map(c => `<tr><th scope="row"><button class="source-button" data-select="${c.id}" aria-pressed="${c.id === selected}" aria-label="Inspect ${c.app}"><span class="app-glyph" aria-hidden="true">${c.letter}</span><span><strong>${c.app}</strong><small data-row-status="${c.id}">${c.muted ? 'Muted' : `${gainText(c.gain)} dB`}</small></span></button></th>${channels.map(dest => `<td class="route-cell">${active() && dest.id === c.id ? `<button class="route-point" data-select="${c.id}" aria-label="Inspect ${c.app} assigned to ${route(c.id)}" aria-pressed="${c.id === selected}">${icon('check')}</button>` : '<span aria-label="No observed assignment">·</span>'}</td>`).join('')}</tr>`).join('')}</tbody></table><span class="table-note"><i></i>${state === 'private' ? 'Synthetic peer assignments' : 'Assignments received from streaming PC'}</span></div><aside class="source-inspector" aria-label="Selected source controls">${inspector()}</aside></div>${footer(`LINUX SOURCE ${icon('arrow')} STUDIO LINK`)}${settings()}`;
}
function miniView() {
  return `${sessionHeader()}${notice()}<div class="mini-rows">${channels.map(c => `<div class="mini-row" style="--channel:${c.color}"><span class="app-glyph" aria-hidden="true">${c.letter}</span><div class="mini-row-main"><div class="mini-row-title"><strong>${c.app}</strong><span>${active() ? route(c.id) : 'Not linked'}</span></div>${gainInput(c)}</div><output data-readout="${c.id}" aria-label="${c.app} demo gain in decibels">${gainText(c.gain)}</output>${mutedButton(c, true)}</div>`).join('')}</div>${footer('LINUX SENDS · dB · DEMO', false)}`;
}
function bindControls() {
  instrument.querySelectorAll('[data-gain]').forEach(input => {
    input.addEventListener('input', () => {
      const c = channels.find(c => c.id === Number(input.dataset.gain));
      c.gain = Math.max(-60, Math.min(0, Number(input.value)));
      input.style.setProperty('--fill', `${(c.gain + 60) / 60 * 100}%`);
      instrument.querySelectorAll(`[data-readout="${c.id}"]`).forEach(o => { o.value = gainText(c.gain); });
      const row = instrument.querySelector(`[data-row-status="${c.id}"]`);
      if (row) row.textContent = c.muted ? 'Muted' : `${gainText(c.gain)} dB`;
      const rails = instrument.querySelectorAll(`[data-channel="${c.id}"] .meter-rail i`);
      const level = active() && !c.muted ? Math.max(0, c.level + c.gain * 1.2) : 0;
      rails.forEach((r, n) => r.style.setProperty('--level', `${Math.max(0, level - n * 7)}%`));
    });
    input.addEventListener('change', () => announce(`Demo gain ${input.value} decibels`));
  });
  instrument.querySelectorAll('[data-mute]').forEach(button => button.addEventListener('click', () => {
    const id = Number(button.dataset.mute);
    const c = channels.find(c => c.id === id); c.muted = !c.muted;
    render(); instrument.querySelector(`[data-mute="${id}"]`).focus();
    announce(`${c.app} ${c.muted ? 'muted' : 'unmuted'} in demo`);
  }));
  instrument.querySelectorAll('[data-select]').forEach(button => button.addEventListener('click', () => {
    selected = Number(button.dataset.select);
    const label = button.getAttribute('aria-label');
    render(); [...instrument.querySelectorAll('[data-select]')].find(b => b.getAttribute('aria-label') === label).focus();
    announce(`Inspecting ${channels.find(c => c.id === selected).app}. No assignment changed.`);
  }));
  instrument.querySelectorAll('[data-switch]').forEach(button => button.addEventListener('click', () => {
    direction = button.dataset.switch; render(); document.querySelector(`[data-direction="${direction}"]`).focus();
  }));
  instrument.querySelector('[data-session]').addEventListener('click', () => {
    if (active()) { resumeState = state; state = 'paused'; } else { state = resumeState; }
    render(); instrument.querySelector('[data-session]').focus(); announce(`Demo ${labels[state]}`);
  });
  instrument.querySelector('[data-settings]')?.addEventListener('click', () => {
    settingsOpen = !settingsOpen; render(); instrument.querySelector('[data-settings]').focus();
  });
  instrument.querySelector('#auto-connect')?.addEventListener('change', event => { autoConnect = event.target.checked; });
}
function render() {
  const palette = palettes[accent];
  channels.forEach((c, n) => { c.color = (theme === 'light' ? palette.lightChannels : palette.channels)[n]; });
  instrument.dataset.theme = theme;
  instrument.style.setProperty('--accent', palette[theme]);
  instrument.style.setProperty('--accent-dark', palette.dark);
  instrument.style.setProperty('--accent-contrast', theme === 'light' ? '#ffffff' : '#182422');
  document.querySelectorAll('.theme-buttons [data-theme]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.theme === theme)));
  document.getElementById('accent').value = accent;
  document.querySelectorAll('.direction-picker [data-direction]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.direction === direction)));
  document.getElementById('preview-stage').dataset.direction = direction;
  document.getElementById('direction-description').textContent = descriptions[direction];
  instrument.className = `instrument ${direction}`;
  instrument.dataset.state = state;
  document.getElementById('demo-state').value = state;
  instrument.innerHTML = ({ console: consoleView, matrix: matrixView, mini: miniView })[direction]();
  bindControls();
}
document.querySelectorAll('.direction-picker [data-direction]').forEach(button => button.addEventListener('click', () => {
  direction = button.dataset.direction; render(); announce(`Showing ${direction} direction`);
}));
document.getElementById('demo-state').addEventListener('change', event => { state = event.target.value; render(); announce(`Preview ${labels[state]}`); });
document.querySelectorAll('.theme-buttons [data-theme]').forEach(button => button.addEventListener('click', () => {
  theme = button.dataset.theme; render(); announce(`${theme} app theme`);
}));
document.getElementById('accent').addEventListener('change', event => { accent = event.target.value; render(); announce(`${accent} accent palette`); });
render();
