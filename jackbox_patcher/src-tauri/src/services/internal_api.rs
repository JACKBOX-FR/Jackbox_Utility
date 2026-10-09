//! API REST + WebSocket locale (port de internal_api) : /api/status, /api/register, /api/games/list, /api/games/open/:game, /ws (navigation).
//! Écoute 127.0.0.1 uniquement. Une extension doit être acceptée par l'utilisateur (événement "extension-request").
use crate::model::{config::Config, state};
use crate::services::{api_utility as catalog, launcher};
use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path as UPath, State}, http::{HeaderMap, StatusCode}, response::Response, routing::{get, post}, Json, Router};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::{atomic::{AtomicU64, Ordering}, Arc, Mutex}, time::Duration};
use tauri::Emitter;
use tokio::sync::{broadcast, oneshot};
use tower_http::cors::CorsLayer;

const SCOPES: [&str; 2] = ["navigation", "game_open_close"];
type Res = (StatusCode, Json<Value>);
fn err(c: StatusCode, m: &str) -> Res { (c, Json(json!({ "error": m, "status": "error" }))) }

pub struct Api {
    dir: PathBuf, app: tauri::AppHandle,
    tokens: Mutex<Vec<(String, Vec<String>)>>, // en mémoire : les extensions se ré-enregistrent à chaque lancement
    pending: Mutex<HashMap<u64, oneshot::Sender<bool>>>, next: AtomicU64,
    events: broadcast::Sender<String>, // événements de navigation poussés aux WebSocket abonnés
}

impl Api {
    pub fn new(dir: PathBuf, app: tauri::AppHandle) -> Arc<Self> {
        Arc::new(Self { dir, app, tokens: Mutex::new(vec![]), pending: Mutex::new(HashMap::new()), next: AtomicU64::new(1), events: broadcast::channel(64).0 })
    }
    /// Réponse de l'utilisateur à une demande d'extension.
    pub fn answer(&self, id: u64, accept: bool) {
        if let Some(tx) = self.pending.lock().unwrap().remove(&id) { let _ = tx.send(accept); }
    }
    fn has_scope(&self, token: &str, scope: &str) -> bool {
        self.tokens.lock().unwrap().iter().any(|(k, sc)| k == token && sc.iter().any(|x| x == scope))
    }
    /// Pousse {"channel": ..., "data": ...} à tous les WebSocket (scope navigation).
    pub fn push(&self, channel: &str, data: Value) { let _ = self.events.send(json!({ "channel": channel, "data": data }).to_string()); }
    /// Après un lancement de jeu : détecte l'ouverture puis la fermeture du pack (game_open / game_close).
    pub fn watch(self: &Arc<Self>, l: launcher::Launched) {
        let Some(game) = l.game.clone() else { return };
        let s = self.clone();
        std::thread::spawn(move || {
            for _ in 0..24 { // attend jusqu'à ~2 min que le pack apparaisse
                std::thread::sleep(Duration::from_secs(5));
                if launcher::is_running(&l.exe) {
                    s.push("game_open", game.clone());
                    loop {
                        std::thread::sleep(Duration::from_secs(5));
                        if !launcher::is_running(&l.exe) { s.push("game_close", game.clone()); return; }
                    }
                }
            }
        });
    }
    fn check(&self, h: &HeaderMap, scope: &str) -> Result<(), Res> {
        let t = h.get("authorization").and_then(|v| v.to_str().ok()).ok_or_else(|| err(StatusCode::FORBIDDEN, "No token provided"))?;
        let g = self.tokens.lock().unwrap();
        let (_, sc) = g.iter().find(|(k, _)| k == t).ok_or_else(|| err(StatusCode::FORBIDDEN, "Invalid token"))?;
        if sc.iter().any(|x| x == scope) { Ok(()) } else { Err(err(StatusCode::FORBIDDEN, "Invalid scope")) }
    }
}

async fn status() -> Res {
    (StatusCode::OK, Json(json!({ "version": env!("CARGO_PKG_VERSION"), "appName": "Jackbox Utility RS", "status": "ok" })))
}

async fn register(State(s): State<Arc<Api>>, Json(body): Json<Value>) -> Res {
    let scopes: Vec<String> = body["scopes"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if scopes.iter().any(|x| !SCOPES.contains(&x.as_str())) { return err(StatusCode::BAD_REQUEST, "unknown scope"); }
    let (tx, rx) = oneshot::channel();
    let id = s.next.fetch_add(1, Ordering::Relaxed);
    {
        let mut p = s.pending.lock().unwrap();
        if !p.is_empty() { return err(StatusCode::TOO_MANY_REQUESTS, "a request is already pending"); }
        p.insert(id, tx);
    }
    let _ = s.app.emit("extension-request", json!({ "id": id, "extension": body["name"], "scopes": scopes }));
    let ok = matches!(tokio::time::timeout(Duration::from_secs(60), rx).await, Ok(Ok(true)));
    s.pending.lock().unwrap().remove(&id);
    if !ok { return err(StatusCode::FORBIDDEN, "Extension not accepted"); }
    let mut b = [0u8; 32];
    if getrandom::getrandom(&mut b).is_err() { return err(StatusCode::INTERNAL_SERVER_ERROR, "no randomness"); }
    let token: String = b.iter().map(|x| format!("{x:02x}")).collect();
    s.tokens.lock().unwrap().push((token.clone(), scopes));
    (StatusCode::OK, Json(json!({ "token": token })))
}

async fn games(State(s): State<Arc<Api>>, h: HeaderMap) -> Res {
    if let Err(e) = s.check(&h, "navigation") { return e; }
    let dir = s.dir.clone();
    let r = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<catalog::CatalogView> {
        let url = state::State::load(&dir).last_server.ok_or_else(|| anyhow::anyhow!("no server selected in the app"))?;
        catalog::view_packs(&dir, &url)
    }).await;
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(json!({ "status": "ok", "data": v.packs }))),
        Ok(Err(e)) => err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn open_game(State(s): State<Arc<Api>>, h: HeaderMap, UPath(game): UPath<String>) -> Res {
    if let Err(e) = s.check(&h, "game_open_close") { return e; }
    let dir = s.dir.clone();
    let r = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<launcher::Launched> {
        launcher::launch_raw(&dir, &Config::load(&dir)?, "game", &game)
    }).await;
    match r {
        Ok(Ok(l)) => { s.watch(l); (StatusCode::OK, Json(json!({ "status": "ok" }))) }
        Ok(Err(e)) if e.to_string().contains("introuvable") => err(StatusCode::NOT_FOUND, "Game not found"),
        Ok(Err(e)) => err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Protocole (identique à l'appli d'origine) : le client envoie {"token":"..."} ; réponse {"status":"ok"} ; ensuite événements.
async fn ws(State(s): State<Arc<Api>>, up: WebSocketUpgrade) -> Response { up.on_upgrade(move |sock| handle_ws(s, sock)) }

async fn handle_ws(s: Arc<Api>, mut sock: WebSocket) {
    let first = tokio::time::timeout(Duration::from_secs(10), sock.recv()).await;
    let token = match first {
        Ok(Some(Ok(Message::Text(t)))) => serde_json::from_str::<Value>(t.as_str()).ok().and_then(|v| v["token"].as_str().map(String::from)),
        _ => None,
    };
    let Some(token) = token.filter(|t| s.has_scope(t, "navigation")) else {
        let _ = sock.send(Message::Text(json!({ "status": "error", "error": "Invalid token or scope" }).to_string().into())).await;
        return;
    };
    let mut rx = s.events.subscribe();
    if sock.send(Message::Text(json!({ "status": "ok" }).to_string().into())).await.is_err() { return; }
    let _ = token;
    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Ok(m) => { if sock.send(Message::Text(m.into())).await.is_err() { break; } }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
            msg = sock.recv() => match msg { Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break, _ => {} },
        }
    }
}

pub async fn serve(api: Arc<Api>, port: u16) {
    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/register", post(register))
        .route("/api/games/list", get(games))
        .route("/api/games/open/{game}", post(open_game))
        .route("/ws", get(ws))
        .layer(CorsLayer::permissive())
        .with_state(api);
    if let Ok(l) = tokio::net::TcpListener::bind(("127.0.0.1", port)).await { let _ = axum::serve(l, app).await; }
}
