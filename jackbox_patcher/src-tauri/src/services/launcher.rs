//! Lancement d'un pack ou d'un jeu (Steam / Epic / natif), avec loaders (port de launcher.dart).
use crate::model::{config::Config, state::{self, State}};
use crate::services::{api_utility as catalog, downloader as patch};
use serde_json::{json, Value};
use std::{path::Path, process::Command};

pub fn params(internal_name: Option<&str>) -> Option<String> {
    internal_name.map(|n| format!("-launchTo games%2F{n}%2F{n}.swf -jbg.config isBundle=false"))
}

fn ensure_loader(app: &Path, assets: &str, dest: &Path, l: &Value, pack: &str, game: Option<&str>) -> anyhow::Result<()> {
    let ver = l["version"].as_str().unwrap_or("").to_string();
    let st = State::load(app);
    let cur = st.packs.get(pack).and_then(|p| match game { Some(g) => p.game_loaders.get(g), None => p.loader_version.as_ref() });
    if cur == Some(&ver) { return Ok(()); }
    let path = l["path"].as_str().ok_or_else(|| anyhow::anyhow!("loader sans path"))?;
    let url = if path.starts_with("http") { path.to_string() } else { format!("{}/{}", assets.trim_end_matches('/'), path.trim_start_matches('/')) };
    let t = patch::download(&url)?;
    patch::extract(None, &t.0, dest, false)?;
    let g = game.map(String::from);
    state::update(app, |s| { let p = s.packs.entry(pack.into()).or_default();
        match g { Some(g) => { p.game_loaders.insert(g, ver); } None => p.loader_version = Some(ver) } })
}

pub struct Launched { pub exe: String, pub game: Option<Value> }

/// Le pack est-il en cours d'exécution ? (tasklist / ps, comme l'appli d'origine ; tasklist tronque à 25 caractères)
pub fn is_running(exe: &str) -> bool {
    let mut c = if cfg!(windows) { let mut c = Command::new("tasklist"); c.args(["/FO", "LIST"]); c } else { let mut c = Command::new("ps"); c.arg("-ax"); c };
    #[cfg(windows)] { use std::os::windows::process::CommandExt; c.creation_flags(0x0800_0000); } // pas de fenêtre console
    let out = c.output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let name: String = if cfg!(windows) { exe.chars().take(25).collect() } else { exe.to_string() };
    !name.is_empty() && out.contains(&name)
}

pub fn launch(app: &Path, cfg: &Config, info_url: &str, pack_id: &str, game_id: Option<&str>) -> anyhow::Result<Launched> {
    let (info, packs) = catalog::packs(info_url, app)?;
    let (_, assets) = catalog::endpoints(&info)?;
    let pack = packs["packs"].as_array().and_then(|a| a.iter().find(|p| p["id"] == pack_id)).ok_or_else(|| anyhow::anyhow!("pack introuvable"))?;
    let mut game = game_id.and_then(|g| pack["games"].as_array()?.iter().find(|x| x["id"] == g));
    let game0: Option<Value> = game.cloned();
    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let exe_name = pack["executables"][os].as_str().map(|e| Path::new(e).file_name().and_then(|f| f.to_str()).unwrap_or(e).to_string()).unwrap_or_default();
    let st = State::load(app);
    let dir = state::pack_dir(&st, cfg, pack_id);
    let launcher = st.packs.get(pack_id).and_then(|p| p.launcher.clone()).unwrap_or_else(|| "native".into());
    let (key, default) = match launcher.as_str() { "steam" => ("steam", false), "epic" => ("epic_games", true), _ => ("native", true) };
    let use_loader = game.and_then(|g| g["launch_with_loaders"][key].as_bool()).unwrap_or(default);

    if use_loader {
        if pack["loader"].is_object() { ensure_loader(app, &assets, &dir, &pack["loader"], pack_id, None)?; }
        if let Some(g) = game {
            if g["loader"].is_object() {
                ensure_loader(app, &assets, &dir.join(g["path"].as_str().unwrap_or("")), &g["loader"], pack_id, g["id"].as_str())?;
            } else { game = None; }
        }
    }
    let p = if use_loader { None } else { params(game.and_then(|g| g["internal_name"].as_str())) };
    let ids = &pack["launchers_id"];
    match launcher.as_str() {
        "steam" => open::that(format!("steam://run/{}//{}", ids["steam"].as_str().unwrap_or(""), p.unwrap_or_default().replace(' ', "%20")))?,
        "epic" => open::that(format!("com.epicgames.launcher://apps/{}?action=launch&silent=true{}", ids["epic"].as_str().unwrap_or(""),
            p.map(|x| format!("%20{}", x.replace(' ', "%20"))).unwrap_or_default()))?,
        _ => {
            let exe = pack["executables"][os].as_str().ok_or_else(|| anyhow::anyhow!("pas d'exécutable pour cet OS"))?;
            Command::new(dir.join(exe)).args(p.unwrap_or_default().split_whitespace()).current_dir(&dir).spawn()?;
        }
    }
    Ok(Launched { exe: exe_name, game: game0.map(|mut g| { g["pack_id"] = json!(pack_id); g }) })
}

/// `jbx launch pack <id>` / `jbx launch game <id>` (serveur = dernier utilisé)
pub fn launch_raw(app: &Path, cfg: &Config, kind: &str, id: &str) -> anyhow::Result<Launched> {
    let url = State::load(app).last_server.ok_or_else(|| anyhow::anyhow!("aucun serveur utilisé : ouvre l'appli une fois"))?;
    if kind == "pack" { return launch(app, cfg, &url, id, None); }
    let (_, packs) = catalog::packs(&url, app)?;
    let pack = packs["packs"].as_array().and_then(|a| a.iter().find(|p| p["games"].as_array().map_or(false, |g| g.iter().any(|x| x["id"] == id))))
        .ok_or_else(|| anyhow::anyhow!("jeu introuvable"))?;
    launch(app, cfg, &url, pack["id"].as_str().unwrap_or(""), Some(id))
}

#[cfg(test)]
mod tests { #[test] fn p() { assert!(super::params(Some("Quiplash3")).unwrap().contains("games%2FQuiplash3%2FQuiplash3.swf")); } }
