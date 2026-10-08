# Jackbox Utility RS (Tauri)

Réécriture en Rust + Tauri v2 de JackboxUtility. Une appli de bureau (Windows / macOS / Linux) pour
changer de serveur Jackbox, lancer un serveur local, installer des patchs de traduction et remettre le jeu à zéro.

## Tout est facultatif

Rien n'est imposé au joueur : le dump local, les patchs de langue et la traduction du contrôleur sont **chacun optionnels** (aucun téléchargement automatique). Le serveur `officiel` (jackbox.tv) est toujours dans la liste pour revenir à l'original, et « Remise à zéro » restaure les fichiers d'origine (dépôt `[restore]`). Seule l'API pour extensions tourne au lancement (127.0.0.1 uniquement) : `[api] enabled = false` la coupe.

## Ce qui est implémenté / pas encore

| Fonction | État |
|---|---|
| Changer de serveur (`joinUrl` dans tous les `jbg.config.jet`) | ✅ |
| Serveur local (sert le dump `jackbox-fr-main-dump`), adresse prête pour `joinUrl` | ✅ (HTTP) |
| Patchs depuis dépôts GitHub, choix par type (ogg / swf+json / jet / images) et par jeu | ✅ |
| Remise à zéro depuis un dépôt `[restore]` | ✅ (dépôt à renseigner) |
| Un fichier par langue (interface) et par langue (dépôts) | ✅ |
| Builds Windows / macOS / Linux via GitHub Actions | ✅ |
| Catalogue des serveurs `info.json` : liste des packs, jeux et patchs, installation (patchs de pack ou de jeu) | ✅ |
| Contrôleur d'installation `tmp3` (URLs UTF-16 dans l'exécutable) | ✅ |
| Détection Steam / Epic (bouton Détecter), lancement pack/jeu (Steam, Epic, natif) avec loaders, `jbx launch pack|game <id>` en ligne de commande | ✅ |
| Versions installées / mises à jour (⟳), news du serveur, cache hors-ligne des serveurs | ✅ |
| UPnP (`upnp = true`) et HTTPS auto-signé (`https = true`) pour le serveur local | ✅ (HTTPS : les jeux peuvent refuser le certificat) |
| Serveur 100 % local sans internet (Jonahbox : ecast + blobcast), installé pas à pas depuis l'appli | ✅ (voir section dédiée ; non testé sur un vrai jeu) |
| Tags (filtre des jeux), dossier de pack choisi à la main (« game finder from path ») | ✅ |
| API REST + WebSocket de navigation pour extensions navigateur (`127.0.0.1:6480`, acceptation par l'utilisateur) | ✅ |
| Copie locale du site jackbox.tv (dump) : téléchargement/màj en un clic, servi comme `http-server`, avec couche de traduction par langue | ✅ |
| Discord Rich Presence, statistiques, version lue dans les fichiers du jeu | ❌ volontairement non porté (re-patcher par-dessus suffit) |
| HTTPS local, ouverture de ports UPnP | ⏳ |

> Ce projet n'a pas été compilé dans l'environnement où il a été généré : le premier `cargo build` peut demander de petits correctifs.

## Où est quoi

    ui/index.html, ui/app.js      Interface (HTML/JS, pas de bundler). Appelle le Rust via invoke(). CSP stricte : pas de script inline.
    src-tauri/src/main.rs         Commandes Tauri (switch_server, patch_install, restore, start_local, ...)
    src-tauri/src/switch.rs       Réécrit/ajoute la ligne `joinUrl:` dans chaque jbg.config.jet
    src-tauri/src/local.rs        Serveur web statique (axum) + détection de l'IP locale
    src-tauri/src/translations.rs Traduction du dump à la volée (translations/<langue>.toml) + extraction des textes
    src-tauri/src/jonahbox.rs     Serveur 100 % local : téléchargement, compilation, certificats, confiance, hosts, démarrage
    src-tauri/src/jb_cache.rs     Dump jackbox.tv -> cache Jonahbox (port de dump_to_cache.py), édition de hosts, config.toml de Jonahbox
    src-tauri/src/api.rs          API REST locale (status, register, games/list, games/open/<id>)
    src-tauri/src/state.rs        state.json : packs détectés, patchs installés (versions), loaders, dernier serveur
    src-tauri/src/finder.rs       Détection Steam (libraryfolders.vdf, appmanifest) et Epic (LauncherInstalled.dat)
    src-tauri/src/launcher.rs     Lancement Steam/Epic/natif, loaders, arguments -launchTo
    src-tauri/src/catalog.rs      Lit info.json -> api/packs.json, liste packs/jeux/patchs, télécharge et installe un patch
    src-tauri/src/tmp3.rs         Contrôleur d'installation TMP3 (port de tmp3_install_controller.dart)
    src-tauri/src/patch.rs        Téléchargement zip d'un dépôt GitHub + extraction filtrée
    src-tauri/src/config.rs       Structures de config.toml et des fichiers sources/<langue>.toml
    src-tauri/src/i18n.rs         Chargement de langs/<langue>.toml
    src-tauri/defaults/           Fichiers par défaut, copiés au 1er lancement
    src-tauri/tauri.conf.json     Nom, identifiant, icônes, cibles de build
    .github/workflows/build.yml   Compile les 3 OS et publie une release (brouillon)
    app-icon.png                  Icône source (remplace-la par la tienne, 1024x1024 idéal)

## Fichiers que TU modifies (créés au 1er lancement)

Dossier : `%APPDATA%/fr.jbx.app` (Windows), `~/Library/Application Support/fr.jbx.app` (macOS), `~/.config/fr.jbx.app` (Linux). Le chemin exact est affiché dans l'appli.

- `config.toml` : tout (dossier du jeu, serveurs, serveur local, types de fichiers à patcher, dépôt de restauration). Modifiable dans l'onglet Configuration.
- `langs/<code>.toml` : textes de l'interface. Pour une nouvelle langue : copie `fr.toml` en `de.toml`, traduis, mets `language = "de"`.
- `catalog.toml` : liste des serveurs de patchs (URLs `info.json`, reprise de `servers.json`). Ajoute une ligne pour un nouveau serveur.
- `[pack_dirs]` dans `config.toml` : dossier de chaque pack (`jpp7 = "..."`), sinon `game_dir`.
- `sources/<code>.toml` : dépôts GitHub à télécharger pour cette langue (`[[repo]]` avec `name`, `url`, `branch`). Pour ajouter un dépôt, ajoute un bloc.

## Comment ça marche

**Changer de serveur** : choisis un serveur de la liste (`[[server.list]]`) ou tape une adresse. Le programme parcourt `game_dir`, et dans chaque `jbg.config.jet` remplace la ligne `joinUrl: ...` par `joinUrl: "adresse",`, ou l'ajoute avant la dernière `}` si elle n'existe pas.

**Serveur local** : 1) `git clone https://github.com/JACKBOX-FR/jackbox-fr-main-dump` ; 2) mets son chemin dans `local.dump_dir` ; 3) bouton Démarrer. L'appli affiche `IP:port` et la place dans le champ d'adresse : clique Appliquer pour l'écrire dans `joinUrl`. `bind = "0.0.0.0"` rend le serveur accessible aux autres appareils du réseau (autorise le port dans ton pare-feu) ; `127.0.0.1` = cette machine seulement.

**Patch** : télécharge `<url>/archive/refs/heads/<branch>.zip` de chaque dépôt de `sources/<langue>.toml`, ne garde que les types activés dans `[patch]` (`audio` = .ogg, `subtitles` = .swf + .json, `text` = .jet, `images` = .png/.jpg, `other` = le reste), filtre par `games` si renseigné, et copie dans `game_dir` en gardant l'arborescence du dépôt.

**Catalogue** : choisis un serveur, clique Charger. L'appli lit `info.json` (dernière entrée `urls`), puis `<api>/packs.json`, et liste packs, jeux et patchs. Le bouton ⬇ télécharge `<assets>/<patch_path>` (ou chaque `patch_paths`), l'extrait dans le dossier du pack (ou du jeu pour un patch de jeu) avec les filtres `[patch]`, puis exécute l'`install_controller` s'il y en a un (`tmp3`).

**Dump jackbox.tv en local** : `dump_dir` (relatif = dossier de config) contient le site statique, exactement comme avec `http-server -p 8000`. Le bouton **Dump** télécharge ou met à jour la copie depuis `dump_repo` (par défaut `JACKBOX-FR/jackbox-fr-main-dump`, ~600 Mo). Démarrer → l'appli affiche `localhost:8000` (solo) et `IP:8000` (réseau) : l'un des deux va dans `joinUrl` (bouton Appliquer). Pour une adresse sans port, mets `port = 80` (droits administrateur requis sous Linux/macOS).

**Traduction du contrôleur** : le dump reste intact ; `translations/<langue>.toml` (langue = `dump_language`) est appliqué à la volée aux fichiers js/html/json/css servis (cache mémoire). Format : `"\"texte anglais\"" = "\"texte français\""` (guillemets compris pour viser le texte entier ; valeur vide = ignorée). Pour traduire un jeu : saisis son fichier (ex. `main/pp7/everyday/script.js`), clique **Extraire** → `translations/template.<fichier>.toml` liste ses textes ; copie les lignes traduites dans `fr.toml`. Une mise à jour du dump ne casse donc pas tes traductions. État : la connexion et les packs 9–11 sont déjà en français dans le dump ; `fr.toml` ajoute 12 messages communs aux 72 contrôleurs (déconnexion, « Merci d'avoir joué », menu des épisodes…). Les textes propres à chaque jeu (packs 1–8 surtout) restent à traduire.

**joinUrl** : pour chaque `jbg.config.jet` (dans `game_dir`, `[pack_dirs]` et les packs détectés), si une propriété `joinUrl` existe sa valeur est remplacée ; sinon la propriété est **ajoutée juste après la première `{`** (jamais de virgule en trop). Gère fichiers multi-lignes ou minifiés, clés avec ou sans guillemets, CRLF et BOM. L'adresse est validée (`http(s)://` et `/` final retirés ; espaces, guillemets refusés). Un nom inconnu n'est plus pris pour une adresse. Les erreurs par fichier sont listées sans bloquer les autres.

**Détection et lancement** : « Détecter » cherche les packs du serveur dans les bibliothèques Steam et Epic et mémorise leur dossier dans `state.json` (un `[pack_dirs]` manuel est prioritaire). ▶ lance le pack ou un jeu : Steam via `steam://run/…`, Epic via `com.epicgames.launcher://…`, sinon l'exécutable du pack. Si le jeu utilise un loader, il est téléchargé/extrait avant. En ligne de commande : `jbx launch pack <id>` ou `jbx launch game <id>` (serveur = dernier utilisé).

**Remise à zéro** : même mécanique avec `[restore]` (repo + branch), tous types et tous jeux. Laisse `repo = ""` tant que le dépôt n'existe pas.

## Compiler

Prérequis : Node 20+, Rust stable, et sous Linux `libwebkit2gtk-4.1-dev` (+ voir doc Tauri v2).

    npm install
    npx tauri icon app-icon.png     # génère src-tauri/icons
    npx tauri dev                   # test
    npx tauri build                 # installeur pour ton OS

**Les 3 OS d'un coup** : pousse le projet sur GitHub, puis `git tag v0.1.0 && git push --tags` (ou lance le workflow à la main). La release brouillon contiendra `.msi`/`.exe` (Windows), `.dmg` (macOS), `.deb`/`.AppImage` (Linux). Les builds macOS/Windows ne sont pas signés (avertissement du système au 1er lancement).

## Étendre

Ajouter une fonction = écrire une `#[tauri::command]` dans `main.rs`, l'ajouter à `generate_handler!`, l'appeler depuis `ui/index.html` avec `inv('nom', {args})`. Les prochains portages (versions installées, détection Steam/Epic, lanceurs) iront dans de nouveaux modules `src-tauri/src/*.rs`.


## API REST (extensions navigateur)

Active par défaut sur `127.0.0.1:6480` (`[api]` dans `config.toml`). Flux : `POST /api/register` avec `{"name":"Mon extension","scopes":["navigation","game_open_close"]}` ouvre une fenêtre d'acceptation dans l'appli ; si acceptée, la réponse contient un `token` (valable jusqu'à la fermeture de l'appli). Ensuite, envoyer `Authorization: <token>` :

| Route | Scope | Rôle |
|---|---|---|
| `GET /api/status` | aucun | version / état |
| `GET /api/games/list` | `navigation` | packs, jeux, patchs du dernier serveur utilisé |
| `POST /api/games/open/<id_du_jeu>` | `game_open_close` | lance le jeu |
| `WS /ws` | `navigation` | envoie `{"token":"..."}` → `{"status":"ok"}`, puis `{"channel":"game_open"\|"game_close"\|"game_page_open","data":{jeu}}` |

`game_open` / `game_close` : détectés en surveillant le processus du pack (toutes les 5 s, comme l'appli d'origine). `game_page_open` : quand tu cliques sur le nom d'un jeu dans l'appli.

## Sécurité et robustesse (revue du code)

- `dump_dir` relatif : résolu dans le dossier de config (et non le dossier courant, qui change selon la façon de lancer l'appli).
- Détection du pack lancé : `tasklist` tronque les noms à 25 caractères, on compare donc sur 25 caractères (sinon « The Jackbox Party Pack 7.exe » n'était jamais reconnu).
- Interface : tout texte venant d'un serveur de patchs est échappé et la CSP interdit les scripts inline (un serveur malveillant ne peut pas injecter de code).
- Zips : chemins `../` bloqués, téléchargement sur disque (pas en mémoire), bit exécutable conservé sous Linux/macOS, timeouts réseau, refus d'écrire dans un dossier vide (`game_dir = ""`).
- `state.json` : écriture atomique + verrou (pas de corruption si deux actions simultanées).
- Serveur local : erreur claire si le port est pris ou `dump_dir` introuvable, CORS ouvert pour les pages de jeu, redirection UPnP retirée à l'arrêt et à la fermeture de l'appli.
- Patchs : plateformes non supportées refusées (défaut de l'appli d'origine : Windows et Linux).
- Non résolu : je n'ai pas pu lire les issues ouvertes du dépôt d'origine (GitHub bloque la requête depuis mon environnement) ; la revue repose sur le code et le CHANGELOG.


## Trois façons de jouer (au choix)

| Mode | Manette (téléphones) | Serveur de jeu (ecast) | Internet |
|---|---|---|---|
| **Officiel** | jackbox.tv | Jackbox | oui |
| **Dump local** (`local`, `fr-labo`) | ton dump (manette FR) | Jackbox officiel | oui (pour ecast) |
| **Serveur complet** (`jonahbox`) | ton dump, converti en cache | **ton PC** (Jonahbox) | **non** |

Choisir un serveur dans la liste réécrit `joinUrl` **et** `serverUrl` (si l'entrée a un `server_url`) dans tous les `jbg.config.jet` : changer de mode remet donc `serverUrl` à `ecast.jackboxgames.com` ou à `jonahbox.local`. Une adresse libre ne modifie que `joinUrl`.

## Serveur 100 % local, sans internet (Jonahbox)

[Jonahbox](https://github.com/JACKBOX-FR/Jonahbox-Dump-Ecast-Blobcast) est un serveur privé en Rust (ecast + blobcast) qui remplace les serveurs de Jackbox ; il faut posséder les jeux. L'appli automatise son installation, **chaque étape est un bouton, dans l'ordre** (section « Serveur complet hors-ligne », avec l'état ✅/❌ de chacune) :

1. **Télécharger** le code source (`[jonahbox].repo`).
2. **Compiler** avec `cargo` (plusieurs minutes ; Rust requis). Ou renseigne `[jonahbox].binary` si tu as déjà un exécutable.
3. **Cache (dump)** : convertit ton dump jackbox.tv en cache Jonahbox (≈ 7 300 fichiers, 480 Mo, ~40 s), applique `translations/<langue>.toml` et remplace `ecast/blobcast.jackboxgames.com` par `[jonahbox].host`. Aucun Python nécessaire. N'écrase pas un cache existant.
4. **Certificats** : crée une autorité locale + un certificat valable 800 jours pour l'IP, `jonahbox.local`, `localhost` et les noms blobcast (`certs/ca.pem`, `server.pem`).
5. **Faire confiance** : installe l'autorité dans le système (`certutil` sous Windows, `security` sous macOS ; **droits administrateur** requis ; sous Linux la commande à lancer est affichée). Sur les **téléphones**, installe `certs/ca.pem` à la main.
6. **Fichier hosts** : ajoute `IP jonahbox.local # jbx` (administrateur requis). Les lignes ajoutées sont marquées et peuvent être retirées d'un clic. Sur les téléphones, il faut que `jonahbox.local` pointe vers l'IP du PC (DNS local de la box ou appli « hosts »).
7. **Démarrer** : écrit le `config.toml` de Jonahbox (ports 443, 80, 38203) et lance le serveur ; **Logs** affiche `jonahbox.log`.
8. **Appliquer aux jeux** : `joinUrl` et `serverUrl` = `jonahbox.local` (sans port).

**Pare-feu** : le bouton affiche les commandes (`netsh …`, redirections IPv4→IPv6 requises sous Windows) à lancer toi-même en administrateur ; l'appli ne les exécute pas. N'ouvre rien sur ta box : tout reste sur le réseau local.

Limites (reprises du README de Jonahbox) : testé par l'auteur sur les Party Packs 2 à 10 ; **Party Pack 11 et plus récents non testés**, Party Pack 1 expérimental. Les fichiers du dump viennent d'une version de la manette qui peut différer de l'actuelle ; en cas de page blanche, supprime `jonahbox/jb_cache` et laisse Jonahbox remplir le cache (`cache_mode = "oneshot"`, une partie par jeu). Compilation testée par Jonahbox sous Windows seulement. Pour couper internet une fois le cache complet : `cache_mode = "offline"`.

## Licence

Jonahbox est sous **AGPL-3.0-or-later** et `jb_cache.rs` est un port de son script `dump_to_cache.py` : si tu distribues ce projet, il doit donc être publié sous AGPL-3.0-or-later (ajoute un fichier `LICENSE`). L'appli ne contient pas le code de Jonahbox : elle le télécharge et le lance comme programme séparé.
