// Paramètres (ParametersRoute de l'appli d'origine) : jeux possédés, serveur, réseau, comportement, informations.
import { store, assetUrl, isOwned, packState, notify } from '../services/store.js';
import { inv, pickFolder } from '../services/tauri.js';
import { t, tf, lang, loadLang, LANGS } from '../services/l10n.js';
import { esc, icon, $, $$, run, toast, dialog } from '../components/ui.js';
import { ensureLoaded } from './main_container.js';

const TABS = [['packs', 'box', 'owned_packs'], ['server', 'server', 'server_information'], ['network', 'network', 'x_network'], ['behavior', 'settings', 'app_behavior'], ['info', 'info', 'app_information']];
const refresh = async () => { store.cat = await inv('server_catalog', { url: store.url, refresh: false }); };
const sw = (id, on) => `<label class="switch"><input type="checkbox" id="${id}" ${on ? 'checked' : ''}><i></i></label>`;
const setCfg = async (key, value) => { await inv('set_config_value', { key, value }); store.cfg = await inv('get_config_json'); };

export async function render(root, [tab = 'packs']) {
  if (!(await ensureLoaded())) return;
  root.innerHTML = `<div class="nav"><div class="pane"><div class="ph"><button class="btn ghost small" id="back">${icon('back')}</button><div class="title">${esc(t('settings'))}</div></div>
    <div class="items">${TABS.map(([id, ic, key]) => `<div class="item ${id === tab ? 'on' : ''}" data-tab="${id}">${icon(ic)}<span>${esc(t(key))}</span></div>`).join('')}</div></div>
    <div class="content" id="c"></div></div>`;
  $('#back', root).onclick = () => location.hash = '#/';
  $$('[data-tab]', root).forEach(e => e.onclick = () => location.hash = '#/settings/' + e.dataset.tab);
  const again = () => render(root, [tab]);
  await ({ packs, server, network, behavior, info }[tab] || packs)($('#c', root), again);
}

// ---- Jeux possédés ----
async function packs(c, again) {
  const owned = store.cat.packs.filter(p => isOwned(p.id)), free = store.cat.packs.filter(p => !isOwned(p.id));
  c.innerHTML = `<div class="title">${esc(t('owned_packs'))}</div>
    <div style="margin:12px 0"><button class="btn accent" id="det">${icon('search')} ${esc(t('automatic_game_finder_button'))}</button></div>
    ${owned.map(p => `<div class="card pad row-set" style="margin-bottom:8px"><div style="display:flex;gap:12px;align-items:center;min-width:0">${p.icon ? `<img src="${esc(assetUrl(p.icon))}" width="32" height="32" alt="">` : icon('box', '28px')}
      <div style="min-width:0"><b>${esc(p.name)}</b><div class="caption" style="overflow:hidden;text-overflow:ellipsis">${esc(t('pack_path'))}: ${esc(packState(p.id).path)} (${esc(packState(p.id).launcher || '')})</div></div></div>
      <div style="display:flex;gap:6px"><button class="btn small" data-pick="${esc(p.id)}">${icon('folder')}</button><button class="btn small" data-clear="${esc(p.id)}">${icon('trash')}</button></div></div>`).join('')}
    ${owned.length ? '' : `<p class="muted">${esc(t('path_inexistant_small_description'))}</p>`}
    <div class="subtitle" style="margin:22px 0 8px">${esc(t('add_pack'))}</div>
    <div class="field"><select id="np">${free.map(p => `<option value="${esc(p.id)}">${esc(p.name)}</option>`).join('')}</select><button class="btn" id="npb">${icon('folder')} ${esc(t('choose_pack'))}</button></div>`;
  const pick = async id => { const p = store.cat.packs.find(x => x.id === id); const path = await pickFolder(t('select_game_location', { game: p.name })); if (!path) return;
    await run(inv('set_pack_path', { url: store.url, pack: id, path }), ''); await refresh(); again(); };
  $$('[data-pick]', c).forEach(b => b.onclick = () => pick(b.dataset.pick));
  $$('[data-clear]', c).forEach(b => b.onclick = async () => { await inv('clear_pack_path', { pack: b.dataset.clear }); await refresh(); again(); });
  $('#npb', c).onclick = () => { const id = $('#np', c).value; if (id) pick(id); };
  $('#det', c).onclick = async () => { const d = dialog({ title: t('automatic_game_finder_title'), html: `<p>${esc(t('automatic_game_finder_in_progress'))}…</p>`, buttons: [], dismissable: false });
    try { const f = await inv('detect_games', { url: store.url }); await refresh(); d.body.innerHTML = `<p>${esc(t('automatic_game_finder_finish', { count: f.length }))}</p>`; }
    catch (e) { d.body.innerHTML = `<p>${esc(String(e))}</p>`; }
    const b = document.createElement('button'); b.className = 'btn accent'; b.textContent = t('close'); b.onclick = () => { d.close(); again(); }; d.actions.append(b); };
}

// ---- Serveur de patchs ----
async function server(c) {
  const i = store.cat.info;
  c.innerHTML = `<div class="title">${esc(t('server_information'))}</div><div class="card pad" style="margin-top:14px;display:flex;gap:16px;align-items:center">
    ${i.image ? `<img src="${esc(assetUrl(i.image))}" width="72" height="72" style="border-radius:8px;object-fit:cover" alt="" data-fallback="remove">` : ''}
    <div style="flex:1"><div class="subtitle">${esc(i.name)}</div><div class="muted">${esc(i.description || '')}</div><div class="caption" style="margin-top:6px">${esc((i.languages || []).join(' · '))}</div></div>
    <button class="btn accent" id="chg">${esc(t('change_server'))}</button></div>`;
  $('#chg', c).onclick = () => location.hash = '#/serverSelect';
}

// ---- Réseau : serveur de jeu, dump local, serveur hors-ligne ----
const JB = [['jb_install', 'x_jb_1'], ['jb_build', 'x_jb_2'], ['jb_cache', 'x_jb_3', { force: false }], ['jb_certs', 'x_jb_4'], ['jb_trust', 'x_jb_5', { on: true }], ['jb_hosts', 'x_jb_6', { on: true, pp1: false }],
  ['jb_start', 'x_jb_7'], ['jb_stop', 'x_stop'], ['switch_server', 'x_jb_8', { target: 'jonahbox' }], ['jb_commands', 'x_jb_fw'], ['jb_logs', 'x_jb_logs'], ['jb_trust', 'x_jb_untrust', { on: false }], ['jb_hosts', 'x_jb_unhosts', { on: false, pp1: true }]];

async function network(c) {
  const sv = await inv('servers'), cfg = store.cfg;
  c.innerHTML = `<div class="title">${esc(t('x_network'))}</div>
    <div class="card pad" style="margin:14px 0"><div class="subtitle">${esc(t('x_join'))}</div><p class="muted">${esc(t('x_join_desc'))}</p>
      <div class="field"><select id="srv">${sv.list.map(x => `<option ${x.name === sv.active ? 'selected' : ''}>${esc(x.name)}</option>`).join('')}</select>
      <input type="text" id="cust" placeholder="${esc(t('x_custom'))}"><button class="btn accent" id="apply">${esc(t('x_apply'))}</button></div></div>
    <div class="card pad" style="margin-bottom:14px"><div class="subtitle">${esc(t('x_local'))}</div><p class="muted">${esc(t('x_local_desc'))}</p>
      <div class="field"><button class="btn green" id="start">${icon('play', '1em', 'fill')} ${esc(t('x_start'))}</button><button class="btn" id="stop">${esc(t('x_stop'))}</button><button class="btn" id="dump">${icon('download')} ${esc(t('x_dump'))}</button></div>
      <div id="addr" class="field" style="flex-wrap:wrap"></div>
      <div class="field"><span class="muted">${esc(t('x_dump_lang'))}</span><input type="text" id="dl" size="6" style="flex:none;width:80px" value="${esc(cfg.local.dump_language)}"></div>
      <div class="field"><input type="text" id="xf" placeholder="main/pp7/everyday/script.js"><button class="btn" id="xs">${esc(t('x_extract'))}</button></div></div>
    <div class="card pad"><div class="subtitle">${esc(t('x_offline'))}</div><p class="muted">${esc(t('x_offline_desc'))}</p>
      <div id="jbst" class="caption" style="margin:6px 0"></div><div id="jbb" style="display:flex;flex-wrap:wrap;gap:6px">${JB.map((b, i) => `<button class="btn small" data-jb="${i}">${esc(t(b[1]))}</button>`).join('')}</div><pre id="jbo"></pre></div>`;
  $('#apply', c).onclick = () => run(inv('switch_server', { target: $('#cust', c).value.trim() || $('#srv', c).value }));
  $('#start', c).onclick = async () => { try { const i = await inv('start_local'); const as = [i.localhost, i.lan, i.external].filter(Boolean);
    $('#addr', c).innerHTML = as.map(a => `<button class="btn small" data-a="${esc(a)}">${esc(i.scheme)}://${esc(a)} → joinUrl</button>`).join('') + (i.note ? `<span style="color:#ffb27a">${esc(i.note)}</span>` : '');
    $$('[data-a]', c).forEach(b => b.onclick = () => run(inv('switch_server', { target: b.dataset.a }))); } catch (e) { toast(String(e), 'err'); } };
  $('#stop', c).onclick = () => { inv('stop_local'); $('#addr', c).innerHTML = ''; };
  $('#dump', c).onclick = () => { toast(t('x_dump_wait')); run(inv('update_dump')).catch(() => {}); };
  $('#dl', c).onchange = e => run(setCfg('local.dump_language', e.target.value.trim()), '').catch(() => {});
  $('#xs', c).onclick = () => run(inv('extract_strings', { file: $('#xf', c).value.trim() }));
  const status = async () => { try { const s = await inv('jb_status'), k = v => v ? '✅' : '❌';
    $('#jbst', c).innerHTML = `${k(s.installed)} ${esc(t('x_s_inst'))} · ${k(s.built)} ${esc(t('x_s_built'))} · ${k(s.cache)} cache · ${k(s.certs)} ${esc(t('x_s_certs'))} · ${k(s.running)} ${esc(t('x_s_run'))} — <b>${esc(s.host)}</b> → ${esc(s.ip)}`; } catch (e) { $('#jbst', c).textContent = '⚠ ' + e; } };
  $$('[data-jb]', c).forEach(b => b.onclick = async () => { const [cmd, key, args] = JB[+b.dataset.jb]; b.disabled = true; toast('… ' + t(key));
    try { const r = await inv(cmd, args || {}); if (cmd === 'jb_commands' || cmd === 'jb_logs') $('#jbo', c).textContent = r; else toast(typeof r === 'string' ? r : 'OK', 'ok'); }
    catch (e) { toast(String(e), 'err'); } b.disabled = false; status(); });
  status();
}

// ---- Comportement ----
async function behavior(c, again) {
  const cfg = store.cfg, names = new Intl.DisplayNames([lang().replace('_', '-')], { type: 'language' });
  const kinds = ['audio', 'subtitles', 'text', 'images', 'other'];
  c.innerHTML = `<div class="title">${esc(t('app_behavior'))}</div>
    <div class="row-set"><div>${icon('globe')} ${esc(t('x_language'))}</div><select id="lg">${LANGS.map(l => `<option value="${l}" ${l === cfg.language ? 'selected' : ''}>${esc(names.of(l.replace('_', '-')) || l)}</option>`).join('')}</select></div>
    <div class="subtitle" style="margin:18px 0 4px">${esc(t('x_patch_types'))}</div><p class="muted">${esc(t('x_patch_types_desc'))}</p>
    ${kinds.map(k => `<div class="row-set"><div>${esc(t('x_k_' + k))}</div>${sw('k_' + k, cfg.patch[k])}</div>`).join('')}
    <div class="subtitle" style="margin:18px 0 4px">${esc(t('x_api'))}</div>
    <div class="row-set"><div>${esc(t('x_api_desc'))}<div class="caption">${esc(t('x_restart'))}</div></div>${sw('api', cfg.api.enabled)}</div>`;
  $('#lg', c).onchange = async e => { await setCfg('language', e.target.value); await loadLang(e.target.value); location.hash = '#/settings/behavior'; again(); };
  kinds.forEach(k => $('#k_' + k, c).onchange = e => run(setCfg('patch.' + k, e.target.checked), '').catch(() => {}));
  $('#api', c).onchange = e => run(setCfg('api.enabled', e.target.checked), '').catch(() => {});
}

// ---- Informations ----
async function info(c) {
  const a = await inv('app_info'), dir = await inv('config_dir'), text = await inv('get_config');
  c.innerHTML = `<div style="display:flex;gap:18px;align-items:center"><img src="assets/logo.png" width="96" height="96" alt=""><div><div class="title">${esc(t('jackbox_utility'))}</div>
    <div class="muted">${esc(t('version'))} ${esc(a.version)}</div><div class="muted" style="max-width:520px">${esc(t('jackbox_utility_description'))}</div></div></div>
    <div style="display:flex;gap:8px;margin:16px 0"><button class="btn" data-href="${esc(a.github)}">${icon('external')} GitHub</button><button class="btn" data-href="${esc(a.discord)}">${icon('external')} Discord</button></div>
    <div class="subtitle" style="margin-top:20px">${esc(t('x_config_file'))}</div><div class="caption">${esc(dir)}</div>
    <textarea id="cfg" rows="14" style="margin-top:8px">${esc(text)}</textarea><div style="margin-top:8px"><button class="btn accent" id="save">${esc(t('x_save'))}</button></div>`;
  $$('[data-href]', c).forEach(b => b.onclick = () => inv('open_url', { url: b.dataset.href }));
  $('#save', c).onclick = async () => { await run(inv('save_config', { text: $('#cfg', c).value }), t('x_saved')).catch(() => {}); store.cfg = await inv('get_config_json').catch(() => store.cfg); };
}
