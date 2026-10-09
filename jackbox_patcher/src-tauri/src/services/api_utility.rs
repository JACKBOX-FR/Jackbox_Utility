//! Serveurs de patchs (info.json -> api/packs.json) : packs, jeux, tags, patchs, versions, news, installation.
use crate::model::{config::Config, state::{self, State}};
use crate::services::{automatic_game_finder as finder, downloader as patch, patch_install_controller as tmp3};
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::{Path, PathBuf}};

#[derive(Serialize)] pub struct ServerView { pub id: String, pub name: String, pub description: String, pub languages: Vec<String>, pub image_url: String, pub controller: String }
#[derive(Serialize)] pub struct TagView { pub id: String, pub name: String, pub icon: String, pub description: String }
#[derive(Serialize)] pub struct PatchView { pub id: String, pub name: String, pub small_description: String, pub version: String, pub game_id: Option<String>, pub installed: Option<String>, pub update: bool }
#[derive(Serialize)] pub struct GameView { pub id: String, pub name: String, pub tags: Vec<String> }
#[derive(Serialize)] pub struct PackView { pub id: String, pub name: String, pub games: Vec<GameView>, pub patchs: Vec<PatchView>, pub path: Option<String>, pub launcher: Option<String> }
#[derive(Serialize)] pub struct CatalogView { pub tags: Vec<TagView>, pub packs: Vec<PackView> }

fn s(v: &Value, k: &str) -> String { v[k].as_str().unwrap_or("").to_string() }
fn version_of(p: &Value) -> String { s(p, "version").replace("Build:", "").trim().to_string() }
fn strs(v: &Value) -> Vec<String> { v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() }

/// (api, assets) de la dernière entrée `urls` du info.json
pub fn endpoints(info: &Value) -> anyhow::Result<(String, String)> {
    let u = info["urls"].as_array().and_then(|a| a.last()).ok_or_else(|| anyhow::anyhow!("info.json sans urls"))?;
    Ok((s(u, "api"), s(u, "assets")))
}
/// URL d'une ressource : absolue telle quelle, sinon relative au dossier `assets` du serveur.
pub fn asset_url(assets: &str, p: &str) -> String { if p.starts_with("http") { p.to_string() } else { format!("{}/{}", assets.trim_end_matches('/'), p.trim_start_matches('/')) } }
pub fn view(info: &Value) -> ServerView {
    let assets = endpoints(info).map(|e| e.1).unwrap_or_default();
    let img = s(info, "image");
    ServerView { id: s(info, "id"), name: s(info, "name"), description: s(info, "description"), languages: strs(&info["languages"]),
        image_url: if img.is_empty() { String::new() } else { asset_url(&assets, &img) }, controller: s(info, "controller") }
}
pub fn parse_tags(v: &Value) -> Vec<TagView> {
    v["tags"].as_array().unwrap_or(&vec![]).iter().map(|t| TagView { id: s(t, "id"), name: s(t, "name"), icon: s(t, "icon"), description: s(t, "description") }).collect()
}

/// `installed(pack_id, patch_id)` = version installée (state.json). Une mise à jour = version différente.
pub fn parse_packs(v: &Value, installed: &dyn Fn(&str, &str) -> Option<String>, detected: &dyn Fn(&str) -> (Option<String>, Option<String>)) -> Vec<PackView> {
    v["packs"].as_array().unwrap_or(&vec![]).iter().map(|p| {
        let pid = s(p, "id");
        let mk = |x: &Value, game: Option<&str>| {
            let version = version_of(x);
            let inst = installed(&pid, &s(x, "id"));
            PatchView { id: s(x, "id"), name: s(x, "name"), small_description: s(x, "small_description"), update: inst.as_ref().map_or(false, |i| *i != version),
                version, game_id: game.map(String::from), installed: inst }
        };
        let games = p["games"].as_array().unwrap_or(&vec![]).iter().map(|g| GameView { id: s(g, "id"), name: s(g, "name"), tags: strs(&g["game_info"]["tags"]) }).collect();
        let mut patchs: Vec<PatchView> = p["patchs"].as_array().unwrap_or(&vec![]).iter().map(|x| mk(x, None)).collect();
        for g in p["games"].as_array().unwrap_or(&vec![]) {
            for x in g["patchs"].as_array().unwrap_or(&vec![]) { patchs.push(mk(x, g["id"].as_str())); }
        }
        let (path, launcher) = detected(&pid);
        PackView { id: pid, name: s(p, "name"), games, patchs, path, launcher }
    }).collect()
}

// ---- partie réseau / disque ----

pub fn get_json(url: &str) -> anyhow::Result<Value> { get_json_t(url, 30) }
pub fn get_json_t(url: &str, secs: u64) -> anyhow::Result<Value> {
    Ok(patch::client(Some(std::time::Duration::from_secs(secs)))?.get(url).send()?.error_for_status()?.json()?)
}

fn cache_files(app_dir: &Path, info_url: &str) -> (PathBuf, PathBuf, PathBuf) {
    let cache = app_dir.join("cache"); let _ = fs::create_dir_all(&cache);
    let key: String = info_url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    (cache.join(format!("{key}.info.json")), cache.join(format!("{key}.packs.json")), cache.join(format!("{key}.welcome.json")))
}
fn rd(f: &Path) -> Option<Value> { serde_json::from_str(&fs::read_to_string(f).ok()?).ok() }
pub fn read_cache(app_dir: &Path, info_url: &str) -> Option<(Value, Value)> {
    let (fi, fp, _) = cache_files(app_dir, info_url);
    Some((rd(&fi)?, rd(&fp)?))
}

/// Récupère info.json + packs.json ; garde un cache (cache/) utilisé hors-ligne.
pub fn packs(info_url: &str, app_dir: &Path) -> anyhow::Result<(Value, Value)> {
    let (fi, fp, _) = cache_files(app_dir, info_url);
    let fetched = (|| -> anyhow::Result<(Value, Value)> {
        let info = get_json(info_url)?; let (api, _) = endpoints(&info)?;
        Ok((info, get_json(&format!("{}/packs.json", api.trim_end_matches('/')))?))
    })();
    match fetched {
        Ok((i, p)) => { let _ = fs::write(&fi, i.to_string()); let _ = fs::write(&fp, p.to_string()); Ok((i, p)) }
        Err(e) => read_cache(app_dir, info_url).ok_or(e),
    }
}

/// Fiche du serveur (nom, image…). refresh=false : cache d'abord (affichage immédiat), réseau seulement s'il n'y a rien.
pub fn info(app_dir: &Path, info_url: &str, refresh: bool) -> anyhow::Result<ServerView> {
    let (fi, _, _) = cache_files(app_dir, info_url);
    if !refresh { if let Some(v) = rd(&fi) { return Ok(view(&v)); } }
    match get_json_t(info_url, 10) {
        Ok(v) => { let _ = fs::write(&fi, v.to_string()); Ok(view(&v)) }
        Err(e) => rd(&fi).map(|v| view(&v)).ok_or(e),
    }
}

/// Toutes les données d'un serveur pour l'interface : {info, assets, tags, categories, packs, state}.
/// refresh=false : uniquement le cache (instantané) ; refresh=true : réseau (met le cache à jour).
pub fn raw(dir: &Path, url: &str, refresh: bool) -> anyhow::Result<Value> {
    let (info, packs) = if refresh { packs(url, dir)? } else { read_cache(dir, url).ok_or_else(|| anyhow::anyhow!("pas de cache"))? };
    let (_, assets) = endpoints(&info)?;
    Ok(serde_json::json!({ "url": url, "info": info, "assets": assets, "tags": packs["tags"], "categories": packs["patchsCategories"],
        "packs": packs["packs"], "state": serde_json::to_value(&State::load(dir).packs)? }))
}

/// Vue complète (tags + packs avec versions/chemins) ; mémorise le serveur utilisé.
pub fn view_packs(dir: &Path, url: &str) -> anyhow::Result<CatalogView> {
    let (_, p) = packs(url, dir)?;
    let _ = state::update(dir, |s| s.last_server = Some(url.to_string()));
    let st = State::load(dir);
    Ok(CatalogView { tags: parse_tags(&p), packs: parse_packs(&p,
        &|pk, pt| st.packs.get(pk).and_then(|x| x.patches.get(pt).cloned()),
        &|pk| st.packs.get(pk).map_or((None, None), |x| (x.path.clone(), x.launcher.clone()))) })
}

/// News du serveur (api/welcome.json) -> (assets, news) ; cache hors-ligne.
pub fn news(app_dir: &Path, info_url: &str) -> anyhow::Result<(String, Value)> {
    let (_, _, fw) = cache_files(app_dir, info_url);
    let info = match read_cache(app_dir, info_url) { Some((i, _)) => i, None => get_json_t(info_url, 10)? };
    let (api, assets) = endpoints(&info)?;
    let w = match get_json_t(&format!("{}/welcome.json", api.trim_end_matches('/')), 10) {
        Ok(w) => { let _ = fs::write(&fw, w.to_string()); w }
        Err(e) => rd(&fw).ok_or(e)?,
    };
    Ok((assets, w["news"].clone()))
}

fn ids_of(p: &Value) -> finder::Ids {
    finder::Ids { pack_id: s(p, "id"), steam: p["launchers_id"]["steam"].as_str().map(String::from), epic: p["launchers_id"]["epic"].as_str().map(String::from) }
}

/// Détecte Steam/Epic pour les packs du serveur ; oublie les packs Steam/Epic qui ont disparu.
pub fn detect(app_dir: &Path, info_url: &str) -> anyhow::Result<Vec<finder::Found>> {
    let (_, packs) = packs(info_url, app_dir)?;
    let ids: Vec<_> = packs["packs"].as_array().unwrap_or(&vec![]).iter().map(ids_of).collect();
    let found = finder::find(&ids);
    let f2 = found.clone();
    state::update(app_dir, move |st| {
        for (id, p) in st.packs.iter_mut() {
            if matches!(p.launcher.as_deref(), Some("steam") | Some("epic")) && !f2.iter().any(|f| &f.pack_id == id) { p.path = None; p.launcher = None; }
        }
        for f in f2 { let p = st.packs.entry(f.pack_id).or_default(); p.path = Some(f.path); p.launcher = Some(f.launcher); }
    })?;
    Ok(found)
}

fn same_path(a: &str, b: &str) -> bool { let n = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase(); n(a) == n(b) }

/// Dossier choisi à la main ("game finder from path") : détecte si c'est une install Steam/Epic, sinon natif.
pub fn set_path(app_dir: &Path, info_url: &str, pack_id: &str, path: &str) -> anyhow::Result<String> {
    if !Path::new(path).is_dir() { anyhow::bail!("dossier introuvable : {path}"); }
    let (_, packs) = packs(info_url, app_dir)?;
    let pack = packs["packs"].as_array().and_then(|a| a.iter().find(|p| p["id"] == pack_id)).ok_or_else(|| anyhow::anyhow!("pack introuvable"))?;
    let launcher = finder::find(&[ids_of(pack)]).into_iter().find(|f| same_path(&f.path, path)).map(|f| f.launcher).unwrap_or_else(|| "native".into());
    let (pk, pa, l) = (pack_id.to_string(), path.to_string(), launcher.clone());
    state::update(app_dir, move |st| { let p = st.packs.entry(pk).or_default(); p.path = Some(pa); p.launcher = Some(l); })?;
    Ok(launcher)
}

/// Télécharge et installe un patch (de pack, ou de jeu si game_id), lance l'install_controller, mémorise la version.
pub fn install(app_dir: &Path, cfg: &Config, info_url: &str, pack_id: &str, patch_id: &str, game_id: Option<&str>, progress: &dyn Fn(&str, f64)) -> anyhow::Result<usize> {
    progress("starting", 0.0);
    let (info, packs) = packs(info_url, app_dir)?;
    let (_, assets) = endpoints(&info)?;
    let pack = packs["packs"].as_array().and_then(|a| a.iter().find(|p| p["id"] == pack_id)).ok_or_else(|| anyhow::anyhow!("pack introuvable"))?;
    let mut dir: PathBuf = state::pack_dir(&State::load(app_dir), cfg, pack_id);
    if dir.as_os_str().is_empty() { anyhow::bail!("dossier du pack inconnu : clique Détecter, ou renseigne [pack_dirs]/game_dir"); }
    let list = match game_id {
        Some(g) => {
            let game = pack["games"].as_array().and_then(|a| a.iter().find(|x| x["id"] == g)).ok_or_else(|| anyhow::anyhow!("jeu introuvable"))?;
            dir = dir.join(s(game, "path"));
            &game["patchs"]
        }
        None => &pack["patchs"],
    };
    let p = list.as_array().and_then(|a| a.iter().find(|x| x["id"] == patch_id)).ok_or_else(|| anyhow::anyhow!("patch introuvable"))?;
    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let supported = match p["supported_platforms"].as_array() { Some(a) => a.iter().any(|x| x.as_str() == Some(os)), None => os != "mac" }; // défaut de l'appli d'origine
    if !supported { anyhow::bail!("patch non supporté sur {os}"); }
    let paths: Vec<String> = match p["patch_paths"].as_array() {
        Some(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
        None => vec![s(p, "patch_path")],
    };
    let (mut n, count) = (0, paths.len().max(1) as f64);
    for (i, path) in paths.iter().enumerate() {
        let url = asset_url(&assets, path);
        let t = patch::download_with(&url, &|done, total| progress("downloading", (i as f64 + total.map_or(0.0, |t| done as f64 / t.max(1) as f64)) / count))?;
        progress("extracting", i as f64 / count);
        n += patch::extract(Some(&cfg.patch), &t.0, &dir, false)?;
    }
    progress("finalizing", 1.0);
    let c = &p["install_controller"];
    if s(c, "id") == "tmp3" { tmp3::run(&dir, &s(c, "controller_url"), &s(c, "online_service_url"))?; }
    let (pk, pt, ver) = (pack_id.to_string(), patch_id.to_string(), version_of(p));
    state::update(app_dir, move |st| { st.packs.entry(pk).or_default().patches.insert(pt, ver); })?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses() {
        let v: Value = serde_json::from_str(r#"{"tags":[{"id":"t","name":"T","icon":"i","description":"d"}],"packs":[{"id":"p","name":"P","games":[{"id":"g","name":"G","game_info":{"tags":["t"]},"patchs":[{"id":"gp","name":"x","version":"2"}]}],"patchs":[{"id":"a","name":"A","version":"Build: 1"}]}]}"#).unwrap();
        let p = parse_packs(&v, &|_, id| if id == "gp" { Some("1".into()) } else { None }, &|_| (None, None));
        assert_eq!(p[0].patchs.len(), 2); assert_eq!(p[0].patchs[1].game_id.as_deref(), Some("g"));
        assert!(p[0].patchs[1].update); assert_eq!(p[0].patchs[0].version, "1");
        assert_eq!(p[0].games[0].tags, vec!["t"]); assert_eq!(parse_tags(&v)[0].name, "T");
    }
    #[test] fn paths() { assert!(same_path("C:\\A\\B\\", "c:/a/b")); }
}
