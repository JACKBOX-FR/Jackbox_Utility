// Choix du serveur de patchs (SelectServerPage de l'appli d'origine).
import { store } from '../services/store.js';
import { inv } from '../services/tauri.js';
import { t, tf } from '../services/l10n.js';
import { esc, icon, $, $$, dialog, toast } from '../components/ui.js';

export async function render(root) {
  root.innerHTML = `<div class="page"><div class="select">
    <div class="head"><img src="assets/logo.png" alt=""><div class="title">${esc(t('jackbox_utility'))}</div><div class="muted body-large">${esc(t('select_server_subtitle'))}</div></div>
    <div class="grid" id="grid"></div></div></div>`;
  const grid = $('#grid', root);
  let local = [], main = [];
  try { local = await inv('catalog_urls'); } catch (e) { toast(String(e), 'err'); }
  const draw = () => {
    const urls = [...new Set([...main, ...local])];
    grid.innerHTML = urls.map((u, i) => `<div class="card server skeleton" data-u="${esc(u)}" id="s${i}"></div>`).join('') +
      `<div class="card server add"><div class="top"><div class="subtitle">${icon('settings', '28px')} ${esc(t('custom_server_title'))}</div></div>
       <div class="desc">${esc(t('custom_server_description'))}</div><div class="row"><button class="btn small" id="add">${icon('plus')} ${esc(t('add_pack'))}</button></div></div>`;
    $('#add', grid).onclick = addCustom;
    urls.forEach((u, i) => fill($('#s' + i, grid), u, !main.includes(u)));
  };
  const fill = async (card, url, custom) => {
    try {
      const i = await inv('server_info', { url, refresh: false });
      card.className = 'card server' + (store.url === url ? ' selected' : '');
      card.innerHTML = `<div class="top">${i.image_url ? `<img class="sv" src="${esc(i.image_url)}" alt="" data-fallback="hide">` : ''}<div><div class="subtitle">${esc(i.name)}</div>
        <div class="caption">${esc((i.languages || []).join(' · '))}</div></div></div><div class="desc">${esc(i.description)}</div>
        <div class="row">${custom ? `<button class="btn ghost small" data-rm title="Remove">${icon('trash')}</button>` : ''}<button class="btn accent small" data-sel>${esc(t('select_server_button'))}</button></div>`;
      $('[data-sel]', card).onclick = async () => { await inv('set_selected_server', { url }); store.cat = null; store.news = null; store.url = url; location.hash = '#/'; };
      $('[data-rm]', card)?.addEventListener('click', async () => { await inv('remove_catalog_url', { url }); local = local.filter(x => x !== url); draw(); });
    } catch { card.className = 'card server'; card.innerHTML = `<div class="subtitle">${icon('alert')} ${esc(t('connection_to_server_failed'))}</div><div class="desc caption">${esc(url)}</div>`; }
  };
  const addCustom = () => {
    const d = dialog({ title: t('custom_server_title'), dismissable: true,
      html: `<p class="muted">${esc(t('custom_server_explanation'))}</p><p style="color:#ffb27a;margin:10px 0">${icon('alert')} ${esc(t('custom_server_warning'))}</p>
        <div class="field"><input type="text" id="cu" placeholder="${esc(t('custom_server_link'))}"></div>`,
      buttons: [{ label: t('cancel') }, { label: t('confirm'), kind: 'accent', onClick: async () => {
        const url = $('#cu', d.el).value.trim();
        try { await inv('server_info', { url, refresh: true }); await inv('add_catalog_url', { url }); local.push(url); draw(); }
        catch (e) { toast(t('custom_server_error') + ' ' + e, 'err'); return false; } } }] });
  };
  draw();
  inv('fetch_main_servers').then(l => { main = l; draw(); }).catch(() => {});
}
