// Traductions : on réutilise les fichiers .arb de l'appli d'origine (un fichier par langue, 14 langues).
// Gère {variable}, {n, plural, =0 {…} one {…} other {…}} et {x, select, …}.
let dict = {}, en = {}, current = 'en';
export const LANGS = ['en', 'fr', 'de', 'es', 'pt', 'pt_BR', 'ru', 'uk', 'pl', 'tr', 'ca', 'nb', 'be', 'ta'];

async function get(code) {
  try { const r = await fetch(`assets/l10n/app_${code}.arb`); return r.ok ? await r.json() : {}; } catch { return {}; }
}
export async function loadLang(code) {
  // app_<lang>.arb = textes de l'appli d'origine ; extra_<lang>.arb = textes ajoutés par cette version (retombent sur l'anglais)
  const extra = async c => { try { const r = await fetch(`assets/l10n/extra_${c}.arb`); return r.ok ? await r.json() : {}; } catch { return {}; } };
  en = { ...(await get('en')), ...(await extra('en')) };
  dict = code === 'en' ? en : { ...(await get(code)), ...(await extra(code)) };
  current = code;
  document.documentElement.lang = code.replace('_', '-');
}
export const lang = () => current;
/** Comme t() mais avec un texte de secours si la clé n'existe pas dans les .arb (fonctions ajoutées par cette version). */
export function tf(key, fallback, params = {}) { const m = dict[key] ?? en[key]; return m === undefined ? fallback : fmt(m, params); }
export function t(key, params = {}) { return fmt(dict[key] ?? en[key] ?? key, params); }

function fmt(s, p) {
  let out = '', i = 0;
  while (i < s.length) {
    if (s[i] !== '{') { out += s[i++]; continue; }
    let d = 1, j = i + 1;
    while (j < s.length && d > 0) { if (s[j] === '{') d++; else if (s[j] === '}') d--; j++; }
    out += expr(s.slice(i + 1, j - 1), p); i = j;
  }
  return out;
}
function expr(body, p) {
  const m = body.match(/^\s*(\w+)\s*(?:,\s*(plural|select)\s*,([\s\S]*))?$/);
  if (!m) return '{' + body + '}';
  const [, name, kind, rest] = m, v = p[name];
  if (!kind) return v ?? '';
  const cases = {}; let pos = 0;
  while (pos < rest.length) {
    const re = /\s*(=?\w+)\s*\{/y; re.lastIndex = pos;
    const x = re.exec(rest); if (!x) break;
    let d = 1, j = re.lastIndex; const st = j;
    while (j < rest.length && d > 0) { if (rest[j] === '{') d++; else if (rest[j] === '}') d--; j++; }
    cases[x[1]] = rest.slice(st, j - 1); pos = j;
  }
  const key = kind === 'plural'
    ? (cases['=' + v] !== undefined ? '=' + v : (Number(v) === 1 && 'one' in cases ? 'one' : 'other'))
    : (String(v) in cases ? String(v) : 'other');
  return fmt((cases[key] ?? '').replace(/#/g, v), p);
}
