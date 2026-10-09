# Sécurité, robustesse et performances

- `dump_dir` relatif : résolu dans le dossier de config (et non le dossier courant, qui change selon la façon de lancer l'appli).
- Détection du pack lancé : `tasklist` tronque les noms à 25 caractères, on compare donc sur 25 caractères (sinon « The Jackbox Party Pack 7.exe » n'était jamais reconnu).
- Interface : tout texte venant d'un serveur de patchs est échappé et la CSP interdit les scripts inline (un serveur malveillant ne peut pas injecter de code).
- Zips : chemins `../` bloqués, téléchargement sur disque (pas en mémoire), bit exécutable conservé sous Linux/macOS, timeouts réseau, refus d'écrire dans un dossier vide (`game_dir = ""`).
- `state.json` : écriture atomique + verrou (pas de corruption si deux actions simultanées).
- Serveur local : erreur claire si le port est pris ou `dump_dir` introuvable, CORS ouvert pour les pages de jeu, redirection UPnP retirée à l'arrêt et à la fermeture de l'appli.
- Patchs : plateformes non supportées refusées (défaut de l'appli d'origine : Windows et Linux).
- Non résolu : je n'ai pas pu lire les issues ouvertes du dépôt d'origine (GitHub bloque la requête depuis mon environnement) ; la revue repose sur le code et le CHANGELOG.
- **Performances** : toute commande qui touche au disque ou au réseau s'exécute hors du thread de la fenêtre (`#[tauri::command(async)]` ou `spawn_blocking`). Une commande synchrone classique de Tauri tourne sur le thread de la fenêtre et la fige : c'était la cause des lenteurs de la première version (parcours du dossier du jeu à chaque changement de serveur, lectures disque). Les données du serveur s'affichent depuis le cache local immédiatement, puis sont rafraîchies en arrière-plan. En mode `tauri dev`, les dépendances sont compilées en optimisé (`[profile.dev.package."*"]`).
