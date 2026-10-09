// Page d'accueil + chargement (MainContainer / LoadingContainer de l'appli d'origine).
import { store, notify, patchesAvailable, assetUrl } from '../services/store.js';
import { inv } from '../services/tauri.js';
import { t, lang } from '../services/l10n.js';
import { esc, icon, ring, $, $$ } from '../components/ui.js';
import { carousel, bindCarousel } from '../components/notifications_carousel.js';

const STEP_ICONS = ['play', 'tower', 'download'];
const fingerprint = c => (c.packs || []).map(p => p.id + ':' + (p.patchs || []).map(x => x.version).join(',') + ':' + (p.games || []).map(g => g.id + (g.patchs || []).map(x => x.version).join()).join('|')).join(';');

/** Choisit le serveur dont la langue correspond à celle de l'appli (automatic_server_finder). */
async function autoPickServer() {
  const urls = await inv('catalog_urls');
  const infos = await Promise.allSettled(urls.map(u => inv('server_info', { url: u, refresh: false })));
  const i = infos.findIndex(r => r.status === 'fulfilled' && (r.value.languages || []).includes(lang().split('_')[0]));
  return i >= 0 ? urls[i] : null;
}

/** Charge le serveur sélectionné : cache d'abord (instantané), puis rafraîchissement en arrière-plan. */
export async function loadServer(onStep = () => {}) {
  onStep(1, 40);
  store.cfg ??= await inv('get_config_json');
  let url = await inv('get_selected_server');
  if (!url) { url = await autoPickServer(); if (!url) { location.hash = '#/serverSelect'; return false; } await inv('set_selected_server', { url }); }
  store.url = url; onStep(1, 100); onStep(2, -1);
  let cat;
  try { cat = await inv('server_catalog', { url, refresh: false }); } catch { cat = await inv('server_catalog', { url, refresh: true }); }
  store.cat = cat; onStep(2, 100); onStep(3, 50);
  inv('server_news', { url }).then(n => { store.news = n; notify(); }).catch(() => {});
  onStep(3, 100);
  inv('server_catalog', { url, refresh: true }).then(c => {
    if (store.url !== url || !store.cat) return;
    const changed = fingerprint(c) !== fingerprint(store.cat);
    store.cat = c; if (changed) notify();
  }).catch(() => {});
  return true;
}
/** Les autres pages appellent ceci : si les données ne sont pas là (rechargement de la fenêtre…), on les charge sans quitter la page. */
export async function ensureLoaded() {
  if (store.cat) return true;
  try { return await loadServer(); } catch { location.hash = '#/'; return false; }
}

function loadingHtml(step, pct, err) {
  const parts = STEP_ICONS.map((ic, i) => {
    const n = i + 1, p = step > n ? 100 : step === n ? pct : 0, bad = err && step === n;
    return `<div class="tl-step">${ring(bad ? 100 : p, bad ? '#d13438' : '#fff')}${icon(ic, '24px')}</div>` +
      (n < 3 ? `<div class="tl-line"><b style="transform:scaleX(${step > n ? 1 : 0})"></b></div>` : '');
  }).join('');
  return `<div class="center"><img src="assets/logo.png" width="96" height="96" alt=""><div class="title">${esc(t('jackbox_utility'))}</div><div class="timeline">${parts}</div>${
    err ? `<div class="error-box">${esc(err)}</div><div style="display:flex;gap:10px"><button class="btn accent" id="retry">${icon('refresh')} ${esc(t('page_continue'))}</button><button class="btn" id="change">${icon('server')} ${esc(t('change_server'))}</button></div>` : ''}</div>`;
}

export async function render(root) {
  if (!store.cat) {
    let step = 1, pct = 0;
    const paint = (err) => { root.innerHTML = `<div class="page">${loadingHtml(step, pct, err)}</div>`;
      $('#retry', root)?.addEventListener('click', () => render(root)); $('#change', root)?.addEventListener('click', () => location.hash = '#/serverSelect'); };
    paint();
    try { if (!(await loadServer((s, p) => { step = s; pct = p; paint(); }))) return; }
    catch (e) { paint(String(e)); return; }
  }
  const c = store.cat, avail = patchesAvailable();
  root.innerHTML = `<div class="page"><div class="main">
    ${c.info.image ? `<img class="logo" src="${esc(assetUrl(c.info.image))}" alt="" data-fallback="remove">` : ''}
    <div class="title-large">${esc(c.info.name)}</div>
    <div class="server-line">${esc(t('connected_to_server', { server: c.info.name }))} <a id="chg">${esc(t('connected_to_server_change'))}</a></div>
    <div class="menu">
      <button class="btn green big" id="play">${icon('play', '1em', 'fill')} ${esc(t('launch_search_game'))}</button>
      <button class="btn accent big" id="patch">${avail ? '<span class="dot"></span>' : ''}${icon('download')} ${esc(t('patch_a_game'))}</button>
      <div class="gap"></div>
      <button class="btn grey big" id="set">${icon('settings')} ${esc(t('settings'))}</button>
    </div>
    ${carousel(store.news)}</div></div>`;
  bindCarousel(root, store.news);
  $('#play', root).onclick = () => location.hash = '#/searchMenu';
  $('#patch', root).onclick = () => location.hash = '#/patch/all';
  $('#set', root).onclick = () => location.hash = '#/settings/packs';
  $('#chg', root).onclick = () => location.hash = '#/serverSelect';
}
