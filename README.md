# Jackbox Utility (réécriture Rust / Tauri)

Application de bureau (Windows, macOS, Linux) pour **patcher et lancer les jeux Jackbox**, **changer rapidement de serveur** (`joinUrl` / `serverUrl` dans tous les `jbg.config.jet`), servir **la manette jackbox.tv en local** (avec traduction par langue) et jouer **sans internet** avec un serveur local ([Jonahbox](https://github.com/JACKBOX-FR/Jonahbox-Dump-Ecast-Blobcast)). Même interface (Fluent sombre), mêmes écrans et mêmes textes que l'appli d'origine.

> Tout est facultatif : le dump, les patchs de langue, la traduction du contrôleur et le serveur hors-ligne ne s'activent que sur clic. Le serveur `officiel` (jackbox.tv) est toujours dans la liste pour revenir à l'original.

## Structure du dépôt (comme l'appli d'origine)

```
servers.json                          liste officielle des serveurs de patchs
documentation/                        guides détaillés (fonctionnement, API REST, Jonahbox, sécurité…)
.github/workflows/build.yml           compile Windows / macOS / Linux
jackbox_patcher/
  package.json
  lib/                                interface (HTML/JS, sans bundler)  ≙ lib/ de l'appli Flutter
    index.html, styles.css, main.js   point d'entrée et routes
    pages/                            main_container, select_server, search_ui, game_ui, patcher, settings
    components/                       ui.js (dialogues, icônes…), notifications_carousel.js
    services/                         tauri.js (pont vers le Rust), l10n.js (.arb), store.js
    assets/                           images, logos, l10n/*.arb (un fichier par langue)
  src-tauri/
    tauri.conf.json, Cargo.toml, capabilities/, icons/
    defaults/                         fichiers copiés au 1er lancement (config.toml, catalog.toml, translations/…)
    src/
      main.rs                         commandes Tauri
      app_configuration.rs            constantes (≙ app_configuration.dart)
      model/                          config.rs, state.rs
      services/
        api_utility.rs                serveurs de patchs (info.json, packs.json, cache, installation)
        downloader.rs                 téléchargement sur disque + extraction des zips
        launcher.rs                   lancement Steam / Epic / natif, loaders
        automatic_game_finder.rs      détection Steam / Epic
        patch_install_controller.rs   contrôleur d'installation TMP3
        internal_api.rs               API REST + WebSocket pour extensions de navigateur
        files.rs                      réécriture de joinUrl / serverUrl
        translations.rs, i18n.rs      traduction du dump, messages du Rust
        local_server.rs               site local (dump jackbox.tv), UPnP, HTTPS
        jonahbox/                     serveur 100 % local (mod.rs) et conversion du dump en cache (cache.rs)
```

## Lancer / compiler

Prérequis : Node 20+, Rust stable, et sous Linux `libwebkit2gtk-4.1-dev` (voir la doc Tauri v2).

```
cd jackbox_patcher
npm install
npx tauri dev        # test
npx tauri build      # installeur pour ton OS
```

Les icônes sont déjà dans `src-tauri/icons` (carré jaune provisoire : remplace `src-tauri/icons/icon.png` par ton logo 1024×1024 puis `npx tauri icon src-tauri/icons/icon.png`). **Les 3 OS d'un coup** : pousse sur GitHub puis `git tag v0.1.0 && git push --tags` ; la release brouillon contient `.msi`/`.exe`, `.dmg`, `.deb`/`.AppImage` (non signés : avertissement du système au 1er lancement).

## Guides

- [Comment ça marche](documentation/how-it-works.md) — fichiers de configuration, changement de serveur, site local, traduction, patchs, détection
- [Trois façons de jouer](documentation/servers-and-modes.md) — officiel, dump local, serveur complet
- [Serveur hors-ligne (Jonahbox)](documentation/jonahbox.md)
- [API REST et WebSocket](documentation/rest-api.md)
- [Sécurité, robustesse et performances](documentation/security.md)

## État

Écrans portés : accueil, choix du serveur, recherche et fiche de jeu, patch (panneau « Tous les patchs » + packs), paramètres (jeux possédés, serveur, réseau, comportement, informations). Non repris volontairement : Discord Rich Presence, statistiques, version lue dans les fichiers du jeu (il suffit de re-patcher par-dessus). Pas encore : classement personnel / jeux masqués, filtres par type et traduction dans la recherche, vidéos des fiches de jeu.

> Le projet n'a pas pu être compilé ni lancé dans l'environnement où il a été généré : les modules sans Tauri sont testés (`cargo test`), l'interface a été vérifiée dans un navigateur avec une API simulée. Le premier build peut demander de petits correctifs.

## Licence

**AGPL-3.0-or-later** (fichier `LICENSE`) : `services/jonahbox/cache.rs` est un port de `dump_to_cache.py` de Jonahbox, qui est sous AGPL. L'appli ne contient pas le code de Jonahbox : elle le télécharge et le lance comme programme séparé. Les assets (logo, motif, fichiers `.arb`) viennent du dépôt Jackbox Utility d'origine.
