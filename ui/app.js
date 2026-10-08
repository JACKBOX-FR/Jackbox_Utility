// Interface : appelle le Rust via invoke(). Tout texte venant d'un serveur est échappé (esc) avant d'aller dans le HTML.
const inv = window.__TAURI__.core.invoke, $ = id => document.getElementById(id);
const esc = t => String(t ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const log = p => p.then(m => $('log').textContent = m ?? 'OK').catch(m => $('log').textContent = '⚠ ' + m);
let S = {}, active = new Set(), cache = null;
const T = (k, d) => S[k] ?? d;

async function init() {
  S = await inv('strings');
  document.querySelectorAll('[data-i]').forEach(el => el.textContent = T(el.dataset.i, el.textContent));
  $('custom').placeholder = T('ui_custom', '');
  const sv = await inv('servers');
  $('srv').innerHTML = sv.list.map(x => `<option ${x.name == sv.active ? 'selected' : ''}>${esc(x.name)}</option>`).join('');
  $('cfg').value = await inv('get_config'); $('dir').textContent = await inv('config_dir');
  $('cat').innerHTML = (await inv('catalog_urls')).map(x => `<option>${esc(x)}</option>`).join('');
}

function render() {
  if (!cache) return;
  const { tags, packs } = cache;
  $('tags').innerHTML = tags.map(t => `<button class="chip ${active.has(t.id) ? 'on' : ''}" data-t="${esc(t.id)}" title="${esc(t.description)}">${esc(t.name)}</button>`).join('');
  $('tags').querySelectorAll('button').forEach(b => b.onclick = () => { active.has(b.dataset.t) ? active.delete(b.dataset.t) : active.add(b.dataset.t); render(); });
  const show = g => [...active].every(t => g.tags.includes(t)); // filtre : le jeu doit avoir tous les tags choisis
  $('packs').innerHTML = packs.map(p => `<details><summary>${esc(p.name)} ${p.path ? '✅ ' + esc(p.launcher) : ''}</summary>
    <div><button data-l="${esc(p.id)}">▶ pack</button>
      <input data-path="${esc(p.id)}" size="28" placeholder="${esc(T('ui_setpath', 'Dossier du pack'))}" value="${esc(p.path ?? '')}"> <button data-sp="${esc(p.id)}">OK</button></div>`
    + p.games.filter(show).map(g => `<div><span class="gname" data-np="${esc(p.id)}" data-ng="${esc(g.id)}" data-nn="${esc(g.name)}">${esc(g.name)}</span> <button data-l="${esc(p.id)}" data-g="${esc(g.id)}">▶</button></div>`).join('')
    + p.patchs.map(x => `<div>🩹 ${esc(x.name)} <small>${esc(x.installed ? x.installed + (x.update ? ' → ' + x.version : ' ✔') : x.version)}</small>
        <button data-p="${esc(p.id)}" data-x="${esc(x.id)}" data-g="${esc(x.game_id ?? '')}">${x.update ? '⟳' : '⬇'}</button></div>`).join('') + '</details>').join('');
  const url = $('cat').value, q = sel => $('packs').querySelectorAll(sel);
  q('span.gname').forEach(sp => sp.onclick = () => inv('notify_page_open', { pack: sp.dataset.np, id: sp.dataset.ng, name: sp.dataset.nn })); // game_page_open (WebSocket)
  q('button[data-x]').forEach(b => b.onclick = () => log(inv('install_server_patch', { url, pack: b.dataset.p, patch: b.dataset.x, game: b.dataset.g || null }).then(r => { load(); return r; })));
  q('button[data-l]').forEach(b => b.onclick = () => log(inv('launch', { url, pack: b.dataset.l, game: b.dataset.g || null })));
  q('button[data-sp]').forEach(b => b.onclick = () => log(inv('set_pack_path', { url, pack: b.dataset.sp, path: $('packs').querySelector(`input[data-path="${CSS.escape(b.dataset.sp)}"]`).value.trim() }).then(l => { load(); return '✅ ' + l; })));
}

async function load() {
  const url = $('cat').value; $('packs').textContent = '…';
  try {
    const i = await inv('server_info', { url }); $('srvinfo').textContent = i.name + ' — ' + i.description;
    inv('server_news', { url }).then(n => $('news').innerHTML = (n.news || []).map(x => `<div>📰 <b>${esc(x.title)}</b> ${esc(x.content)}</div>`).join('')).catch(() => {});
    cache = await inv('server_packs', { url }); render();
  } catch (m) { $('packs').textContent = '⚠ ' + m; }
}

$('apply').onclick = () => log(inv('switch_server', { target: $('custom').value.trim() || $('srv').value }));
$('start').onclick = async () => { try { const i = await inv('start_local');
  $('addr').innerHTML = `${esc(i.scheme)}: <b>${esc(i.localhost)}</b> · <b>${esc(i.lan)}</b>` + (i.external ? ` · <b>${esc(i.external)}</b>` : '') + (i.note ? ` ⚠ ${esc(i.note)}` : '');
  $('custom').value = i.external || i.lan; } catch (m) { $('log').textContent = '⚠ ' + m; } };
$('dump').onclick = () => { $('log').textContent = '…'; log(inv('update_dump')); };
$('xs').onclick = () => log(inv('extract_strings', { file: $('xf').value.trim() }));
$('stop').onclick = () => { inv('stop_local'); $('addr').textContent = ''; };
$('load').onclick = load;
$('detect').onclick = () => log(inv('detect_games', { url: $('cat').value }).then(f => { load(); return f.length + ' pack(s) : ' + f.map(x => x.pack_id + ' (' + x.launcher + ')').join(', '); }));
$('install').onclick = () => log(inv('patch_install'));
$('restore').onclick = () => log(inv('restore'));
$('save').onclick = () => log(inv('save_config', { text: $('cfg').value }).then(() => 'OK'));

// Demande d'accès d'une extension navigateur (API REST) : l'utilisateur accepte ou refuse
window.__TAURI__.event.listen('extension-request', ev => {
  const { id, extension, scopes } = ev.payload;
  $('mt').textContent = T('ui_ext_title', 'Extension request');
  $('mb').textContent = (extension ?? '?') + ' — ' + scopes.join(', ');
  $('yes').textContent = T('ui_accept', 'Accept'); $('no').textContent = T('ui_refuse', 'Refuse');
  $('modal').style.display = 'flex';
  const done = ok => { $('modal').style.display = 'none'; inv('answer_extension', { id, accept: ok }); };
  $('yes').onclick = () => done(true); $('no').onclick = () => done(false);
});

// ---- Serveur complet hors-ligne (Jonahbox) : étapes à lancer dans l'ordre, chacune est facultative ----
const JB = [
  ['jb_install', '1. Télécharger'], ['jb_build', '2. Compiler'], ['jb_cache', '3. Cache (dump)', { force: false }], ['jb_certs', '4. Certificats'],
  ['jb_trust', '5. Faire confiance', { on: true }], ['jb_hosts', '6. Fichier hosts', { on: true, pp1: false }], ['jb_start', '7. Démarrer'], ['jb_stop', 'Arrêter'],
  ['switch_server', 'Appliquer aux jeux', { target: 'jonahbox' }], ['jb_commands', 'Pare-feu'], ['jb_logs', 'Logs'],
  ['jb_trust', 'Retirer confiance', { on: false }], ['jb_hosts', 'Retirer hosts', { on: false, pp1: true }]];
$('jbbtns').innerHTML = JB.map((b, i) => `<button data-jb="${i}" style="margin:2px">${esc(b[1])}</button>`).join('');
async function jbStatus() {
  try {
    const s = await inv('jb_status'), ok = v => v ? '✅' : '❌';
    $('jbstatus').innerHTML = `${ok(s.installed)} téléchargé · ${ok(s.built)} compilé · ${ok(s.cache)} cache · ${ok(s.certs)} certificats · ${ok(s.running)} en marche — <b>${esc(s.host)}</b> → ${esc(s.ip)}`;
  } catch (m) { $('jbstatus').textContent = '⚠ ' + m; }
}
$('jbbtns').querySelectorAll('button').forEach(b => b.onclick = async () => {
  const [cmd, label, args] = JB[+b.dataset.jb]; $('log').textContent = '… ' + label;
  try {
    const r = await inv(cmd, args || {});
    if (['jb_commands', 'jb_logs'].includes(cmd)) $('jbout').textContent = r; else $('log').textContent = r ?? 'OK';
  } catch (m) { $('log').textContent = '⚠ ' + m; }
  jbStatus();
});
jbStatus();
init();
