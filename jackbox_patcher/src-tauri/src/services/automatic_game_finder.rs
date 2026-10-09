//! Détection automatique des packs installés (Steam, Epic Games).
use regex::Regex;
use serde::Serialize;
use std::{env, fs, path::{Path, PathBuf}};

#[derive(Serialize, Clone)] pub struct Found { pub pack_id: String, pub path: String, pub launcher: String }
pub struct Ids { pub pack_id: String, pub steam: Option<String>, pub epic: Option<String> }

#[cfg(windows)]
fn reg(path: &str, key: &str) -> Option<String> {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
    RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path).ok()?.get_value::<String, _>(key).ok()
}
#[cfg(not(windows))]
fn reg(_: &str, _: &str) -> Option<String> { None }

fn home() -> PathBuf { PathBuf::from(env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_default()) }

fn steam_root() -> Option<PathBuf> {
    let c: Vec<PathBuf> = if cfg!(windows) {
        reg("SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath").map(PathBuf::from)
            .into_iter().chain([PathBuf::from("C:\\Program Files (x86)\\Steam")]).collect()
    } else if cfg!(target_os = "macos") {
        vec![home().join("Library/Application Support/Steam")]
    } else {
        [".steam/steam", ".var/app/com.valvesoftware.Steam/.steam/steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam"]
            .iter().map(|p| home().join(p)).collect()
    };
    c.into_iter().find(|p| p.is_dir())
}

pub fn parse_vdf_paths(t: &str) -> Vec<PathBuf> {
    let re = Regex::new(r#""path"\s+"([^"]+)""#).unwrap();
    re.captures_iter(t).map(|c| PathBuf::from(c[1].replace("\\\\", "\\"))).collect()
}
pub fn parse_installdir(t: &str) -> Option<String> {
    Regex::new(r#""installdir"\s+"([^"]+)""#).unwrap().captures(t).map(|c| c[1].to_string())
}

fn steam(ids: &[Ids]) -> Vec<Found> {
    let Some(root) = steam_root() else { return vec![] };
    let mut libs = vec![root.clone()];
    if let Ok(t) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) { libs.extend(parse_vdf_paths(&t)); }
    let mut out = vec![];
    for i in ids {
        let Some(app) = &i.steam else { continue };
        for l in &libs {
            let m = l.join("steamapps").join(format!("appmanifest_{app}.acf"));
            if let Some(dir) = fs::read_to_string(&m).ok().and_then(|t| parse_installdir(&t)) {
                out.push(Found { pack_id: i.pack_id.clone(), path: l.join("steamapps/common").join(dir).to_string_lossy().into(), launcher: "steam".into() });
                break;
            }
        }
    }
    out
}

fn epic_file() -> Option<PathBuf> {
    let f: Vec<PathBuf> = if cfg!(windows) {
        reg("SOFTWARE\\WOW6432Node\\Epic Games\\EpicGamesLauncher", "AppDataPath")
            .map(|p| Path::new(&p).join("../../UnrealEngineLauncher/LauncherInstalled.dat"))
            .into_iter().chain(env::var("PROGRAMDATA").ok().map(|p| Path::new(&p).join("Epic/UnrealEngineLauncher/LauncherInstalled.dat"))).collect()
    } else if cfg!(target_os = "macos") {
        vec![home().join("Library/Application Support/Epic/UnrealEngineLauncher/LauncherInstalled.dat")]
    } else { vec![] };
    f.into_iter().find(|p| p.is_file())
}

fn epic(ids: &[Ids]) -> Vec<Found> {
    let Some(v) = epic_file().and_then(|f| fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else { return vec![] };
    let apps = v["InstallationList"].as_array().cloned().unwrap_or_default();
    ids.iter().filter_map(|i| {
        let id = i.epic.as_ref()?;
        let a = apps.iter().find(|a| a["AppName"].as_str() == Some(id))?;
        Some(Found { pack_id: i.pack_id.clone(), path: a["InstallLocation"].as_str()?.into(), launcher: "epic".into() })
    }).collect()
}

/// Steam d'abord, puis Epic pour les packs restants.
pub fn find(ids: &[Ids]) -> Vec<Found> {
    let mut r = steam(ids);
    for f in epic(ids) { if !r.iter().any(|x| x.pack_id == f.pack_id) { r.push(f); } }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vdf() {
        let t = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t\t\"apps\" { \"331670\" \"1\" }\n\t}\n\t\"1\" { \"path\"\t\"D:\\\\Games\" }\n}";
        let p = parse_vdf_paths(t);
        assert_eq!(p.len(), 2); assert_eq!(p[1], PathBuf::from("D:\\Games"));
        assert_eq!(parse_installdir("\"installdir\"\t\t\"The Jackbox Party Pack\"").as_deref(), Some("The Jackbox Party Pack"));
    }
}
