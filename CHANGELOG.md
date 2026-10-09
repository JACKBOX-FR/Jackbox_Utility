# Changelog

## 0.1.0 — réécriture en Rust (Tauri)

- Application Tauri v2 (Windows, macOS, Linux) qui reprend l'interface Fluent sombre, les écrans et les textes (.arb, 14 langues) de Jackbox Utility.
- Changement rapide de serveur de jeu : `joinUrl` et `serverUrl` réécrits dans tous les `jbg.config.jet`, propriété ajoutée si absente.
- Site local (dump jackbox.tv) avec couche de traduction par langue ; serveur 100 % hors-ligne via Jonahbox (cache, certificats, hosts).
- Catalogue de serveurs de patchs en cache (affichage immédiat), patchs avec progression, versions et mises à jour, détection Steam/Epic, lancement avec loaders.
- API REST + WebSocket de navigation pour extensions de navigateur.
- Commandes lourdes hors du thread de la fenêtre (fin des lenteurs), sécurité renforcée (échappement HTML, CSP, zips, état atomique).
