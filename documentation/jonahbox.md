# Serveur 100 % local, sans internet (Jonahbox)

[Jonahbox](https://github.com/JACKBOX-FR/Jonahbox-Dump-Ecast-Blobcast) est un serveur privé en Rust (ecast + blobcast) qui remplace les serveurs de Jackbox ; il faut posséder les jeux. L'appli automatise son installation, **chaque étape est un bouton, dans l'ordre** (Paramètres → Réseau et serveurs → « Serveur complet hors-ligne », avec l'état ✅/❌ de chacune) :

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
