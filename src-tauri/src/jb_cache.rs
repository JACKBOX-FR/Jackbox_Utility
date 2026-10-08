//! Port en Rust de `dump_to_cache.py` (Jonahbox, AGPL-3.0-or-later : ce fichier en est donc dérivé).
//! Parties "pures" de l'intégration Jonahbox : conversion du dump jackbox.tv en cache (port de dump_to_cache.py),
//! édition du fichier hosts, génération du config.toml de Jonahbox.
use crate::translations::Translator;
use aho_corasick::{AhoCorasick, MatchKind};
use sha1::{Digest, Sha1};
use std::{fs, path::Path};
use walkdir::WalkDir;

const HOST: &str = "jackbox.tv";
const TEXT_EXT: [&str; 9] = [".js", ".mjs", ".html", ".htm", ".css", ".json", ".webmanifest", ".txt", ".jet"];
/// Adresses officielles remplacées par celle de ton serveur dans les fichiers texte de la manette.
pub const REWRITE: [&str; 3] = ["ecast.jackboxgames.com", "blobcast.jackboxgames.com", "blobcast-test.jackboxgames.com"];

#[derive(Default, Debug)]
pub struct Stats { pub added: usize, pub skipped: usize, pub rewritten_files: usize, pub rewritten_refs: usize }

/// Format du cache de Jonahbox pour https://jackbox.tv/P : cache/jackbox.tv/P = {"etag","content_type","compressed"} + cache/jackbox.tv/<etag> = contenu.
/// La traduction (translations/<langue>.toml) est appliquée avant la redirection des adresses.
pub fn build_cache(dump: &Path, cache: &Path, new_host: &str, tr: &Translator, force: bool) -> anyhow::Result<Stats> {
    if !dump.is_dir() { anyhow::bail!("dump introuvable : {} (bouton « Dump »)", dump.display()); }
    let ac = AhoCorasick::builder().match_kind(MatchKind::LeftmostLongest).build(REWRITE)?;
    let repl = vec![new_host; REWRITE.len()];
    let mut st = Stats::default();
    let main = dump.join("main");
    for e in WalkDir::new(&main).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
        let rel = e.path().strip_prefix(&main)?.to_string_lossy().replace('\\', "/");
        add(cache, &format!("/main/{rel}"), e.path(), &ac, &repl, tr, force, &mut st)?;
    }
    for e in fs::read_dir(dump)?.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().to_string();
        if e.path().is_file() && !["README.md", "CNAME", "LICENSE"].contains(&name.as_str()) {
            let url = if name == "index.html" { "/".to_string() } else { format!("/{name}") };
            add(cache, &url, &e.path(), &ac, &repl, tr, force, &mut st)?;
        }
    }
    Ok(st)
}

#[allow(clippy::too_many_arguments)]
fn add(cache: &Path, url: &str, src: &Path, ac: &AhoCorasick, repl: &[&str], tr: &Translator, force: bool, st: &mut Stats) -> anyhow::Result<()> {
    let key = cache.join(HOST).join(if url == "/" { "index.html" } else { url.trim_start_matches('/') });
    if key.exists() && !force { st.skipped += 1; return Ok(()); }
    let mut data = fs::read(src)?;
    let ext = src.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
    if TEXT_EXT.contains(&ext.as_str()) {
        if !tr.is_empty() { if let Ok(s) = std::str::from_utf8(&data) { data = tr.apply(s).into_bytes(); } }
        let n = ac.find_iter(&data).count();
        if n > 0 { data = ac.replace_all_bytes(&data, repl); st.rewritten_files += 1; st.rewritten_refs += n; }
    }
    let etag = format!("dump-{}", Sha1::digest(&data).iter().map(|b| format!("{b:02x}")).collect::<String>());
    let blob = cache.join(HOST).join(&etag);
    fs::create_dir_all(key.parent().unwrap())?;
    if !blob.exists() { fs::write(&blob, &data)?; }
    let ctype = mime_guess::from_path(src).first_or_octet_stream().to_string();
    fs::write(&key, serde_json::json!({ "etag": etag, "content_type": ctype, "compressed": false }).to_string())?;
    st.added += 1;
    Ok(())
}

pub const MARK: &str = "# jbx";
fn host_ok(s: &str) -> bool { !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || ".-:".contains(c)) }

/// Ajoute (enable) ou retire les lignes marquées `# jbx` du fichier hosts ; le reste du fichier n'est pas touché.
pub fn hosts_edit(text: &str, ip: &str, names: &[&str], enable: bool) -> anyhow::Result<String> {
    if !host_ok(ip) || names.iter().any(|n| !host_ok(n)) { anyhow::bail!("adresse ou nom invalide pour le fichier hosts"); }
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: Vec<String> = text.lines().filter(|l| !l.trim_end().ends_with(MARK)).map(String::from).collect();
    if enable { for n in names { out.push(format!("{ip} {n} {MARK}")); } }
    let mut s = out.join(nl); s.push_str(nl);
    Ok(s)
}

pub struct CfgParams<'a> { pub host: &'a str, pub ip: &'a str, pub cache_mode: &'a str, pub cert: &'a str, pub key: &'a str, pub https: u16, pub http: u16, pub blobcast: u16 }

/// config.toml de Jonahbox (mêmes sections que son config.toml d'origine). Les chemins sont correctement échappés (Windows).
pub fn render_config(p: &CfgParams) -> String {
    let q = |s: &str| toml::Value::String(s.to_string()).to_string();
    let mode = if ["online", "oneshot", "offline"].contains(&p.cache_mode) { p.cache_mode } else { "oneshot" };
    format!("accessible_host = {host}\ntui = false\n\n[cache]\ncache_path = \"jb_cache\"\ncache_mode = {mode}\n\n[doodles]\nrender = true\npath = \"doodle_renders\"\n\n[tts]\npiper_bin = \"piper-tts\"\nffmpeg_bin = \"ffmpeg\"\nvoices_path = \"voices\"\ntts_dir = \"tts\"\nop_mode = \"proxy\"\n\n[ecast]\nop_mode = \"native\"\nserver_url = {eu}\n\n[blobcast]\nop_mode = \"native\"\nserver_url = {bu}\n\n[tls]\ncert = {cert}\nkey = {key}\n\n[ports]\nhttps = {https}\nblobcast = {blob}\nhttp = {http}\n",
        host = q(p.host), mode = q(mode), eu = q(&format!("https://{}:8888", p.ip)), bu = q(&format!("https://{}:8080", p.ip)),
        cert = q(p.cert), key = q(p.key), https = p.https, blob = p.blobcast, http = p.http)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    fn tmp(n: &str) -> std::path::PathBuf { let d = std::env::temp_dir().join(format!("jbx-test-{n}-{}", std::process::id())); let _ = fs::remove_dir_all(&d); fs::create_dir_all(&d).unwrap(); d }

    #[test] fn cache_conversion_rewrites_translates_and_skips() {
        let (dump, cache) = (tmp("d"), tmp("c"));
        fs::create_dir_all(dump.join("main/pp1")).unwrap();
        fs::write(dump.join("index.html"), "<a>ecast.jackboxgames.com</a>").unwrap();
        fs::write(dump.join("README.md"), "x").unwrap();
        fs::write(dump.join("main/pp1/script.js"), "u=\"https://blobcast.jackboxgames.com\";t=\"Thanks\";c=\"ecast.jackboxgames.com\"").unwrap();
        fs::write(dump.join("main/pp1/img.png"), [1u8, 2, 3]).unwrap();
        let mut m = HashMap::new(); m.insert("\"Thanks\"".to_string(), "\"Merci\"".to_string());
        let st = build_cache(&dump, &cache, "jonahbox.local", &Translator::from_map(m).unwrap(), false).unwrap();
        assert_eq!((st.added, st.skipped, st.rewritten_files, st.rewritten_refs), (3, 0, 2, 3));
        let meta: serde_json::Value = serde_json::from_str(&fs::read_to_string(cache.join("jackbox.tv/main/pp1/script.js")).unwrap()).unwrap();
        let blob = fs::read_to_string(cache.join("jackbox.tv").join(meta["etag"].as_str().unwrap())).unwrap();
        assert_eq!(blob, "u=\"https://jonahbox.local\";t=\"Merci\";c=\"jonahbox.local\"");
        assert!(meta["etag"].as_str().unwrap().starts_with("dump-") && meta["compressed"] == false);
        assert!(cache.join("jackbox.tv/index.html").is_file() && !cache.join("jackbox.tv/README.md").exists());
        let again = build_cache(&dump, &cache, "jonahbox.local", &Translator::empty(), false).unwrap();
        assert_eq!((again.added, again.skipped), (0, 3)); // n'écrase pas sans force
    }
    #[test] fn hosts_add_remove_keeps_other_lines() {
        let base = "127.0.0.1 localhost\r\n# commentaire\r\n";
        let on = hosts_edit(base, "192.168.1.5", &["jonahbox.local"], true).unwrap();
        assert!(on.contains("192.168.1.5 jonahbox.local # jbx\r\n") && on.starts_with(base));
        let on2 = hosts_edit(&on, "192.168.1.9", &["jonahbox.local"], true).unwrap(); // pas de doublon
        assert_eq!(on2.matches("jonahbox.local").count(), 1);
        assert_eq!(hosts_edit(&on2, "x", &[], false).unwrap(), base);
        assert!(hosts_edit(base, "1.2.3.4", &["a\nb"], true).is_err());
    }
    #[test] fn config_is_valid_toml_with_windows_paths() {
        let s = render_config(&CfgParams { host: "jonahbox.local", ip: "192.168.1.5", cache_mode: "offline", cert: "C:\\Users\\me\\certs\\server.pem", key: "C:\\k.pem", https: 443, http: 80, blobcast: 38203 });
        let v: toml::Value = toml::from_str(&s).unwrap();
        assert_eq!(v["tls"]["cert"].as_str().unwrap(), "C:\\Users\\me\\certs\\server.pem");
        assert_eq!(v["cache"]["cache_mode"].as_str().unwrap(), "offline");
        assert_eq!(v["ports"]["https"].as_integer().unwrap(), 443);
        assert_eq!(toml::from_str::<toml::Value>(&render_config(&CfgParams { host: "h", ip: "i", cache_mode: "bad", cert: "c", key: "k", https: 1, http: 2, blobcast: 3 })).unwrap()["cache"]["cache_mode"].as_str().unwrap(), "oneshot");
    }
}
