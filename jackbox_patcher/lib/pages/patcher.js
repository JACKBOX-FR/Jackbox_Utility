// Patch des jeux (PatcherMenuWidget / pack_container / pack_patch / game_patch de l'appli d'origine).
import { store, assetUrl, isOwned, packState, patchStatus, versionOf, notify } from '../services/store.js';
import { inv, listen } from '../services/tauri.js';
import { t } from '../services/l10n.js';
import { esc, icon, $, $$, dialog, confirmDialog, md, toast } from '../components/ui.js';
import { ensureLoaded } from './main_container.js';

const ST = { none: ['patch_not_installed', 'off'], installed: ['patch_installed', 'ok'], outdated: ['patch_outdated', 'warn'], unavailable: ['patch_unavailable', 'off'] };
const TYPES = { TEXT: 'patch_modification_content_text', INTERNAL: 'patch_modification_content_internal', SUBTITLES: 'patch_modification_content_subtitles', WEBSITE: 'patch_modification_content_website', AUDIO: 'patch_modification_content_audios', AUDIOS: 'patch_modification_content_audios' };
const badge = s => `<span class="badge ${ST[s][1]}">${esc(t(ST[s][0], { count: 1 }))}</span>`;
const allPatches = p => [...(p.patchs || []).map(x => [x, null]), ...(p.games || []).flatMap(g => (g.patchs || []).map(x => [x, g.id]))];

function row(pack, patch, gameId, sub, gameName) {
  const st = patchStatus(pack.id, patch);
  const btn = st === 'none' || st === 'outdated'
    ? `<button class="btn small ${st === 'outdated' ? 'accent' : 'green'}" data-install>${icon('download')} ${esc(t(ST[st][0], { count: 1 }))}</button>` : '';
  return `<div class="card patch ${sub ? 'sub' : ''}" data-pack="${esc(pack.id)}" data-patch="${esc(patch.id)}" data-game="${esc(gameId || '')}">
    <div class="grow" data-info style="cursor:pointer"><div class="nm">${esc(patch.name)}</div>
    <div class="caption">${esc(gameName ? gameName + ' · ' : '')}${esc(patch.small_description || '')} ${versionOf(patch) ? '· ' + esc(t('version')) + ' ' + esc(versionOf(patch)) : ''}</div></div>
    ${btn || badge(st)}</div>`;  // le bouton suffit quand une action est possible
}

function packSection(pack, withHeader) {
  const rows = [];
  for (const [x, g] of allPatches(pack)) {
    const gm = g ? pack.games.find(y => y.id === g) : null;
    rows.push(row(pack, x, g, false, gm?.name));
    (x.components || []).forEach(c => rows.push(`<div class="card patch sub"><div class="grow"><div class="nm">${esc(c.name)}</div><div class="caption">${esc(c.small_description || '')}</div></div></div>`));
  }
  if (!rows.length) return '';
  return (withHeader ? `<div class="group" data-goto="${esc(pack.id)}" style="cursor:pointer">${pack.icon ? `<img src="${esc(assetUrl(pack.icon))}" width="22" height="22" alt="">` : icon('box')} ${esc(pack.name)}</div>` : '') + rows.join('');
}

export async function render(root, [sel = 'all']) {
  if (!(await ensureLoaded())) return;
  const packs = store.cat.packs.filter(p => store.showAllPacks || isOwned(p.id));
  const cur = packs.find(p => p.id === sel);
  let main;
  if (cur) {
    const st = packState(cur.id);
    main = `<div class="hero" style="background-image:url('${esc(assetUrl(cur.background))}')"><div class="in">${cur.icon ? `<img src="${esc(assetUrl(cur.icon))}" alt="">` : ''}<div><div class="title">${esc(cur.name)}</div><div class="caption">${esc(cur.description || '')}</div></div></div></div>` +
      (!st.path ? `<div class="card pad" style="margin-bottom:14px">${icon('alert')} <b>${esc(t('path_inexistant'))}</b><div class="muted">${esc(t('path_inexistant_description'))}</div><div style="margin-top:10px"><button class="btn small accent" data-settings>${icon('settings')} ${esc(t('settings'))}</button></div></div>` : '') +
      (packSection(cur, false) || `<p class="muted">${esc(t('game_patch_unavailable'))}</p>`);
  } else {
    const secs = packs.map(p => packSection(p, true)).join('');
    main = `<div class="title" style="margin-bottom:12px">${esc(t('all_patches'))}</div>` + (secs || (packs.length ? `<p class="muted">${esc(t('game_patch_unavailable'))}</p>` :
      `<div class="card pad">${icon('alert')} <b>${esc(t('path_inexistant'))}</b><div class="muted">${esc(t('path_inexistant_description'))}</div><div style="margin-top:10px"><button class="btn small accent" data-settings>${icon('settings')} ${esc(t('settings'))}</button></div></div>`));
  }
  root.innerHTML = `<div class="nav"><div class="pane"><div class="ph"><button class="btn ghost small" id="back">${icon('back')}</button><div class="title">${esc(t('patch_a_game'))}</div></div>
    <div class="items"><div class="item ${!cur ? 'on' : ''}" data-goto="all">${icon('home')}<span>${esc(t('all_patches'))}</span></div><div class="sep"></div>
    ${packs.map(p => `<div class="item ${cur?.id === p.id ? 'on' : ''}" data-goto="${esc(p.id)}">${p.icon ? `<img src="${esc(assetUrl(p.icon))}" alt="" loading="lazy">` : icon('box')}<span>${esc(p.name)}</span></div>`).join('')}</div>
    <div class="item" id="toggle">${icon('box')}<span>${esc(store.showAllPacks ? t('show_owned_packs_only') : t('show_all_packs'))}</span></div></div>
    <div class="content">${main}</div></div>`;
  $('#back', root).onclick = () => location.hash = '#/';
  $('#toggle', root).onclick = () => { store.showAllPacks = !store.showAllPacks; render(root, [sel]); };
  $$('[data-goto]', root).forEach(e => e.onclick = () => location.hash = '#/patch/' + encodeURIComponent(e.dataset.goto));
  $$('[data-settings]', root).forEach(e => e.onclick = () => location.hash = '#/settings/packs');
  $$('.patch[data-patch]', root).forEach(el => {
    const pack = store.cat.packs.find(p => p.id === el.dataset.pack), g = el.dataset.game || null;
    const patch = (g ? pack.games.find(x => x.id === g).patchs : pack.patchs).find(x => x.id === el.dataset.patch);
    const again = () => render(root, [sel]);
    $('[data-info]', el).onclick = () => infoDialog(pack, patch, g, again);
    $('[data-install]', el)?.addEventListener('click', () => installFlow(pack, patch, g, again));
  });
}

export async function refreshState() { store.cat = await inv('server_catalog', { url: store.url, refresh: false }); }

export function installFlow(pack, patch, gameId, done) {
  confirmDialog(t('installing_a_patch'), t('installing_a_patch_description')).then(async ok => {
    if (!ok) return;
    const d = dialog({ title: t('installing_a_patch'), dismissable: false, buttons: [], html: `<p id="st">${esc(t('starting'))}…</p><div class="progress"><b id="pb" style="width:0%"></b></div>` });
    const fin = msg => { $('#st', d.el).textContent = msg; const b = document.createElement('button'); b.className = 'btn accent'; b.textContent = t('close'); b.onclick = () => { d.close(); done?.(); }; d.actions.append(b); };
    const un = await listen('patch-progress', e => { $('#st', d.el).textContent = t(e.payload.stage) + '…'; $('#pb', d.el).style.width = Math.round(e.payload.percent * 100) + '%'; });
    try {
      await inv('install_server_patch', { url: store.url, pack: pack.id, patch: patch.id, game: gameId });
      await refreshState(); $('#pb', d.el).style.width = '100%'; fin(t('installing_a_patch_end'));
    } catch (e) { fin(t('download_error') + ' — ' + e); } finally { un(); }
  });
}

export function infoDialog(pack, patch, gameId, done) {
  const st = patchStatus(pack.id, patch), inst = packState(pack.id).patches?.[patch.id];
  const types = [].concat(patch.patch_type || []).map(x => TYPES[String(x).toUpperCase()]).filter(Boolean);
  const d = dialog({ title: patch.name, html: `<div class="md">${md(patch.description || patch.small_description || '')}</div>
    <p class="caption" style="margin-top:10px">${esc(t('version'))}: ${esc(versionOf(patch))}${inst ? ' · ' + esc(t('installed_version')) + ': ' + esc(inst) : ''}${patch.authors ? ' · ' + esc(t('authors')) + ': ' + esc([].concat(patch.authors).join(', ')) : ''}</p>
    ${types.length ? `<p style="margin-top:10px"><b>${esc(t('patch_modification'))}</b></p><ul>${types.map(k => `<li>${esc(t(k))}</li>`).join('')}</ul>` : ''}`,
    buttons: [...(inst ? [{ label: t('delete_version'), onClick: async () => { if (await confirmDialog(t('delete_version'), t('delete_version_description'))) { await inv('forget_patch', { pack: pack.id, patch: patch.id }); await refreshState(); done?.(); } else return false; } }] : []),
      ...(st === 'none' || st === 'outdated' ? [{ label: t(ST[st][0], { count: 1 }), kind: 'accent', onClick: () => { setTimeout(() => installFlow(pack, patch, gameId, done), 0); } }] : []),
      { label: t('close') }] });
}
