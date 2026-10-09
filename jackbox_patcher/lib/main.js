// Point d'entrée : routes (équivalent des routes de main.dart), langue, événements globaux.
import { store, subscribe } from './services/store.js';
import { inv, listen } from './services/tauri.js';
import { loadLang, t, tf } from './services/l10n.js';
import { esc, dialog, $ } from './components/ui.js';
import * as main from './pages/main_container.js';
import * as selectServer from './pages/select_server.js';
import * as settings from './pages/settings.js';
import * as patcher from './pages/patcher.js';
import * as search from './pages/search_ui.js';
import * as game from './pages/game_ui.js';

const routes = { '': main, serverSelect: selectServer, settings, patch: patcher, searchMenu: search, search, game };
const app = document.getElementById('app');
let current = '';

async function route() {
  const [name = '', ...rest] = location.hash.replace(/^#\/?/, '').split('/');
  current = name;
  app.innerHTML = '';
  const el = document.createElement('div'); el.style.height = '100%'; app.append(el);
  try { await (routes[name] || main).render(el, rest.map(decodeURIComponent)); }
  catch (e) { el.innerHTML = `<div class="center"><div class="error-box">${esc(String(e))}</div><a class="btn accent" href="#/">OK</a></div>`; }
}
window.addEventListener('hashchange', route);
// Les données ont changé en arrière-plan (nouvelles, mise à jour du catalogue) : on redessine les pages de consultation
subscribe(() => { if (['', 'patch', 'game', 'searchMenu'].includes(current) && !document.querySelector('.overlay')) route(); });

// Image introuvable (réseau coupé, serveur indisponible) : masquée via data-fallback (les onerror inline sont interdits par la CSP)
document.addEventListener('error', e => { const i = e.target; if (i.tagName === 'IMG' && i.dataset.fallback) { if (i.dataset.fallback === 'remove') i.remove(); else i.style.visibility = 'hidden'; } }, true);
// Liens externes (markdown, boutons) : ouverts dans le navigateur par défaut
document.addEventListener('click', e => { const a = e.target.closest('a[data-href]'); if (a) { e.preventDefault(); inv('open_url', { url: a.dataset.href }); } });
// Échap / bouton "précédent" de la souris = retour (closable_route_with_esc, mouse_back_button_recognizer)
document.addEventListener('keydown', e => { if (e.key !== 'Escape') return; const ov = document.querySelector('.overlay'); if (ov) ov.remove(); else if (location.hash && location.hash !== '#/') history.back(); });
document.addEventListener('mouseup', e => { if (e.button === 3) history.back(); });

// Une extension de navigateur demande l'accès à l'API locale
listen('extension-request', ev => {
  const { id, extension, scopes } = ev.payload;
  dialog({ title: tf('x_ext_title', 'Extension request'), dismissable: false, html: `<p><b>${esc(extension ?? '?')}</b></p><p class="muted">${esc(scopes.join(', '))}</p>`,
    buttons: [{ label: tf('x_refuse', 'Refuse'), onClick: () => inv('answer_extension', { id, accept: false }) }, { label: tf('x_accept', 'Accept'), kind: 'accent', onClick: () => inv('answer_extension', { id, accept: true }) }] });
});

(async () => {
  try { store.cfg = await inv('get_config_json'); await loadLang(store.cfg.language || 'en'); }
  catch (e) { await loadLang('en'); }
  route();
})();
