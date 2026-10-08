#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod api; mod catalog; mod config; mod finder; mod i18n; mod jb_cache; mod jonahbox; mod launcher; mod local; mod patch; mod state; mod switch; mod tmp3; mod translations;
use config::{Config, ServerCfg};
use std::{collections::HashMap, fs, path::PathBuf, sync::{Arc, Mutex}};
use tauri::{Manager, State};

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

#[tauri::command] fn config_dir(a: State<App>) -> String { a.dir.display().to_string() }
#[tauri::command] fn get_config(a: State<App>) -> Result<String, String> { fs::read_to_string(a.dir.join("config.toml")).map_err(e) }
#[tauri::command] fn save_config(a: State<App>, text: String) -> Result<(), String> {
    toml::from_str::<Config>(&text).map_err(e)?; fs::write(a.dir.join("config.toml"), text).map_err(e)
}
#[tauri::command] fn strings(a: State<App>) -> Result<HashMap<String, String>, String> {
    let c = cfg(&a)?; Ok(tr(&a, &c).0)
}
#[tauri::command] fn servers(a: State<App>) -> Result<ServerCfg, String> { Ok(cfg(&a)?.server) }

#[tauri::command] fn switch_server(a: State<App>, target: String) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c);
    // "jonahbox" prend toujours l'hôte de [jonahbox].host ; une adresse libre ne modifie que joinUrl
    let (raw, srv_raw): (String, Option<String>) = if target == "jonahbox" { (c.jonahbox.host.clone(), Some(c.jonahbox.host.clone())) }
        else if let Some(s) = c.server.list.iter().find(|s| s.name == target) { (s.join_url.clone(), s.server_url.clone()) }
        else { (target.clone(), None) };
    let url = switch::clean_url(&raw).map_err(|_| t.t("unknown_server", &[&target]))?; // refuse un nom inconnu / une adresse invalide
    let srv = srv_raw.map(|x| switch::clean_url(&x)).transpose().map_err(e)?;
    let mut kv = vec![("joinUrl", url.as_str())];
    if let Some(s) = &srv { kv.push(("serverUrl", s.as_str())); }
    let dirs = state::all_dirs(&state::State::load(&a.dir), &c);
    if dirs.is_empty() { return Err("aucun dossier de jeu : renseigne game_dir ou clique Détecter".into()); }
    let mut o = switch::Outcome::default();
    for d in dirs { let r = switch::set_keys(&d, &kv); o.changed += r.changed; o.errors.extend(r.errors); }
    let mut msg = t.t("switched", &[&url, &o.changed.to_string()]);
    if !o.errors.is_empty() { msg += &format!(" — {} erreur(s) : {}", o.errors.len(), o.errors[0]); }
    Ok(msg)
}
#[tauri::command] async fn patch_install(a: State<'_, App>) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c);
    let repos = c.sources(&a.dir).map_err(e)?.repo;
    let n = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<usize> {
        let mut total = 0;
        for r in repos.iter().filter(|r| !r.url.is_empty()) { total += patch::install(&c, r)?; }
        Ok(total)
    }).await.map_err(e)?.map_err(e)?;
    Ok(t.t("patched", &[&n.to_string()]))
}
#[tauri::command] async fn restore(a: State<'_, App>) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c);
    let r = tauri::async_runtime::spawn_blocking(move || patch::restore(&c)).await.map_err(e)?.map_err(e)?;
    Ok(t.t(if r.is_some() { "restored" } else { "no_restore" }, &[]))
}
#[tauri::command] async fn start_local(a: State<'_, App>) -> Result<local::LocalInfo, String> {
    let c = cfg(&a)?;
    if a.server.lock().unwrap().is_some() { return Err("already running".into()); }
    let (r, info) = local::start(&c, &a.dir).await.map_err(e)?;
    *a.server.lock().unwrap() = Some(r);
    Ok(info) // info.lan / info.external = adresse à mettre dans joinUrl
}
#[tauri::command] fn stop_local(a: State<App>) {
    if let Some(r) = a.server.lock().unwrap().take() { local::stop(r, false); }
}

#[tauri::command] fn catalog_urls(a: State<App>) -> Result<Vec<String>, String> {
    #[derive(serde::Deserialize)] struct C { urls: Vec<String> }
    let c: C = toml::from_str(&fs::read_to_string(a.dir.join("catalog.toml")).map_err(e)?).map_err(e)?;
    Ok(c.urls)
}
#[tauri::command] async fn server_info(url: String) -> Result<catalog::ServerView, String> {
    tauri::async_runtime::spawn_blocking(move || catalog::get_json(&url).map(|i| catalog::view(&i)))
        .await.map_err(e)?.map_err(e)
}
#[tauri::command] async fn server_packs(a: State<'_, App>, url: String) -> Result<catalog::CatalogView, String> {
    let dir = a.dir.clone();
    tauri::async_runtime::spawn_blocking(move || catalog::view_packs(&dir, &url)).await.map_err(e)?.map_err(e)
}
#[tauri::command] async fn set_pack_path(a: State<'_, App>, url: String, pack: String, path: String) -> Result<String, String> {
    let dir = a.dir.clone();
    tauri::async_runtime::spawn_blocking(move || catalog::set_path(&dir, &url, &pack, &path)).await.map_err(e)?.map_err(e)
}
#[tauri::command] fn answer_extension(a: State<App>, id: u64, accept: bool) { a.api.answer(id, accept); }
#[tauri::command] async fn server_news(url: String) -> Result<serde_json::Value, String> {
    let (assets, news) = tauri::async_runtime::spawn_blocking(move || catalog::news(&url)).await.map_err(e)?.map_err(e)?;
    Ok(serde_json::json!({ "assets": assets, "news": news }))
}
#[tauri::command] async fn detect_games(a: State<'_, App>, url: String) -> Result<Vec<finder::Found>, String> {
    let dir = a.dir.clone();
    tauri::async_runtime::spawn_blocking(move || catalog::detect(&dir, &url)).await.map_err(e)?.map_err(e)
}
#[tauri::command] async fn launch(a: State<'_, App>, url: String, pack: String, game: Option<String>) -> Result<(), String> {
    let (c, dir, api) = (cfg(&a)?, a.dir.clone(), a.api.clone());
    let l = tauri::async_runtime::spawn_blocking(move || launcher::launch(&dir, &c, &url, &pack, game.as_deref())).await.map_err(e)?.map_err(e)?;
    api.watch(l); // événements game_open / game_close pour les extensions
    Ok(())
}
/// L'utilisateur ouvre la page d'un jeu dans l'appli -> événement game_page_open
#[tauri::command] fn notify_page_open(a: State<App>, pack: String, id: String, name: String) {
    a.api.push("game_page_open", serde_json::json!({ "pack_id": pack, "id": id, "name": name }));
}
#[tauri::command] async fn update_dump(a: State<'_, App>) -> Result<String, String> {
    let (c, dir) = (cfg(&a)?, a.dir.clone());
    let n = tauri::async_runtime::spawn_blocking(move || local::update_dump(&c, &dir)).await.map_err(e)?.map_err(e)?;
    Ok(format!("dump : {n} fichiers"))
}
/// Prépare une traduction : liste les textes d'un fichier du dump (ex. main/pp7/everyday/script.js) dans translations/template.<nom>.toml
#[tauri::command] fn extract_strings(a: State<App>, file: String) -> Result<String, String> {
    let c = cfg(&a)?;
    let rel = std::path::Path::new(file.trim());
    if rel.is_absolute() || rel.components().any(|x| matches!(x, std::path::Component::ParentDir)) { return Err("chemin invalide".into()); }
    let js = fs::read_to_string(local::dump_path(&c, &a.dir).join(rel)).map_err(e)?;
    let lines = translations::extract(&js);
    let name: String = file.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect();
    let out = a.dir.join("translations").join(format!("template.{name}.toml"));
    fs::create_dir_all(out.parent().unwrap()).map_err(e)?;
    fs::write(&out, lines.join("\n")).map_err(e)?;
    Ok(format!("{} textes -> {}", lines.len(), out.display()))
}
#[tauri::command] async fn install_server_patch(a: State<'_, App>, url: String, pack: String, patch: String, game: Option<String>) -> Result<String, String> {
    let c = cfg(&a)?; let t = tr(&a, &c); let dir = a.dir.clone();
    let n = tauri::async_runtime::spawn_blocking(move || catalog::install(&dir, &c, &url, &pack, &patch, game.as_deref()))
        .await.map_err(e)?.map_err(e)?;
    Ok(t.t("patched", &[&n.to_string()]))
}


// ---- Serveur complet hors-ligne (Jonahbox) ----
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(e)?.map_err(e)
}
#[tauri::command] fn jb_status(a: State<App>) -> Result<serde_json::Value, String> {
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
#[tauri::command] fn jb_commands(a: State<App>) -> Result<String, String> { Ok(jonahbox::commands(&cfg(&a)?)) }
#[tauri::command] fn jb_logs(a: State<App>) -> Result<String, String> { Ok(jonahbox::logs(&cfg(&a)?, &a.dir, 60)) }
#[tauri::command] async fn jb_start(a: State<'_, App>) -> Result<String, String> {
    let (c, d) = (cfg(&a)?, a.dir.clone());
    if a.jonah.lock().unwrap().as_mut().map_or(false, |ch| ch.try_wait().map(|s| s.is_none()).unwrap_or(false)) { return Err("déjà démarré".into()); }
    let child = blocking(move || jonahbox::start(&c, &d)).await?;
    *a.jonah.lock().unwrap() = Some(child);
    Ok("Jonahbox démarré (logs : bouton Logs)".into())
}
#[tauri::command] fn jb_stop(a: State<App>) {
    if let Some(mut c) = a.jonah.lock().unwrap().take() { let _ = c.kill(); let _ = c.wait(); }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            for (name, content) in DEFAULTS {
                let p = dir.join(name);
                if !p.exists() { fs::create_dir_all(p.parent().unwrap())?; fs::write(p, content)?; }
            }
            let args: Vec<String> = std::env::args().skip(1).collect();
            if args.len() == 3 && args[0] == "launch" {
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
        .invoke_handler(tauri::generate_handler![config_dir, get_config, save_config, strings, servers,
            switch_server, catalog_urls, server_info, server_packs, server_news, detect_games, jb_status, jb_install, jb_build, jb_cache, jb_certs, jb_trust, jb_hosts, jb_commands, jb_logs, jb_start, jb_stop, notify_page_open, update_dump, extract_strings, set_pack_path, answer_extension, launch, install_server_patch, patch_install, restore, start_local, stop_local])
        .build(tauri::generate_context!())
        .expect("erreur au lancement")
        .run(|app, ev| {
            // à la fermeture : arrête le serveur local et retire la redirection UPnP
            if let tauri::RunEvent::Exit = ev {
                if let Some(a) = app.try_state::<App>() {
                    if let Some(r) = a.server.lock().unwrap().take() { local::stop(r, true); }
                    if let Some(mut c) = a.jonah.lock().unwrap().take() { let _ = c.kill(); let _ = c.wait(); }
                }
            }
        });
}
