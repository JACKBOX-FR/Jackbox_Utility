use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub language: String, pub game_dir: String,
    pub server: ServerCfg, pub local: LocalCfg, pub patch: PatchCfg, pub restore: RestoreCfg,
    #[serde(default)] pub api: ApiCfg,
    #[serde(default)] pub jonahbox: JonahCfg,
    #[serde(default)] pub pack_dirs: std::collections::HashMap<String, String>,
}
#[derive(Deserialize, Serialize, Clone)]
pub struct ServerCfg { pub active: String, pub list: Vec<Server> }
#[derive(Deserialize, Serialize, Clone)]
pub struct Server { pub name: String, pub join_url: String, #[serde(default)] pub server_url: Option<String> } // server_url : écrit aussi serverUrl (ecast) si présent
#[derive(Deserialize, Serialize, Clone)]
pub struct LocalCfg {
    pub dump_dir: String, pub bind: String, pub port: u16, pub upnp: bool, #[serde(default)] pub https: bool,
    #[serde(default)] pub dump_language: String,                         // translations/<langue>.toml appliqué au dump ("" = aucune)
    #[serde(default = "d_repo")] pub dump_repo: String,                  // dépôt du dump à télécharger / mettre à jour
    #[serde(default = "d_branch")] pub dump_branch: String,
}
fn d_repo() -> String { "https://github.com/JACKBOX-FR/jackbox-fr-main-dump".into() }
fn d_branch() -> String { "main".into() }
#[derive(Deserialize, Serialize, Clone)]
pub struct PatchCfg {
    pub audio: bool, pub subtitles: bool, pub text: bool, pub images: bool, pub other: bool,
    pub games: Vec<String>, pub sources_dir: String,
}
#[derive(Deserialize, Serialize, Clone)]
pub struct ApiCfg { pub enabled: bool, pub port: u16 }
impl Default for ApiCfg { fn default() -> Self { Self { enabled: true, port: 6480 } } }
#[derive(Deserialize, Serialize, Clone)]
pub struct JonahCfg {
    pub repo: String, pub branch: String, pub dir: String, pub binary: String, pub host: String, pub ip: String,
    pub cache_mode: String, pub https_port: u16, pub http_port: u16, pub blobcast_port: u16,
}
impl Default for JonahCfg { fn default() -> Self { Self {
    repo: "https://github.com/JACKBOX-FR/Jonahbox-Dump-Ecast-Blobcast".into(), branch: "main".into(), dir: "jonahbox".into(), binary: String::new(),
    host: "jonahbox.local".into(), ip: String::new(), cache_mode: "oneshot".into(), https_port: 443, http_port: 80, blobcast_port: 38203 } } }
#[derive(Deserialize, Serialize, Clone)]
pub struct RestoreCfg { pub repo: String, pub branch: String }
#[derive(Deserialize)]
pub struct Sources { pub repo: Vec<Repo> }
#[derive(Deserialize, Clone)]
pub struct Repo { pub name: String, pub url: String, pub branch: String }

impl Config {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        Ok(toml::from_str(&fs::read_to_string(dir.join("config.toml"))?)?)
    }
    pub fn sources(&self, dir: &Path) -> anyhow::Result<Sources> {
        let p = dir.join(&self.patch.sources_dir).join(format!("{}.toml", self.language));
        if !p.is_file() { return Ok(Sources { repo: vec![] }); } // langue sans fichier de dépôts : rien à télécharger
        Ok(toml::from_str(&fs::read_to_string(p)?)?)
    }
}
