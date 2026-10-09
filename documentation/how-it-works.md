# Comment ça marche

## Dossier de configuration (créé au premier lancement)

`%APPDATA%/fr.jbx.app` (Windows), `~/Library/Application Support/fr.jbx.app` (macOS), `~/.config/fr.jbx.app` (Linux). Le chemin exact est affiché dans Paramètres → Informations.

| Fichier | Rôle |
|---|---|
| `config.toml` | Réglages : dossier du jeu, serveurs de jeu, serveur local, types de fichiers à patcher, Jonahbox, API. Les interrupteurs de l'appli modifient une seule valeur et gardent tes commentaires. |
| `catalog.toml` | Liste des serveurs de patchs (URL `info.json`), complétée par la liste officielle (`servers.json` du dépôt d'origine). |
| `sources/<langue>.toml` | Dépôts GitHub à télécharger pour cette langue (bouton « Installer » de la config avancée). |
| `translations/<langue>.toml` | Traduction du contrôleur jackbox.tv servi en local ou converti pour Jonahbox. |
| `state.json` | Packs détectés, patchs installés (versions), loaders, dernier serveur. |
| `cache/` | Copie locale des serveurs de patchs (affichage immédiat, fonctionne hors-ligne). |

Les textes de l'interface viennent de `jackbox_patcher/lib/assets/l10n/app_<langue>.arb` (fichiers de l'appli d'origine, 14 langues) et de `extra_<langue>.arb` (ajouts de cette version, repli sur l'anglais). Pour corriger ou ajouter une langue : modifie ou copie un de ces fichiers.

## Changer de serveur de jeu

Paramètres → Réseau et serveurs. Pour chaque `jbg.config.jet` (dans `game_dir`, `[pack_dirs]` et les packs détectés), la valeur de `joinUrl` est remplacée, ou la propriété est ajoutée juste après la première `{` si elle n'existe pas (jamais de virgule en trop). Fichiers multi-lignes ou minifiés, clés avec ou sans guillemets, CRLF et BOM sont gérés. L'adresse est validée (`http(s)://` et `/` final retirés ; espaces et guillemets refusés). Si l'entrée de `config.toml` définit `server_url`, `serverUrl` est aussi réécrit : choisir `officiel` remet donc `ecast.jackboxgames.com`, `jonahbox` met `jonahbox.local`. Une adresse libre ne modifie que `joinUrl`.

## Site local (dump jackbox.tv)

Le dump est un site statique, servi comme avec `http-server -p 8000`. « Télécharger / mettre à jour le dump » le récupère depuis `local.dump_repo` (`dump_dir` relatif = dossier de config). Démarrer affiche `localhost:8000` (solo) et `IP:8000` (réseau) ; un clic écrit l'adresse choisie dans `joinUrl`. Pour une adresse sans port : `port = 80` (droits administrateur sous Linux/macOS).

**Traduction du contrôleur** : le dump reste intact ; `translations/<langue>.toml` (langue = `local.dump_language`) est appliqué à la volée aux fichiers js/html/json/css servis. Format : `"\"texte anglais\"" = "\"texte français\""` (guillemets compris pour viser le texte entier ; valeur vide = ignorée). Pour traduire un jeu : saisis son fichier (ex. `main/pp7/everyday/script.js`) puis « Extraire les textes » → `translations/template.<fichier>.toml`. Une mise à jour du dump ne casse pas tes traductions. État : la connexion et les packs 9–11 sont déjà en français dans le dump ; `fr.toml` ajoute 12 messages communs aux 72 contrôleurs. Les textes propres à chaque jeu (packs 1–8 surtout) restent à traduire.

## Patchs

Accueil → Patcher un jeu. Le panneau de gauche liste « Tous les patchs » et chaque pack possédé. Un patch se télécharge sur disque (barre de progression), s'extrait dans le dossier du pack (ou du jeu) en respectant les types choisis (Paramètres → Comportement : audio, sous-titres, texte, images, autres), lance son `install_controller` (`tmp3`) puis mémorise sa version (⟳ = mise à jour disponible). Les plateformes non supportées par le patch sont refusées (défaut de l'appli d'origine : Windows et Linux). Les patchs peuvent être réinstallés par-dessus : pas besoin de tout retélécharger.

## Détection et lancement

Paramètres → Packs possédés → « Détecter automatiquement » cherche les packs du serveur dans les bibliothèques Steam et Epic ; un dossier peut aussi être choisi à la main (sélecteur natif). ▶ lance un pack ou un jeu : Steam via `steam://run/…`, Epic via `com.epicgames.launcher://…`, sinon l'exécutable du pack ; si le jeu utilise un loader, il est téléchargé/extrait avant. En ligne de commande : `jbx launch pack <id>` ou `jbx launch game <id>` (serveur = dernier utilisé).
