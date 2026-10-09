# Trois façons de jouer (au choix)

| Mode | Manette (téléphones) | Serveur de jeu (ecast) | Internet |
|---|---|---|---|
| **Officiel** | jackbox.tv | Jackbox | oui |
| **Dump local** (`local`, `fr-labo`) | ton dump (manette FR) | Jackbox officiel | oui (pour ecast) |
| **Serveur complet** (`jonahbox`) | ton dump, converti en cache | **ton PC** (Jonahbox) | **non** |

Choisir un serveur dans la liste réécrit `joinUrl` **et** `serverUrl` (si l'entrée a un `server_url`) dans tous les `jbg.config.jet` : changer de mode remet donc `serverUrl` à `ecast.jackboxgames.com` ou à `jonahbox.local`. Une adresse libre ne modifie que `joinUrl`.
