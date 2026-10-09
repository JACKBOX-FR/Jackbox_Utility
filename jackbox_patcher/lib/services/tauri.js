// Pont avec le Rust (commandes Tauri) : un seul endroit à adapter si une commande change.
const T = window.__TAURI__;
export const inv = (cmd, args) => T.core.invoke(cmd, args);
export const listen = (event, fn) => T.event.listen(event, fn);
/** Sélecteur de dossier natif (plugin dialog). Retourne null si annulé. */
export async function pickFolder(title) {
  const r = await T.dialog.open({ directory: true, title });
  return r || null;
}
