# API REST + WebSocket (extensions de navigateur)

Active par défaut sur `127.0.0.1:6480` (`[api]` dans `config.toml`). Flux : `POST /api/register` avec `{"name":"Mon extension","scopes":["navigation","game_open_close"]}` ouvre une fenêtre d'acceptation dans l'appli ; si acceptée, la réponse contient un `token` (valable jusqu'à la fermeture de l'appli). Ensuite, envoyer `Authorization: <token>` :

| Route | Scope | Rôle |
|---|---|---|
| `GET /api/status` | aucun | version / état |
| `GET /api/games/list` | `navigation` | packs, jeux, patchs du dernier serveur utilisé |
| `POST /api/games/open/<id_du_jeu>` | `game_open_close` | lance le jeu |
| `WS /ws` | `navigation` | envoie `{"token":"..."}` → `{"status":"ok"}`, puis `{"channel":"game_open"\|"game_close"\|"game_page_open","data":{jeu}}` |

`game_open` / `game_close` : détectés en surveillant le processus du pack (toutes les 5 s, comme l'appli d'origine). `game_page_open` : quand tu cliques sur le nom d'un jeu dans l'appli.
