// Actualités du serveur (NotificationCaroussel de l'appli d'origine).
import { esc, md, dialog, $$ } from './ui.js';
import { assetUrl } from '../services/store.js';
import { t } from '../services/l10n.js';

export function carousel(news) {
  const list = news?.news || [];
  if (!list.length) return '';
  const img = p => !p ? '' : /^https?:/.test(p) ? p : (news.assets || '').replace(/\/$/, '') + '/' + p;
  return `<div class="carousel">${list.map((n, i) => `<div class="news" data-news="${i}" style="background-image:url('${esc(img(n.image))}')">
    <div><b>${esc(n.title)}</b><span>${esc(n.smallDescription || '')}</span></div></div>`).join('')}</div>`;
}
export function bindCarousel(root, news) {
  $$('[data-news]', root).forEach(el => el.onclick = () => {
    const n = news.news[+el.dataset.news];
    dialog({ title: n.title, html: `<div class="md">${md(n.content || n.smallDescription || '')}</div>`, buttons: [{ label: t('close') }] });
  });
}
