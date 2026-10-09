// État partagé de l'interface (équivalent des singletons UserData / APIService de l'appli d'origine).
export const store = {
  cfg: null,          // config.toml (JSON)
  url: null,          // info.json du serveur de patchs sélectionné
  cat: null,          // {info, assets, tags, categories, packs, state} du serveur
  news: null,         // {assets, news}
  showAllPacks: false,
  os: /Windows/i.test(navigator.userAgent) ? 'windows' : /Mac/i.test(navigator.userAgent) ? 'mac' : 'linux',
};
const subs = new Set();
export const subscribe = f => { subs.add(f); return () => subs.delete(f); };
export const notify = () => subs.forEach(f => f());

/** URL d'une ressource du serveur (absolue telle quelle, sinon relative à `assets`). */
export function assetUrl(p) {
  if (!p) return '';
  if (/^https?:/.test(p)) return p;
  return (store.cat?.assets || '').replace(/\/$/, '') + '/' + p.replace(/^\//, '');
}
export const packState = id => store.cat?.state?.[id] || {};
export const isOwned = id => !!packState(id).path;
export const versionOf = p => String(p.version ?? '').replace('Build:', '').trim();

/** none | installed | outdated | unavailable (mêmes états que l'appli d'origine) */
export function patchStatus(packId, patch) {
  const plat = patch.supported_platforms || ['windows', 'linux'];
  if (!plat.includes(store.os)) return 'unavailable';
  const inst = packState(packId).patches?.[patch.id];
  if (inst == null) return 'none';
  return inst === versionOf(patch) ? 'installed' : 'outdated';
}
/** Nombre de patchs à installer/mettre à jour sur les packs possédés (pastille rouge du menu). */
export function patchesAvailable() {
  let n = 0;
  for (const p of store.cat?.packs || []) {
    if (!isOwned(p.id)) continue;
    const all = [...(p.patchs || []), ...(p.games || []).flatMap(g => g.patchs || [])];
    for (const x of all) if (patchStatus(p.id, x) === 'outdated') n++;
  }
  return n;
}
