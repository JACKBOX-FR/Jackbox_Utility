#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_configuration;
mod model;
mod services;

use model::{config::{Config, ServerCfg}, state};
use services::{api_utility as catalog, automatic_game_finder as finder, downloader as patch, files as switch, i18n, internal_api as api, jonahbox, launcher, local_server as local, translations};
use std::{collections::HashMap, fs, path::PathBuf, sync::{Arc, Mutex}};
use tauri::{Emitter, Manager, State};

struct App { dir: PathBuf, server: Mutex<Option<local::Running>>, api: Arc<api::Api>, jonah: Mutex<Option<std::process::Child>> }

// Fichiers copiés au 1er lancement dans le dossier de config (puis modifiables à la main)
const DEFAULTS: &[(&str, &str)] = &[
    ("config.toml", include_str!("../defaults/config.toml")),
    ("langs/fr.toml", include_str!("../defaults/langs/fr.toml")),
    ("langs/en.toml", include_str!("../defaults/langs/en.toml")),
    ("sources/fr.toml", include_str!("../defaults/sources/fr.toml")),
    ("translations/fr.toml", include_str!("../defaults/translations/fr.toml")),
    ("catalog.toml", include_str!("../defaults/catalog.toml")),
    ("sources/en.toml", include_str!("../defaults/sources/en.toml")),
];

fn e<T: ToString>(x: T) -> String { x.to_string() }
fn cfg(a: &App) -> Result<Config, String> { Config::load(&a.dir).map_err(e) }
fn tr(a: &App, c: &Config) -> i18n::I18n { i18n::I18n::load(&a.dir, &c.language) }
/// Exécute du travail bloquant (disque, réseau) hors du thread de la fenêtre.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(e)?.map_err(e)
}

// NB : `#[tauri::command(async)]` = exécuté sur un thread à part. Une commande synchrone classique tourne sur le thread de la
// fenêtre et la fige (c'était la cause des lenteurs : parcours du dossier du jeu, lectures disque…).

// ---- Configuration ----
#[tauri::command] fn config_dir(a: State<App>) -> String { a.dir.display().to_string() }
#[tauri::command(async)] fn app_info() -> serde_json::Value {
    serde_json::json!({ "version": env!("CARGO_PKG_VERSION"), "github": app_configuration::GITHUB_URL, "discord": app_configuration::DISCORD_URL })
}
#[tauri::command(async)] fn get_config(a: State<App>) -> Result<String, String> { fs::read_to_string(a.dir.join("config.toml")).map_err(e) }
#[tauri::command(async)] fn get_config_json(a: State<App>) -> Result<serde_json::Value, String> { serde_json::to_value(cfg(&a)?).map_err(e) }
#[tauri::command(async)] fn save_config(a: State<App>, text: String) -> Result<(), String> {
    toml::from_str::<Config>(&text).map_err(e)?; fs::write(a.dir.join("config.toml"), text).map_err(e)
}
/// Modifie une seule valeur (ex. "patch.audio", "language") en gardant les commentaires du config.toml.
#[tauri::command(async)] fn set_config_value(a: State<App>, key: String, value: serde_json::Value) -> Result<(), String> {
    use serde_json::Value as J;
    let path = a.dir.join("config.toml");
    let mut doc: toml_edit::DocumentMut = fs::read_to_string(&path).map_err(e)?.parse().map_err(e)?;
    let item = match &value {
        J::Bool(b) => toml_edit::value(*b),
        J::String(s) => toml_edit::value(s.as_str()),
        J::Number(n) => match n.as_i64() { Some(i) => toml_edit::value(i), None => toml_edit::value(n.as_f64().unwrap_or(0.0)) },
        J::Array(v) => { let mut arr = toml_edit::Array::new(); for x in v { if let Some(s) = x.as_str() { arr.push(s); } } toml_edit::value(arr) }
        _ => return Err("valeur non gérée".into()),
    };
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        [k] => doc[*k] = item,
        [t, k] => doc[*t][*k] = item,
        _ => return Err("clé invalide".into()),
    }
    let text = doc.to_string();
    toml::from_str::<Config>(&text).map_err(e)?; // on n'écrit jamais un fichier invalide
    fs::write(path, text).map_err(e)
}
#[tauri::command(async)] fn strings(a: State<App>) -> Result<HashMap<String, String>, String> { let c = cfg(&a)?; Ok(tr(&a, &c).0) }
#[tauri::command(async)] fn servers(a: State<App>) -> Result<ServerCfg, String> { Ok(cfg(&a)?.server) }

// ---- Serveur de jeu (joinUrl / serverUrl dans les jbg.config.jet) ----
#[tauri::command] async fn switch_server(a: State<'_, App>, target: String) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c);
    // "jonahbox" prend toujours l'hôte de [jonahbox].host ; une adresse libre ne modifie que joinUrl
    let (raw, srv_raw): (String, Option<String>) = if target == "jonahbox" { (c.jonahbox.host.clone(), Some(c.jonahbox.host.clone())) }
        else if let Some(s) = c.server.list.iter().find(|s| s.name == target) { (s.join_url.clone(), s.server_url.clone()) }
        else { (target.clone(), None) };
    let url = switch::clean_url(&raw).map_err(|_| t.t("unknown_server", &[&target]))?; // refuse un nom inconnu / une adresse invalide
    let srv = srv_raw.map(|x| switch::clean_url(&x)).transpose().map_err(e)?;
    let dirs = state::all_dirs(&state::State::load(&a.dir), &c);
    if dirs.is_empty() { return Err("aucun dossier de jeu : renseigne game_dir ou clique Détecter".into()); }
    blocking(move || {
        let mut kv = vec![("joinUrl", url.as_str())];
        if let Some(s) = &srv { kv.push(("serverUrl", s.as_str())); }
        let mut o = switch::Outcome::default();
        for d in dirs { let r = switch::set_keys(&d, &kv); o.changed += r.changed; o.errors.extend(r.errors); }
        let mut msg = t.t("switched", &[&url, &o.changed.to_string()]);
        if !o.errors.is_empty() { msg += &format!(" — {} erreur(s) : {}", o.errors.len(), o.errors[0]); }
        Ok(msg)
    }).await
}

// ---- Dépôts de patch GitHub (config.toml / sources) ----
#[tauri::command] async fn patch_install(a: State<'_, App>) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c);
    let repos = c.sources(&a.dir).map_err(e)?.repo;
    let n = blocking(move || { let mut total = 0; for r in repos.iter().filter(|r| !r.url.is_empty()) { total += patch::install(&c, r)?; } Ok(total) }).await?;
    Ok(t.t("patched", &[&n.to_string()]))
}
#[tauri::command] async fn restore(a: State<'_, App>) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c);
    let r = blocking(move || patch::restore(&c)).await?;
    Ok(t.t(if r.is_some() { "restored" } else { "no_restore" }, &[]))
}

// ---- Serveur local (dump jackbox.tv) ----
#[tauri::command] async fn start_local(a: State<'_, App>) -> Result<local::LocalInfo, String> {
    let c = cfg(&a)?;
    if a.server.lock().unwrap().is_some() { return Err("already running".into()); }
    let (r, info) = local::start(&c, &a.dir).await.map_err(e)?;
    *a.server.lock().unwrap() = Some(r);
    Ok(info) // info.lan / info.external = adresse à mettre dans joinUrl
}
#[tauri::command] fn stop_local(a: State<App>) { if let Some(r) = a.server.lock().unwrap().take() { local::stop(r, false); } }
#[tauri::command] async fn update_dump(a: State<'_, App>) -> Result<String, String> {
    let (c, dir) = (cfg(&a)?, a.dir.clone());
    blocking(move || local::update_dump(&c, &dir).map(|n| format!("dump : {n} fichiers"))).await
}
/// Prépare une traduction : liste les textes d'un fichier du dump (ex. main/pp7/everyday/script.js) dans translations/template.<nom>.toml
#[tauri::command] async fn extract_strings(a: State<'_, App>, file: String) -> Result<String, String> {
    let (c, dir) = (cfg(&a)?, a.dir.clone());
    blocking(move || {
        let rel = std::path::Path::new(file.trim());
        if rel.is_absolute() || rel.components().any(|x| matches!(x, std::path::Component::ParentDir)) { anyhow::bail!("chemin invalide"); }
        let js = fs::read_to_string(local::dump_path(&c, &dir).join(rel))?;
        let lines = translations::extract(&js);
        let name: String = file.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect();
        let out = dir.join("translations").join(format!("template.{name}.toml"));
        fs::create_dir_all(out.parent().unwrap())?;
        fs::write(&out, lines.join("\n"))?;
        Ok(format!("{} textes -> {}", lines.len(), out.display()))
    }).await
}

// ---- Liste des serveurs de patchs ----
fn catalog_file(a: &App) -> PathBuf { a.dir.join("catalog.toml") }
fn read_urls(a: &App) -> Result<Vec<String>, String> {
    #[derive(serde::Deserialize)] struct C { urls: Vec<String> }
    Ok(toml::from_str::<C>(&fs::read_to_string(catalog_file(a)).map_err(e)?).map_err(e)?.urls)
}
#[tauri::command(async)] fn catalog_urls(a: State<App>) -> Result<Vec<String>, String> { read_urls(&a) }
/// Ajoute un serveur personnalisé (info.json) à catalog.toml.
#[tauri::command(async)] fn add_catalog_url(a: State<App>, url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    if !(url.starts_with("https://") || url.starts_with("http://")) { return Err("l'adresse doit commencer par http:// ou https://".into()); }
    let mut doc: toml_edit::DocumentMut = fs::read_to_string(catalog_file(&a)).map_err(e)?.parse().map_err(e)?;
    let arr = doc["urls"].as_array_mut().ok_or("catalog.toml : `urls` manquant")?;
    if !arr.iter().any(|x| x.as_str() == Some(&url)) { arr.push(url); }
    fs::write(catalog_file(&a), doc.to_string()).map_err(e)
}
#[tauri::command(async)] fn remove_catalog_url(a: State<App>, url: String) -> Result<(), String> {
    let mut doc: toml_edit::DocumentMut = fs::read_to_string(catalog_file(&a)).map_err(e)?.parse().map_err(e)?;
    let arr = doc["urls"].as_array_mut().ok_or("catalog.toml : `urls` manquant")?;
    arr.retain(|x| x.as_str() != Some(&url));
    fs::write(catalog_file(&a), doc.to_string()).map_err(e)
}
/// Liste officielle des serveurs (servers.json du dépôt d'origine), pour compléter la liste locale.
#[tauri::command] async fn fetch_main_servers() -> Result<Vec<String>, String> {
    blocking(|| Ok(serde_json::from_value(catalog::get_json_t(app_configuration::MAIN_SERVER_URL, 8)?)?)).await
}
#[tauri::command(async)] fn get_selected_server(a: State<App>) -> Result<Option<String>, String> { Ok(state::State::load(&a.dir).last_server) }
#[tauri::command(async)] fn set_selected_server(a: State<App>, url: Option<String>) -> Result<(), String> { state::update(&a.dir, |s| s.last_server = url).map_err(e) }

// ---- Catalogue : données du serveur sélectionné (cache d'abord = affichage immédiat) ----
#[tauri::command] async fn server_info(a: State<'_, App>, url: String, refresh: bool) -> Result<catalog::ServerView, String> {
    let dir = a.dir.clone(); blocking(move || catalog::info(&dir, &url, refresh)).await
}
#[tauri::command] async fn server_catalog(a: State<'_, App>, url: String, refresh: bool) -> Result<serde_json::Value, String> {
    let dir = a.dir.clone(); blocking(move || catalog::raw(&dir, &url, refresh)).await
}
#[tauri::command] async fn server_news(a: State<'_, App>, url: String) -> Result<serde_json::Value, String> {
    let dir = a.dir.clone();
    let (assets, news) = blocking(move || catalog::news(&dir, &url)).await?;
    Ok(serde_json::json!({ "assets": assets, "news": news }))
}
#[tauri::command] async fn set_pack_path(a: State<'_, App>, url: String, pack: String, path: String) -> Result<String, String> {
    let dir = a.dir.clone(); blocking(move || catalog::set_path(&dir, &url, &pack, &path)).await
}
#[tauri::command] async fn detect_games(a: State<'_, App>, url: String) -> Result<Vec<finder::Found>, String> {
    let dir = a.dir.clone(); blocking(move || catalog::detect(&dir, &url)).await
}
/// Retire la version mémorisée d'un patch (jeu réinitialisé depuis Steam, par ex.).
#[tauri::command(async)] fn forget_patch(a: State<App>, pack: String, patch: String) -> Result<(), String> {
    state::update(&a.dir, |s| { if let Some(p) = s.packs.get_mut(&pack) { p.patches.remove(&patch); } }).map_err(e)
}
#[tauri::command] async fn launch(a: State<'_, App>, url: String, pack: String, game: Option<String>) -> Result<(), String> {
    let (c, dir, api) = (cfg(&a)?, a.dir.clone(), a.api.clone());
    let l = blocking(move || launcher::launch(&dir, &c, &url, &pack, game.as_deref())).await?;
    api.watch(l); // événements game_open / game_close pour les extensions
    Ok(())
}
/// L'utilisateur ouvre la page d'un jeu dans l'appli -> événement game_page_open
#[tauri::command] fn notify_page_open(a: State<App>, pack: String, id: String, name: String) {
    a.api.push("game_page_open", serde_json::json!({ "pack_id": pack, "id": id, "name": name }));
}
/// Installe un patch ; émet l'événement `patch-progress` {stage, percent} (starting/downloading/extracting/finalizing).
#[tauri::command] async fn install_server_patch(app: tauri::AppHandle, a: State<'_, App>, url: String, pack: String, patch: String, game: Option<String>) -> Result<String, String> {
    let (c, dir) = (cfg(&a)?, a.dir.clone());
    let n = blocking(move || catalog::install(&dir, &c, &url, &pack, &patch, game.as_deref(), &|stage, p| {
        let _ = app.emit("patch-progress", serde_json::json!({ "stage": stage, "percent": p }));
    })).await?;
    Ok(n.to_string())
}

// ---- Serveur complet hors-ligne (Jonahbox) ----
#[tauri::command(async)] fn jb_status(a: State<App>) -> Result<serde_json::Value, String> {
    let c = cfg(&a)?;
    let running = { let mut g = a.jonah.lock().unwrap(); match g.as_mut() { Some(ch) => ch.try_wait().map(|s| s.is_none()).unwrap_or(false), None => false } };
    let mut v = serde_json::to_value(jonahbox::status(&c, &a.dir)).map_err(e)?;
    v["running"] = running.into();
    Ok(v)
}
#[tauri::command] async fn jb_install(a: State<'_, App>) -> Result<String, String> { let (c, d) = (cfg(&a)?, a.dir.clone()); blocking(move || jonahbox::install(&c, &d).map(|n| format!("{n} fichiers"))).await }
#[tauri::command] async fn jb_build(a: State<'_, App>) -> Result<String, String> { let (c, d) = (cfg(&a)?, a.dir.clone()); blocking(move || jonahbox::build(&c, &d)).await }
#[tauri::command] async fn jb_cache(a: State<'_, App>, force: bool) -> Result<String, String> { let (c, d) = (cfg(&a)?, a.dir.clone()); blocking(move || jonahbox::cache(&c, &d, force)).await }
#[tauri::command] async fn jb_certs(a: State<'_, App>) -> Result<String, String> { let (c, d) = (cfg(&a)?, a.dir.clone()); blocking(move || jonahbox::certs(&c, &d)).await }
#[tauri::command] async fn jb_trust(a: State<'_, App>, on: bool) -> Result<String, String> { let (c, d) = (cfg(&a)?, a.dir.clone()); blocking(move || jonahbox::trust(&c, &d, on)).await }
#[tauri::command] async fn jb_hosts(a: State<'_, App>, on: bool, pp1: bool) -> Result<String, String> { let c = cfg(&a)?; blocking(move || jonahbox::hosts(&c, on, pp1)).await }
#[tauri::command(async)] fn jb_commands(a: State<App>) -> Result<String, String> { Ok(jonahbox::commands(&cfg(&a)?)) }
#[tauri::command(async)] fn jb_logs(a: State<App>) -> Result<String, String> { Ok(jonahbox::logs(&cfg(&a)?, &a.dir, 60)) }
#[tauri::command] async fn jb_start(a: State<'_, App>) -> Result<String, String> {
    let (c, d) = (cfg(&a)?, a.dir.clone());
    if a.jonah.lock().unwrap().as_mut().map_or(false, |ch| ch.try_wait().map(|s| s.is_none()).unwrap_or(false)) { return Err("déjà démarré".into()); }
    let child = blocking(move || jonahbox::start(&c, &d)).await?;
    *a.jonah.lock().unwrap() = Some(child);
    Ok("Jonahbox démarré (logs : bouton Logs)".into())
}
#[tauri::command] fn jb_stop(a: State<App>) { if let Some(mut c) = a.jonah.lock().unwrap().take() { let _ = c.kill(); let _ = c.wait(); } }

/// Ouvre un lien https dans le navigateur par défaut (liens GitHub / Discord / markdown des actualités).
#[tauri::command(async)] fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") { return Err("seuls les liens https:// sont ouverts".into()); }
    open::that(url).map_err(e)
}
/// Oublie le dossier d'un pack (bouton corbeille des jeux possédés).
#[tauri::command(async)] fn clear_pack_path(a: State<App>, pack: String) -> Result<(), String> {
    state::update(&a.dir, |s| { if let Some(p) = s.packs.get_mut(&pack) { p.path = None; p.launcher = None; } }).map_err(e)
}

// ---- API pour extensions navigateur ----
#[tauri::command] fn answer_extension(a: State<App>, id: u64, accept: bool) { a.api.answer(id, accept); }

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            for (name, content) in DEFAULTS {
                let p = dir.join(name);
                if !p.exists() { fs::create_dir_all(p.parent().unwrap())?; fs::write(p, content)?; }
            }
            let args: Vec<String> = std::env::args().skip(1).collect();
            if args.len() == 3 && args[0] == "launch" { // jbx launch pack|game <id>
                match Config::load(&dir).map_err(|x| x.to_string()).and_then(|c| launcher::launch_raw(&dir, &c, &args[1], &args[2]).map_err(|x| x.to_string())) {
                    Ok(_) => {}
                    Err(x) => eprintln!("{x}"),
                }
                app.handle().exit(0);
                return Ok(());
            }
            let api = api::Api::new(dir.clone(), app.handle().clone());
            if let Ok(c) = Config::load(&dir) { if c.api.enabled { tauri::async_runtime::spawn(api::serve(api.clone(), c.api.port)); } }
            app.manage(App { dir, server: Mutex::new(None), api, jonah: Mutex::new(None) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![config_dir, app_info, get_config, get_config_json, save_config, set_config_value, strings, servers,
            switch_server, patch_install, restore, start_local, stop_local, update_dump, extract_strings,
            catalog_urls, add_catalog_url, remove_catalog_url, fetch_main_servers, get_selected_server, set_selected_server,
            server_info, server_catalog, server_news, set_pack_path, detect_games, forget_patch, open_url, clear_pack_path, launch, notify_page_open, install_server_patch,
            jb_status, jb_install, jb_build, jb_cache, jb_certs, jb_trust, jb_hosts, jb_commands, jb_logs, jb_start, jb_stop, answer_extension])
        .build(tauri::generate_context!())
        .expect("erreur au lancement")
        .run(|app, ev| {
            // à la fermeture : arrête le serveur local / Jonahbox et retire la redirection UPnP
            if let tauri::RunEvent::Exit = ev {
                if let Some(a) = app.try_state::<App>() {
                    if let Some(r) = a.server.lock().unwrap().take() { local::stop(r, true); }
                    if let Some(mut c) = a.jonah.lock().unwrap().take() { let _ = c.kill(); let _ = c.wait(); }
                }
            }
        });
}
