//! État utilisateur (state.json) : packs détectés, patchs installés, loaders, dernier serveur.
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::{Path, PathBuf}, sync::Mutex};

static LOCK: Mutex<()> = Mutex::new(()); // évite les écritures concurrentes de state.json

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PackState {
    pub path: Option<String>, pub launcher: Option<String>, pub loader_version: Option<String>,
    #[serde(default)] pub patches: HashMap<String, String>,       // patch_id -> version installée
    #[serde(default)] pub game_loaders: HashMap<String, String>,  // game_id -> version du loader
}
#[derive(Default, Serialize, Deserialize)]
pub struct State { #[serde(default)] pub last_server: Option<String>, #[serde(default)] pub packs: HashMap<String, PackState> }

impl State {
    pub fn load(dir: &Path) -> Self { fs::read_to_string(dir.join("state.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default() }
    pub fn save(&self, dir: &Path) -> anyhow::Result<()> {
        let tmp = dir.join("state.json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?; fs::rename(tmp, dir.join("state.json"))?; Ok(())
    }
}
pub fn update<F: FnOnce(&mut State)>(dir: &Path, f: F) -> anyhow::Result<()> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = State::load(dir); f(&mut s); s.save(dir)
}

/// Dossier d'un pack : `[pack_dirs]` (manuel) > détection auto > game_dir.
pub fn pack_dir(st: &State, cfg: &Config, pack_id: &str) -> PathBuf {
    cfg.pack_dirs.get(pack_id).cloned().or_else(|| st.packs.get(pack_id).and_then(|p| p.path.clone()))
        .unwrap_or_else(|| cfg.game_dir.clone()).into()
}
/// Tous les dossiers de jeu connus (pour le changement de serveur).
pub fn all_dirs(st: &State, cfg: &Config) -> Vec<String> {
    let mut v: Vec<String> = vec![cfg.game_dir.clone()];
    v.extend(cfg.pack_dirs.values().cloned());
    v.extend(st.packs.values().filter_map(|p| p.path.clone()));
    v.retain(|d| !d.is_empty()); v.sort(); v.dedup(); v
}
