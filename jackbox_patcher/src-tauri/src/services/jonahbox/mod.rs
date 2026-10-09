//! Serveur 100 % local (Jonahbox : ecast + blobcast en Rust) : installation, compilation, cache de la manette,
//! certificats HTTPS, confiance, fichier hosts, démarrage/arrêt. Tout est déclenché explicitement par l'utilisateur.
pub mod cache;
use crate::model::config::Config;
use crate::services::{downloader as patch, local_server, translations::Translator};
use cache as jb_cache;
use rcgen::{BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose};
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}, process::{Child, Command, Stdio}};

const CA_NAME: &str = "jbx local CA";

pub fn dir(cfg: &Config, app: &Path) -> PathBuf { let p = Path::new(&cfg.jonahbox.dir); if p.is_absolute() { p.to_path_buf() } else { app.join(p) } }
pub fn binary(cfg: &Config, app: &Path) -> PathBuf {
    if !cfg.jonahbox.binary.is_empty() { return PathBuf::from(&cfg.jonahbox.binary); }
    dir(cfg, app).join("target/release").join(if cfg!(windows) { "jonahbox.exe" } else { "jonahbox" })
}
fn certs_dir(cfg: &Config, app: &Path) -> PathBuf { dir(cfg, app).join("certs") }
pub fn ip(cfg: &Config) -> String { if cfg.jonahbox.ip.is_empty() { local_server::lan_ip() } else { cfg.jonahbox.ip.clone() } }

#[derive(Serialize)]
pub struct Status { pub installed: bool, pub built: bool, pub cache: bool, pub certs: bool, pub host: String, pub ip: String, pub ca: String }
pub fn status(cfg: &Config, app: &Path) -> Status {
    let d = dir(cfg, app);
    Status { installed: d.join("Cargo.toml").is_file(), built: binary(cfg, app).is_file(), cache: d.join("jb_cache/jackbox.tv/index.html").is_file(),
        certs: certs_dir(cfg, app).join("server.pem").is_file(), host: cfg.jonahbox.host.clone(), ip: ip(cfg),
        ca: certs_dir(cfg, app).join("ca.pem").display().to_string() }
}

/// 1. Télécharge le code source de Jonahbox (AGPL-3.0) depuis `[jonahbox].repo`.
pub fn install(cfg: &Config, app: &Path) -> anyhow::Result<usize> {
    let url = format!("{}/archive/refs/heads/{}.zip", cfg.jonahbox.repo.trim_end_matches('/'), cfg.jonahbox.branch);
    let t = patch::download(&url)?;
    patch::extract(None, &t.0, &dir(cfg, app), true)
}

/// 2. Compile avec cargo (long : plusieurs minutes). Alternative : renseigner `[jonahbox].binary`.
pub fn build(cfg: &Config, app: &Path) -> anyhow::Result<String> {
    let d = dir(cfg, app);
    if !d.join("Cargo.toml").is_file() { anyhow::bail!("Jonahbox n'est pas téléchargé (étape 1)"); }
    if Command::new("cargo").arg("--version").output().is_err() { anyhow::bail!("cargo introuvable : installe Rust (https://rustup.rs) ou renseigne [jonahbox].binary"); }
    let log = fs::File::create(d.join("build.log"))?;
    let st = Command::new("cargo").args(["build", "--release"]).current_dir(&d).stdout(log.try_clone()?).stderr(log).status()?;
    if !st.success() { anyhow::bail!("compilation échouée, voir {}", d.join("build.log").display()); }
    Ok(binary(cfg, app).display().to_string())
}

/// 3. Convertit le dump jackbox.tv en cache de Jonahbox (traduction + redirection des adresses vers `host`).
pub fn cache(cfg: &Config, app: &Path, force: bool) -> anyhow::Result<String> {
    let tr = Translator::load(app, &cfg.local.dump_language)?;
    let st = jb_cache::build_cache(&local_server::dump_path(cfg, app), &dir(cfg, app).join("jb_cache"), &cfg.jonahbox.host, &tr, force)?;
    Ok(format!("{} entrées ajoutées, {} déjà présentes, {} adresses redirigées vers {} dans {} fichiers", st.added, st.skipped, st.rewritten_refs, cfg.jonahbox.host, st.rewritten_files))
}

/// 4. Autorité de certification locale (créée une fois) + certificat serveur valable 800 jours (limite Apple) pour IP, hôte et noms blobcast.
pub fn certs(cfg: &Config, app: &Path) -> anyhow::Result<String> {
    let d = certs_dir(cfg, app);
    fs::create_dir_all(&d)?;
    let key_path = d.join("ca-key.pem");
    let ca_key = if key_path.is_file() { KeyPair::from_pem(&fs::read_to_string(&key_path)?)? } else { let k = KeyPair::generate()?; fs::write(&key_path, k.serialize_pem())?; k };
    let mut p = CertificateParams::new(Vec::<String>::new())?;
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name.push(DnType::CommonName, CA_NAME);
    p.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca = p.self_signed(&ca_key)?;
    if !d.join("ca.pem").is_file() { fs::write(d.join("ca.pem"), ca.pem())?; } // on garde l'original : c'est lui que l'utilisateur a installé

    let host = cfg.jonahbox.host.clone();
    let sans = vec![host.clone(), ip(cfg), "127.0.0.1".into(), "localhost".into(), "blobcast.jackboxgames.com".into(), "blobcast-test.jackboxgames.com".into()];
    let mut lp = CertificateParams::new(sans)?;
    lp.distinguished_name.push(DnType::CommonName, host.as_str());
    lp.not_before = time::OffsetDateTime::now_utc() - time::Duration::days(1);
    lp.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(800);
    lp.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let lk = KeyPair::generate()?;
    let leaf = lp.signed_by(&lk, &ca, &ca_key)?;
    fs::write(d.join("server.pem"), leaf.pem())?;
    fs::write(d.join("server-key.pem"), lk.serialize_pem())?;
    Ok(format!("certificats dans {} ; installe {} sur les appareils (téléphones) et via l'étape « confiance » sur ce PC", d.display(), d.join("ca.pem").display()))
}

/// 5. Installe (ou retire) l'autorité dans le magasin de confiance du système. Nécessite les droits administrateur sous Windows.
pub fn trust(cfg: &Config, app: &Path, on: bool) -> anyhow::Result<String> {
    let ca = certs_dir(cfg, app).join("ca.pem");
    if on && !ca.is_file() { anyhow::bail!("génère d'abord les certificats (étape 4)"); }
    let out = if cfg!(windows) {
        let mut c = Command::new("certutil");
        if on { c.args(["-addstore", "-f", "Root"]).arg(&ca); } else { c.args(["-delstore", "Root", CA_NAME]); }
        c.output()?
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("security");
        let kc = format!("{}/Library/Keychains/login.keychain-db", std::env::var("HOME").unwrap_or_default());
        if on { c.args(["add-trusted-cert", "-d", "-r", "trustRoot", "-k", &kc]).arg(&ca); } else { c.args(["delete-certificate", "-c", CA_NAME]); }
        c.output()?
    } else {
        anyhow::bail!("Linux : sudo cp {} /usr/local/share/ca-certificates/jbx-ca.crt && sudo update-ca-certificates (Firefox/Chrome ont parfois leur propre magasin)", ca.display());
    };
    if !out.status.success() {
        anyhow::bail!("{}{} — droits administrateur requis ?", String::from_utf8_lossy(&out.stdout).trim(), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(if on { "autorité de certification installée" } else { "autorité de certification retirée" }.into())
}

/// 6. Ajoute/retire `IP hôte # jbx` dans le fichier hosts (droits administrateur requis). pp1 = noms blobcast du Party Pack 1 (expérimental).
pub fn hosts(cfg: &Config, on: bool, pp1: bool) -> anyhow::Result<String> {
    let path = if cfg!(windows) { PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())).join("System32/drivers/etc/hosts") } else { PathBuf::from("/etc/hosts") };
    let mut names = vec![cfg.jonahbox.host.as_str()];
    if pp1 { names.extend(["blobcast.jackboxgames.com", "blobcast-test.jackboxgames.com"]); }
    let new = jb_cache::hosts_edit(&fs::read_to_string(&path)?, &ip(cfg), &names, on)?;
    fs::write(&path, new).map_err(|e| anyhow::anyhow!("{e} — droits administrateur requis pour modifier {}", path.display()))?;
    Ok(if on { format!("{} → {} ajouté dans {}", ip(cfg), names.join(", "), path.display()) } else { "lignes # jbx retirées".into() })
}

/// Commandes pare-feu / redirection de ports à lancer soi-même (administrateur) : on ne les exécute pas à ta place.
pub fn commands(cfg: &Config) -> String {
    let (j, ip) = (&cfg.jonahbox, ip(cfg));
    if cfg!(windows) {
        let mut s = format!("netsh advfirewall firewall add rule name=\"Jonahbox\" dir=in action=allow protocol=TCP localport={},{},{}\n", j.https_port, j.http_port, j.blobcast_port);
        for p in [j.https_port, j.http_port, j.blobcast_port] { s += &format!("netsh interface portproxy add v4tov6 listenaddress={ip} listenport={p} connectaddress=::1 connectport={p}\n"); }
        s + "# pour annuler : netsh interface portproxy reset"
    } else {
        format!("sudo ufw allow {}/tcp && sudo ufw allow {}/tcp && sudo ufw allow {}/tcp\n# ports < 1024 sans root : sudo setcap 'cap_net_bind_service=+ep' <binaire jonahbox>", j.https_port, j.http_port, j.blobcast_port)
    }
}

/// 7. Écrit config.toml de Jonahbox et lance le serveur (journal : jonahbox.log).
pub fn start(cfg: &Config, app: &Path) -> anyhow::Result<Child> {
    let bin = binary(cfg, app);
    if !bin.is_file() { anyhow::bail!("binaire introuvable : {} (étape 2, ou [jonahbox].binary)", bin.display()); }
    let c = certs_dir(cfg, app);
    let (cert, key) = (c.join("server.pem"), c.join("server-key.pem"));
    if !cert.is_file() { anyhow::bail!("certificats manquants (étape 4)"); }
    let d = dir(cfg, app);
    fs::create_dir_all(&d)?;
    let j = &cfg.jonahbox;
    fs::write(d.join("config.toml"), jb_cache::render_config(&jb_cache::CfgParams { host: &j.host, ip: &ip(cfg), cache_mode: &j.cache_mode,
        cert: &cert.to_string_lossy(), key: &key.to_string_lossy(), https: j.https_port, http: j.http_port, blobcast: j.blobcast_port }))?;
    let log = fs::File::create(d.join("jonahbox.log"))?;
    let mut cmd = Command::new(&bin);
    cmd.current_dir(&d).stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log);
    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x0800_0000); }
    Ok(cmd.spawn()?)
}

pub fn logs(cfg: &Config, app: &Path, n: usize) -> String {
    let t = fs::read_to_string(dir(cfg, app).join("jonahbox.log")).unwrap_or_default();
    let l: Vec<&str> = t.lines().collect();
    l[l.len().saturating_sub(n)..].join("\n")
}
