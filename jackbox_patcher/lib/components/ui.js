// Petits composants partagés (équivalent de lib/components/*).
import { t } from '../services/l10n.js';
export const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
export const $ = (sel, root = document) => root.querySelector(sel);
export const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

const P = {
  play: '<path d="M7 4v16l13-8z"/>', download: '<path d="M12 3v12m0 0l-5-5m5 5l5-5M4 21h16"/>', settings: '<circle cx="12" cy="12" r="3"/><path d="M12 2v3m0 14v3M2 12h3m14 0h3M5 5l2 2m10 10l2 2M19 5l-2 2M7 17l-2 2"/>',
  back: '<path d="M15 5l-7 7 7 7"/>', home: '<path d="M3 11l9-8 9 8M5 10v10h5v-6h4v6h5V10"/>', box: '<path d="M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10"/>',
  server: '<rect x="3" y="4" width="18" height="7" rx="1"/><rect x="3" y="13" width="18" height="7" rx="1"/><path d="M7 8h.01M7 17h.01"/>', info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10h.01"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>', check: '<path d="M5 12l5 5 9-10"/>', alert: '<path d="M12 3l10 18H2zM12 10v5m0 3h.01"/>', x: '<path d="M6 6l12 12M18 6L6 18"/>',
  plus: '<path d="M12 5v14M5 12h14"/>', folder: '<path d="M3 6h6l2 2h10v11H3z"/>', refresh: '<path d="M20 11a8 8 0 10-2 6m2-12v6h-6"/>', trash: '<path d="M4 7h16M9 7V4h6v3m-8 0l1 13h8l1-13"/>',
  link: '<path d="M10 14a4 4 0 005.7 0l3-3a4 4 0 00-5.7-5.7l-1 1M14 10a4 4 0 00-5.7 0l-3 3a4 4 0 005.7 5.7l1-1"/>', globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"/>',
  users: '<circle cx="9" cy="8" r="3"/><path d="M3 20c0-4 3-6 6-6s6 2 6 6M16 5a3 3 0 010 6M18 14c2 1 3 3 3 6"/>', clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  tower: '<path d="M12 12v9M8 21h8M9 9a4 4 0 016 0M6 6a8 8 0 0112 0M12 12h.01"/>', wifi: '<path d="M2 9a15 15 0 0120 0M5 13a10 10 0 0114 0M8.5 16.5a5 5 0 017 0M12 20h.01"/>',
  network: '<circle cx="12" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="19" r="2"/><path d="M12 7v5M12 12l-6 5M12 12l6 5"/>', star: '<path d="M12 3l3 6 6 1-4.5 4.5 1 6.5-5.5-3-5.5 3 1-6.5L3 10l6-1z"/>',
  external: '<path d="M14 4h6v6M20 4l-9 9M18 14v6H4V6h6"/>', tag: '<path d="M3 12V3h9l9 9-9 9z"/><path d="M7.5 7.5h.01"/>',
};
export const icon = (n, size = '1em', cls = '') => `<svg class="i ${cls}" style="font-size:${size}" viewBox="0 0 24 24">${P[n] || ''}</svg>`;

/** Anneau de progression (ProgressRing) : pct 0..100, indéterminé si pct < 0. */
export function ring(pct, color = '#fff', size = 64) {
  const r = (size - 6) / 2, c = 2 * Math.PI * r;
  return `<svg class="ring ${pct < 0 ? 'spin' : ''}" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}"><circle cx="${size / 2}" cy="${size / 2}" r="${r}" fill="none" stroke="#ffffff22" stroke-width="3"/>` +
    `<circle cx="${size / 2}" cy="${size / 2}" r="${r}" fill="none" stroke="${color}" stroke-width="3" stroke-linecap="round" stroke-dasharray="${c}" stroke-dashoffset="${pct < 0 ? c * .75 : c * (1 - pct / 100)}"/></svg>`;
}

export function toast(msg, kind = '') {
  const el = document.createElement('div'); el.className = 'toast ' + kind; el.textContent = msg;
  document.getElementById('toasts').append(el); setTimeout(() => el.remove(), kind === 'err' ? 9000 : 4500);
}
/** Appelle une action async avec retour utilisateur (toast succès / erreur). */
export async function run(promise, okMsg) {
  try { const r = await promise; const m = okMsg === undefined ? (typeof r === 'string' ? r : '') : okMsg; if (m) toast(m, 'ok'); return r; }
  catch (e) { toast(String(e), 'err'); throw e; }
}

/** Dialogue modal. buttons: [{label, kind, onClick}] ; retourne {close, el}. */
export function dialog({ title, html = '', buttons = [{ label: t('close') }], dismissable = true }) {
  const ov = document.createElement('div'); ov.className = 'overlay';
  ov.innerHTML = `<div class="dialog" role="dialog"><h2>${esc(title)}</h2><div class="body">${html}</div><div class="actions"></div></div>`;
  const close = () => ov.remove(), actions = $('.actions', ov);
  buttons.forEach(b => { const e = document.createElement('button'); e.className = 'btn ' + (b.kind || ''); e.textContent = b.label;
    e.onclick = async () => { if (b.onClick) { const r = await b.onClick(ctl); if (r === false) return; } close(); }; actions.append(e); });
  if (dismissable) ov.addEventListener('mousedown', ev => { if (ev.target === ov) close(); });
  const ctl = { close, el: ov, body: $('.body', ov), actions }; document.body.append(ov); return ctl;
}
export const confirmDialog = (title, text) => new Promise(res => dialog({ title, html: `<p>${esc(text)}</p>`, dismissable: false,
  buttons: [{ label: t('cancel'), onClick: () => res(false) }, { label: t('confirm'), kind: 'accent', onClick: () => res(true) }] }));

/** Markdown minimal (titres, gras, italique, listes, liens https) — le texte est échappé avant transformation. */
export function md(src) {
  const lines = esc(src).split(/\r?\n/); let out = '', list = false;
  for (let l of lines) {
    const li = l.match(/^\s*[-*]\s+(.*)/);
    if (li && !list) { out += '<ul>'; list = true; } if (!li && list) { out += '</ul>'; list = false; }
    l = (li ? li[1] : l).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/\*(.+?)\*/g, '<i>$1</i>').replace(/\[(.+?)\]\((https:\/\/[^)\s]+)\)/g, '<a data-href="$2">$1</a>');
    const h = l.match(/^(#{1,3})\s+(.*)/);
    out += li ? `<li>${l}</li>` : h ? `<h${h[1].length}>${h[2]}</h${h[1].length}>` : l.trim() ? `<p>${l}</p>` : '';
  }
  return out + (list ? '</ul>' : '');
}
