use crate::{model::config::Config, services::translations::Translator};
use axum::{body::{to_bytes, Body, Bytes}, extract::{Request, State}, http::{header, StatusCode}, middleware::{from_fn_with_state, Next}, response::Response, Router};
use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, Mutex}};
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tower_http::{cors::CorsLayer, services::ServeDir};

pub struct Running { pub handle: tauri::async_runtime::JoinHandle<()>, pub upnp_port: Option<u16> }
#[derive(Serialize)]
pub struct LocalInfo { pub localhost: String, pub lan: String, pub external: Option<String>, pub scheme: String, pub note: Option<String> }

/// Serveur web statique du dump. https = certificat auto-signé ; upnp = ouverture du port sur la box.
/// Un chemin relatif est résolu dans le dossier de config (et non le dossier courant, imprévisible pour une appli graphique).
pub fn dump_path(cfg: &Config, app_dir: &Path) -> PathBuf {
    let p = Path::new(&cfg.local.dump_dir);
    if p.is_absolute() { p.to_path_buf() } else { app_dir.join(p) }
}

/// Télécharge / met à jour le dump (copie locale du site) depuis `dump_repo`.
pub fn update_dump(cfg: &Config, app_dir: &Path) -> anyhow::Result<usize> {
    let url = format!("{}/archive/refs/heads/{}.zip", cfg.local.dump_repo.trim_end_matches('/'), cfg.local.dump_branch);
    let t = crate::services::downloader::download(&url)?;
    crate::services::downloader::extract(None, &t.0, &dump_path(cfg, app_dir), true)
}

#[derive(Clone)]
struct Tx { tr: Arc<Translator>, cache: Arc<Mutex<HashMap<String, Bytes>>> }

/// Applique translations/<langue>.toml aux fichiers texte servis (js/html/json/css), avec cache mémoire.
async fn translate(State(t): State<Tx>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let res = next.run(req).await;
    if t.tr.is_empty() || res.status() != StatusCode::OK { return res; }
    let ct = res.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    if !["javascript", "html", "json", "css"].iter().any(|k| ct.contains(k)) { return res; }
    let (mut parts, body) = res.into_parts();
    parts.headers.remove(header::CONTENT_LENGTH);
    let cached = t.cache.lock().unwrap().get(&path).cloned();
    let bytes = match cached {
        Some(b) => b,
        None => {
            let raw = match to_bytes(body, 256 * 1024 * 1024).await { Ok(b) => b, Err(_) => return Response::builder().status(500).body(Body::empty()).unwrap() };
            let out = match std::str::from_utf8(&raw) { Ok(s) => Bytes::from(t.tr.apply(s)), Err(_) => raw };
            t.cache.lock().unwrap().insert(path, out.clone());
            out
        }
    };
    Response::from_parts(parts, Body::from(bytes))
}

pub async fn start(cfg: &Config, app_dir: &Path) -> anyhow::Result<(Running, LocalInfo)> {
    let addr: SocketAddr = format!("{}:{}", cfg.local.bind, cfg.local.port).parse()?;
    let dump = dump_path(cfg, app_dir);
    if !dump.is_dir() { anyhow::bail!("dump introuvable : {} (clique « Dump » pour le télécharger)", dump.display()); }
    let tx = Tx { tr: Arc::new(Translator::load(app_dir, &cfg.local.dump_language)?), cache: Arc::new(Mutex::new(HashMap::new())) };
    let app = Router::new().fallback_service(ServeDir::new(dump)).layer(from_fn_with_state(tx, translate)).layer(CorsLayer::permissive());
    let std_l = std::net::TcpListener::bind(addr)?; // échoue tout de suite si le port est pris
    std_l.set_nonblocking(true)?;
    let lan = lan_ip();
    let handle = if cfg.local.https {
        let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string(), lan.clone()])?;
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem(ck.cert.pem().into_bytes(), ck.key_pair.serialize_pem().into_bytes()).await?;
        let srv = axum_server::from_tcp_rustls(std_l, tls)?; // axum-server 0.8 : renvoie un Result
        tauri::async_runtime::spawn(async move { let _ = srv.serve(app.into_make_service()).await; })
    } else {
        let l = tokio::net::TcpListener::from_std(std_l)?;
        tauri::async_runtime::spawn(async move { let _ = axum::serve(l, app).await; })
    };
    let (mut external, mut note, mut upnp_port) = (None, None, None);
    if cfg.local.upnp {
        let (ip, port) = (lan.clone(), cfg.local.port);
        match tauri::async_runtime::spawn_blocking(move || upnp_open(&ip, port)).await? {
            Ok(ext) => { external = Some(format!("{ext}:{port}")); upnp_port = Some(port); }
            Err(e) => note = Some(format!("UPnP : {e}")),
        }
    }
    let info = LocalInfo { localhost: format!("localhost:{}", cfg.local.port), lan: format!("{lan}:{}", cfg.local.port), external, scheme: if cfg.local.https { "https" } else { "http" }.into(), note };
    Ok((Running { handle, upnp_port }, info))
}

/// wait=false : l'UI ne se fige pas ; wait=true : à la fermeture de l'appli (retire la redirection de port).
pub fn stop(r: Running, wait: bool) {
    r.handle.abort();
    if let Some(p) = r.upnp_port {
        let f = move || { let _ = igd_next::search_gateway(Default::default()).map(|g| g.remove_port(igd_next::PortMappingProtocol::TCP, p)); };
        if wait { f() } else { std::thread::spawn(f); }
    }
}

fn upnp_open(ip: &str, port: u16) -> anyhow::Result<String> {
    let gw = igd_next::search_gateway(Default::default())?;
    let ip4: Ipv4Addr = ip.parse()?;
    gw.add_port(igd_next::PortMappingProtocol::TCP, port, SocketAddr::V4(SocketAddrV4::new(ip4, port)), 0, "jbx")?;
    Ok(gw.get_external_ip()?.to_string())
}

/// IP locale utilisable par les autres machines du réseau.
pub fn lan_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0").and_then(|s| { s.connect("8.8.8.8:80")?; s.local_addr() })
        .map(|a| a.ip().to_string()).unwrap_or_else(|_| "127.0.0.1".into())
}
