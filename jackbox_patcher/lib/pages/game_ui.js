// Fiche d'un jeu (GameInfoRoute de l'appli d'origine) : infos, captures, lancement, patchs du jeu.
import { store, assetUrl, isOwned, patchStatus, versionOf } from '../services/store.js';
import { inv } from '../services/tauri.js';
import { t } from '../services/l10n.js';
import { esc, icon, $, $$, run, toast, md } from '../components/ui.js';
import { ensureLoaded } from './main_container.js';
import { installFlow, infoDialog } from './patcher.js';

const TYPE = { COOP: 'game_type_coop', VERSUS: 'game_type_versus', TEAM: 'game_type_team' };

export async function render(root, [packId, gameId]) {
  if (!(await ensureLoaded())) return;
  const pack = store.cat.packs.find(p => p.id === packId), game = pack?.games.find(g => g.id === gameId);
  if (!game) { location.hash = '#/searchMenu'; return; }
  inv('notify_page_open', { pack: pack.id, id: game.id, name: game.name }).catch(() => {});
  const i = game.game_info || {}, owned = isOwned(pack.id), shots = (i.images || []).filter(x => /\.(webp|png|jpe?g)$/i.test(x));
  const patches = game.patchs || [];
  root.innerHTML = `<div class="page"><button class="btn back" id="back">${icon('back')}</button>
    <div class="gamehero" style="background-image:url('${esc(assetUrl(game.background))}')"></div>
    <div class="gamebody"><div class="title-large">${esc(game.name)}</div><div class="body-large muted">${esc(i.tagline || '')}</div>
    <div class="facts">${i.players ? `<span>${icon('users')} ${i.players.min}–${i.players.max} ${esc(t('players'))}</span>` : ''}
      ${i.playtime ? `<span>${icon('clock')} ${i.playtime.min}–${i.playtime.max} ${esc(t('minutes'))}</span>` : ''}
      ${TYPE[i.type] ? `<span>${icon('star')} ${esc(t(TYPE[i.type]))}</span>` : ''}<span>${icon('box')} ${esc(pack.name)}</span></div>
    <div style="display:flex;gap:10px;flex-wrap:wrap;margin:14px 0">
      <button class="btn green" id="launch" ${owned ? '' : 'disabled'}>${icon('play', '1em', 'fill')} ${esc(t('launch_game'))}</button>
      <button class="btn" id="launchpack" ${owned ? '' : 'disabled'}>${esc(t('launch_pack'))}</button></div>
    ${owned ? '' : `<div class="card pad" style="margin-bottom:14px">${icon('alert')} ${esc(t('path_inexistant_description'))} <button class="btn small accent" id="set" style="margin-left:8px">${icon('settings')} ${esc(t('settings'))}</button></div>`}
    <div class="md">${md(i.description || '')}</div>
    ${(i.tags || []).length ? `<div class="chips" style="margin-top:12px">${i.tags.map(x => `<span class="chip">${esc((store.cat.tags.find(y => y.id === x) || { name: x }).name)}</span>`).join('')}</div>` : ''}
    ${i.audience ? `<p class="muted" style="margin-top:10px"><b>${esc(t('audience'))}</b> — ${esc(i.audience_description || '')}</p>` : ''}
    ${shots.length ? `<div class="shots">${shots.map(x => `<img src="${esc(assetUrl(x))}" loading="lazy" alt="">`).join('')}</div>` : ''}
    ${patches.length ? `<div class="group">${esc(t('patch_a_game'))}</div>${patches.map(x => { const st = patchStatus(pack.id, x);
      return `<div class="card patch" data-patch="${esc(x.id)}"><div class="grow"><div class="nm">${esc(x.name)}</div><div class="caption">${esc(x.small_description || '')} ${versionOf(x) ? '· ' + esc(versionOf(x)) : ''}</div></div>
        ${st === 'none' || st === 'outdated' ? `<button class="btn small accent" data-inst>${icon('download')} ${esc(t(st === 'none' ? 'patch_not_installed' : 'patch_outdated', { count: 1 }))}</button>` : `<span class="badge ${st === 'installed' ? 'ok' : 'off'}">${esc(t(st === 'installed' ? 'patch_installed' : 'patch_unavailable', { count: 1 }))}</span>`}</div>`; }).join('')}` : ''}
    </div></div></div>`;
  $('#back', root).onclick = () => history.back();
  $('#set', root)?.addEventListener('click', () => location.hash = '#/settings/packs');
  const go = (gid) => async () => { toast(t('launching')); await run(inv('launch', { url: store.url, pack: pack.id, game: gid }), t('launched')).catch(() => {}); };
  $('#launch', root).onclick = go(game.id); $('#launchpack', root).onclick = go(null);
  $$('.patch[data-patch]', root).forEach(el => { const x = patches.find(y => y.id === el.dataset.patch);
    $('[data-inst]', el)?.addEventListener('click', () => installFlow(pack, x, game.id, () => render(root, [packId, gameId]))); });
}
