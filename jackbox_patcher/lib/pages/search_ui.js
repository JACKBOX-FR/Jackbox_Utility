// Recherche de jeux (SearchGameMenuWidget / SearchGameRoute de l'appli d'origine).
import { store, assetUrl, isOwned } from '../services/store.js';
import { t } from '../services/l10n.js';
import { esc, icon, $, $$ } from '../components/ui.js';
import { ensureLoaded } from './main_container.js';

const norm = s => String(s ?? '').normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
const S = { q: '', pack: null, tags: new Set(), players: 0 };

export async function render(root) {
  if (!(await ensureLoaded())) return;
  const packs = () => store.cat.packs.filter(p => store.showAllPacks || isOwned(p.id));
  const games = () => packs().flatMap(p => (p.games || []).map(g => ({ g, p })));
  const match = ({ g, p }) => {
    const i = g.game_info || {};
    return (!S.q || norm(g.name).includes(norm(S.q))) && (!S.pack || p.id === S.pack) && [...S.tags].every(x => (i.tags || []).includes(x))
      && (!S.players || (i.players && i.players.min <= S.players && S.players <= i.players.max));
  };
  const shell = () => {
    root.innerHTML = `<div class="page"><div class="content" style="max-width:1100px;margin:0 auto">
      <div style="display:flex;align-items:center;gap:10px"><button class="btn ghost small" id="back">${icon('back')}</button><div class="title">${esc(t('search_game'))}</div></div>
      <p class="muted" style="margin-top:4px">${esc(t('all_games_description'))}</p>
      <div class="searchbar"><input type="text" id="q" placeholder="${esc(t('search'))}…" value="${esc(S.q)}">
        <select id="pl"><option value="0">${esc(t('filter_players_number'))}</option>${[1, 2, 3, 4, 5, 6, 7, 8].map(n => `<option ${S.players === n ? 'selected' : ''}>${n}</option>`).join('')}</select>
        <button class="btn" id="rnd">${icon('refresh')} ${esc(t('random_game'))}</button><button class="btn" id="all">${esc(store.showAllPacks ? t('show_owned_packs_only') : t('show_all_packs'))}</button></div>
      <div class="muted caption">${esc(t('search_by_pack'))}</div><div class="chips" id="pc">${packs().map(p => `<button class="chip ${S.pack === p.id ? 'on' : ''}" data-pack="${esc(p.id)}">${esc(p.name)}</button>`).join('')}</div>
      <div class="muted caption">${esc(t('search_by_tags'))}</div><div class="chips" id="tc">${(store.cat.tags || []).map(x => `<button class="chip ${S.tags.has(x.id) ? 'on' : ''}" data-tag="${esc(x.id)}" title="${esc(x.description || '')}">${esc(x.name)}</button>`).join('')}</div>
      <div id="res"></div></div></div>`;
    $('#back', root).onclick = () => location.hash = '#/';
    $('#q', root).oninput = e => { S.q = e.target.value; list(); };
    $('#pl', root).onchange = e => { S.players = +e.target.value || 0; list(); };
    $('#all', root).onclick = () => { store.showAllPacks = !store.showAllPacks; S.pack = null; shell(); };
    $('#rnd', root).onclick = () => { const l = games().filter(match); if (l.length) { const r = l[Math.floor(Math.random() * l.length)]; location.hash = `#/game/${encodeURIComponent(r.p.id)}/${encodeURIComponent(r.g.id)}`; } };
    $$('[data-pack]', root).forEach(b => b.onclick = () => { S.pack = S.pack === b.dataset.pack ? null : b.dataset.pack; shell(); });
    $$('[data-tag]', root).forEach(b => b.onclick = () => { S.tags.has(b.dataset.tag) ? S.tags.delete(b.dataset.tag) : S.tags.add(b.dataset.tag); shell(); });
    list();
  };
  const list = () => {
    const l = games().filter(match);
    $('#res', root).innerHTML = l.length ? `<div class="games">${l.map(({ g, p }) => { const i = g.game_info || {};
      return `<div class="game" data-p="${esc(p.id)}" data-g="${esc(g.id)}" style="background-image:url('${esc(assetUrl(g.background))}')"><div><b>${esc(g.name)}</b>
        <span>${i.players ? esc(i.players.min + '–' + i.players.max + ' ' + t('players')) + ' · ' : ''}${esc(p.name)}</span></div></div>`; }).join('')}</div>`
      : `<div class="card pad"><b>${esc(t('no_game_in_this_category_title'))}</b><div class="muted">${esc(t('no_game_in_this_category_description'))}</div></div>`;
    $$('.game', root).forEach(el => el.onclick = () => location.hash = `#/game/${encodeURIComponent(el.dataset.p)}/${encodeURIComponent(el.dataset.g)}`);
  };
  shell();
}
