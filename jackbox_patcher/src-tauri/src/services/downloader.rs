use crate::model::config::{Config, PatchCfg, Repo};
use std::{fs::{self, File}, io::{copy, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

fn wanted(p: &PatchCfg, name: &str) -> bool {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "ogg" => p.audio,
        "swf" | "json" => p.subtitles,
        "jet" => p.text,
        "png" | "jpg" | "jpeg" => p.images,
        _ => p.other,
    }
}
fn game_ok(p: &PatchCfg, name: &str) -> bool { p.games.is_empty() || p.games.iter().any(|g| name.contains(g.as_str())) }

pub fn client(timeout: Option<Duration>) -> anyhow::Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(15)).timeout(timeout).user_agent("jbx").build()?)
}

/// Fichier temporaire supprimé automatiquement (les gros zips ne passent plus en mémoire).
pub struct Tmp(pub File, PathBuf);
impl Drop for Tmp { fn drop(&mut self) { let _ = fs::remove_file(&self.1); } }

pub fn download(url: &str) -> anyhow::Result<Tmp> { download_with(url, &|_, _| {}) }

/// Télécharge sur disque ; `on(octets_lus, total)` est appelé ~8 fois par seconde (barre de progression).
pub fn download_with(url: &str, on: &dyn Fn(u64, Option<u64>)) -> anyhow::Result<Tmp> {
    let mut r = client(None)?.get(url).send()?.error_for_status()?;
    let total = r.content_length();
    let n = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("jbx-{}-{n}.zip", std::process::id()));
    let mut t = Tmp(fs::OpenOptions::new().read(true).write(true).create_new(true).open(&path)?, path);
    let (mut buf, mut done, mut last) = (vec![0u8; 64 * 1024], 0u64, Instant::now());
    loop {
        let k = r.read(&mut buf)?;
        if k == 0 { break; }
        t.0.write_all(&buf[..k])?;
        done += k as u64;
        if last.elapsed() >= Duration::from_millis(120) { on(done, total); last = Instant::now(); }
    }
    on(done, total);
    t.0.seek(SeekFrom::Start(0))?;
    Ok(t)
}

/// Extrait un zip dans `dest` (filter = filtres [patch], None = tout, ex: loaders). `strip_root` : retire le dossier racine (zip GitHub).
pub fn extract<R: Read + Seek>(filter: Option<&PatchCfg>, reader: R, dest: &Path, strip_root: bool) -> anyhow::Result<usize> {
    if dest.as_os_str().is_empty() { anyhow::bail!("dossier de destination vide"); }
    let mut zip = zip::ZipArchive::new(reader)?;
    let mut n = 0;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.is_dir() { continue; }
        let rel: PathBuf = match f.enclosed_name() { // enclosed_name bloque les chemins "../"
            Some(p) if strip_root => p.components().skip(1).collect(),
            Some(p) => p,
            None => continue,
        };
        if rel.as_os_str().is_empty() { continue; }
        let s = rel.to_string_lossy().to_string();
        if let Some(fl) = filter { if !wanted(fl, &s) || !game_ok(fl, &s) { continue; } }
        let out = dest.join(&rel);
        if let Some(d) = out.parent() { fs::create_dir_all(d)?; }
        copy(&mut f, &mut File::create(&out)?)?;
        #[cfg(unix)] { // garde le bit exécutable (ex: Launcher.sh)
            use std::os::unix::fs::PermissionsExt;
            if let Some(m) = f.unix_mode() { let _ = fs::set_permissions(&out, fs::Permissions::from_mode(m)); }
        }
        n += 1;
    }
    Ok(n)
}

/// Patch depuis un dépôt GitHub (zip de la branche).
pub fn install(cfg: &Config, repo: &Repo) -> anyhow::Result<usize> {
    if cfg.game_dir.trim().is_empty() { anyhow::bail!("game_dir est vide dans config.toml"); }
    let url = format!("{}/archive/refs/heads/{}.zip", repo.url.trim_end_matches('/'), repo.branch);
    let t = download(&url).map_err(|e| anyhow::anyhow!("{} : {e}", repo.name))?;
    extract(Some(&cfg.patch), &t.0, Path::new(&cfg.game_dir), true)
}

/// Remise à zéro : dépôt [restore], tous types et tous jeux.
pub fn restore(cfg: &Config) -> anyhow::Result<Option<usize>> {
    if cfg.restore.repo.is_empty() { return Ok(None); }
    let mut all = cfg.clone();
    all.patch = PatchCfg { audio: true, subtitles: true, text: true, images: true, other: true, games: vec![], ..cfg.patch.clone() };
    let repo = Repo { name: "restore".into(), url: cfg.restore.repo.clone(), branch: cfg.restore.branch.clone() };
    Ok(Some(install(&all, &repo)?))
}
