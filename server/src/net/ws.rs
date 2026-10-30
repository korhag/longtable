// ------------------------------------------------------------
// Module: ws
// Purpose: Accept browsers, check the passphrase, and carry
//          JSON messages. Map and portrait uploads arrive as
//          HTTP because they are files, not game commands.
// ------------------------------------------------------------

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, DefaultBodyLimit, FromRequest, Multipart, Path, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::app::App;
use crate::dice::simulate;
use crate::systems::{rules_for, save_system};
use crate::map::compile::compile;
use crate::map::drafts::{check_revision, safe_id, Draft, DraftStore};
use crate::map::layout::MapLayout;
use crate::map::{Scene, DIMETRIC_PROFILE};
use crate::net::protocol::{dice_meshes, ClientMsg, Keyframe, PlayedDie, RollEvent, ServerMsg};
use crate::session::state::{now_ms, CommandEffect};

pub fn router(app: Arc<App>) -> Router {
    let maps = Router::new()
        .route("/api/maps/import", post(import_map))
        .layer(DefaultBodyLimit::max(512 * 1024 * 1024))
        .with_state(app.clone());
    let pack_import = Router::new()
        .route("/api/packs/import", post(import_pack))
        .route("/api/packs/check", post(check_pack))
        .layer(DefaultBodyLimit::max(crate::asset_import::MAX_COMPRESSED + 1024 * 1024))
        .with_state(app.clone());
    let mut router = Router::new()
        .route("/api/health", get(health))
        .route("/api/host", get(host_info))
        .route("/api/register", post(crate::net::accounts::register))
        .route("/api/login", post(crate::net::accounts::login))
        .route("/api/me", get(crate::net::accounts::me).patch(crate::net::accounts::update_me))
        .route("/api/tables", post(crate::net::accounts::create_table))
        .route("/api/tables/{id}/open", post(crate::net::accounts::open_table))
        .route("/api/tables/{id}/request", post(crate::net::accounts::request_seat))
        .route("/api/tables/{id}/approve", post(crate::net::accounts::approve))
        .route("/api/friends", post(crate::net::accounts::friend_request))
        .route("/api/friends/{id}/accept", post(crate::net::accounts::accept_friend))
        .route("/api/maps/{id}/image", get(map_image))
        .route("/api/maps/{id}/assets/{*path}", get(map_asset))
        .route("/api/tokens/{id}/image", post(upload_token_image).get(token_image))
        .route("/api/packs", get(packs))
        .route("/api/drafts", get(list_drafts).post(create_draft))
        .route("/api/drafts/{id}", get(read_draft).put(write_draft).delete(remove_draft))
        .route("/api/drafts/{id}/publish", post(publish_draft))
        .route("/ws", get(websocket))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(app.clone())
        .merge(maps)
        .merge(pack_import);

    let packs_dir = ServeDir::new(app.assets_dir.join("packs"));
    router = router.nest_service("/packs", packs_dir);

    let index = app.static_dir.join("index.html");
    if index.is_file() {
        let files = ServeDir::new(&app.static_dir).not_found_service(ServeFile::new(index));
        router = router.fallback_service(files);
        tracing::info!("serving the client from {}", app.static_dir.display());
    } else {
        tracing::info!("no built client at {} (fine while Vite is running)", app.static_dir.display());
    }
    router.layer(axum::middleware::from_fn(cache_client))
}

/// The page must be checked every visit, or a rebuild stays invisible.
/// Hashed files under /assets keep their long cache.
async fn cache_client(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    if path.starts_with("/api/") || path.starts_with("/packs/") || path == "/ws" {
        return response;
    }
    let value = if path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static(value));
    response
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

async fn packs(State(app): State<Arc<App>>) -> Json<crate::assets::Catalog> {
    Json(app.catalog.lock().await.clone())
}

async fn import_pack(State(app): State<Arc<App>>, request: Request) -> Result<Json<crate::assets::Catalog>, ApiError> {
    require_gm(&app, request.headers()).await?;
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?;
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?
    {
        if field.name() == Some("pack") {
            bytes = Some(field.bytes().await.map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?);
        }
    }
    let bytes = bytes.ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "choose a pack archive".into()))?;
    if bytes.len() > crate::asset_import::MAX_COMPRESSED {
        return Err(ApiError(StatusCode::BAD_REQUEST, "that pack is larger than 100 MiB compressed".into()));
    }
    let assets_dir = app.assets_dir.clone();
    let owned = bytes.to_vec();
    let pack = tokio::task::spawn_blocking(move || crate::asset_import::import_bytes(&owned, &assets_dir))
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "the importer stopped".into()))?
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    let assets_dir = app.assets_dir.clone();
    let mut catalog = tokio::task::spawn_blocking(move || crate::assets::load_catalog_mode(&assets_dir, false))
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "the library could not be read".into()))?
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    if !catalog.packs.iter().any(|item| item.id == pack.id) {
        catalog.packs.push(pack);
    }
    let revision = {
        let table = app.table.lock().await;
        bump_revision(&table.catalog_revision)
    };
    catalog.revision = revision.clone();
    *app.catalog.lock().await = catalog.clone();
    {
        let mut table = app.table.lock().await;
        table.catalog_revision = revision;
        table.note_cast_footprints(&catalog);
        table.broadcast_state();
    }
    Ok(Json(catalog))
}

async fn check_pack(State(app): State<Arc<App>>, request: Request) -> Result<Json<crate::asset_import::PackReport>, ApiError> {
    require_gm(&app, request.headers()).await?;
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?;
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?
    {
        if field.name() == Some("pack") {
            bytes = Some(field.bytes().await.map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?);
        }
    }
    let bytes = bytes.ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "choose a pack archive".into()))?;
    let owned = bytes.to_vec();
    let report = tokio::task::spawn_blocking(move || crate::asset_import::check_bytes(&owned))
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "the checker stopped".into()))?
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    Ok(Json(report))
}

fn bump_revision(current: &str) -> String {
    let number = current.split(':').next().unwrap_or("0").parse::<u32>().unwrap_or(0).saturating_add(1);
    format!("{number}")
}

async fn require_gm(app: &App, headers: &HeaderMap) -> Result<String, ApiError> {
    let token = bearer(headers).ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in first".into()))?;
    let table = app.table.lock().await;
    let viewer = table
        .viewer_for_session(&token)
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()))?;
    if viewer.role != crate::net::protocol::Role::Gm {
        return Err(ApiError(StatusCode::FORBIDDEN, "only the GM can build the map".into()));
    }
    Ok(token)
}

async fn list_drafts(State(app): State<Arc<App>>, headers: HeaderMap) -> Result<Json<Vec<crate::map::drafts::DraftSummary>>, ApiError> {
    require_gm(&app, &headers).await?;
    Ok(Json(DraftStore::new(&app.data_dir).list()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewDraftBody {
    #[serde(default)]
    name: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    copy_live: bool,
}

async fn create_draft(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(body): Json<NewDraftBody>,
) -> Result<Json<Draft>, ApiError> {
    require_gm(&app, &headers).await?;
    let layout = if body.copy_live {
        let table = app.table.lock().await;
        table
            .scene
            .as_ref()
            .and_then(|scene| scene.layout.clone())
            .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "the table has no built map to copy".into()))?
    } else {
        let width = if body.width == 0 { 30 } else { body.width };
        let height = if body.height == 0 { 20 } else { body.height };
        if !(5..=200).contains(&width) || !(5..=200).contains(&height) {
            return Err(ApiError(StatusCode::BAD_REQUEST, "a map is between 5 and 200 squares on a side".into()));
        }
        MapLayout::blank(width, height, "core/grass")
    };
    let trimmed = body.name.trim();
    let draft = Draft {
        id: Uuid::new_v4().to_string(),
        name: if trimmed.is_empty() { "Untitled map".into() } else { trimmed.chars().take(48).collect() },
        updated_at: now_ms(),
        layout,
        scene_id: None,
    };
    DraftStore::new(&app.data_dir)
        .put(&draft)
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    Ok(Json(draft))
}

async fn read_draft(State(app): State<Arc<App>>, Path(id): Path<String>, headers: HeaderMap) -> Result<Json<Draft>, ApiError> {
    require_gm(&app, &headers).await?;
    let draft = DraftStore::new(&app.data_dir)
        .get(&id)
        .map_err(|err| ApiError(StatusCode::NOT_FOUND, err))?;
    Ok(Json(draft))
}

async fn write_draft(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(mut draft): Json<Draft>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_gm(&app, &headers).await?;
    draft.id = safe_id(&id).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    let store = DraftStore::new(&app.data_dir);
    let stored = store.get(&draft.id).ok().map(|existing| existing.updated_at);
    check_revision(stored, draft.updated_at).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    draft.updated_at = now_ms();
    store.put(&draft).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    Ok(Json(serde_json::json!({ "updatedAt": draft.updated_at })))
}

async fn remove_draft(State(app): State<Arc<App>>, Path(id): Path<String>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    require_gm(&app, &headers).await?;
    DraftStore::new(&app.data_dir)
        .delete(&id)
        .map_err(|err| ApiError(StatusCode::NOT_FOUND, err))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn publish_draft(State(app): State<Arc<App>>, Path(id): Path<String>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    let token = require_gm(&app, &headers).await?;
    let draft = DraftStore::new(&app.data_dir)
        .get(&id)
        .map_err(|err| ApiError(StatusCode::NOT_FOUND, err))?;
    let catalog = app.catalog.lock().await.clone();
    let compiled = compile(&draft.layout, &catalog).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    let mut table = app.table.lock().await;
    let viewer = table
        .viewer_for_session(&token)
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()))?;
    let scene_id = table
        .publish_built(&viewer, draft.name.clone(), draft.layout.clone(), compiled, draft.scene_id.clone())
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    table.broadcast_state();
    drop(table);
    let mut saved = draft;
    saved.scene_id = Some(scene_id);
    saved.updated_at = now_ms();
    DraftStore::new(&app.data_dir).put(&saved).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    app.request_save();
    Ok(StatusCode::NO_CONTENT)
}

/// The computer running the game can read its own key. Other machines cannot.
async fn host_info(ConnectInfo(peer): ConnectInfo<SocketAddr>, State(app): State<Arc<App>>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !peer.ip().is_loopback() {
        return Err(StatusCode::NOT_FOUND);
    }
    let table = app.table.lock().await;
    Ok(Json(json!({
        "gmKey": table.gm_key,
        "passphrase": table.passphrase,
    })))
}

async fn account_seat(
    app: &Arc<App>,
    token: &str,
    table_id: Option<&str>,
    resume_id: Option<&str>,
) -> Result<crate::session::Seat, String> {
    let table_id = table_id.filter(|id| !id.is_empty()).ok_or_else(|| "pick a table".to_string())?;
    let (account, door) = {
        let house = app.house.lock().await;
        let account = house.account(token).ok_or_else(|| "sign in again".to_string())?;
        if house.open_id().as_deref() != Some(table_id) {
            return Err("that table is not open".into());
        }
        let door = house.door(&account.id, table_id);
        (account, door)
    };
    let (as_gm, resume, consume) = match door {
        crate::house::Door::Owner { seat } => (true, seat.or_else(|| clean_resume(resume_id)), false),
        crate::house::Door::Always { seat } => (false, seat.or_else(|| clean_resume(resume_id)), false),
        crate::house::Door::Once { seat } => (false, seat, true),
        crate::house::Door::Spent { seat } => {
            let client = clean_resume(resume_id);
            if client.is_some() && client == seat {
                (false, client, false)
            } else {
                return Err("ask the GM before you sit again".into());
            }
        }
        crate::house::Door::Closed => return Err("ask the GM before you sit".into()),
    };
    if !as_gm {
        let table = app.table.lock().await;
        let gm_here = table.players.iter().any(|player| player.role == crate::net::protocol::Role::Gm && player.connected);
        if !gm_here {
            return Err("the GM is not at the table".into());
        }
    }
    let seat = {
        let mut table = app.table.lock().await;
        let passphrase = table.passphrase.clone();
        let gm_key = table.gm_key.clone();
        let seated = if as_gm {
            table.take_seat(&account.username, &passphrase, true, Some(gm_key.as_str()), resume.as_deref())?
        } else {
            table.take_seat(&account.username, &passphrase, false, None, resume.as_deref())?
        };
        if let Some(player) = table.players.iter_mut().find(|player| player.id == seated.player_id) {
            player.name = account.username.clone();
            player.color = account.color.clone();
        }
        seated
    };
    {
        let mut house = app.house.lock().await;
        house.remember_seat(table_id, &account.id, &seat.player_id, consume);
    }
    Ok(seat)
}

fn clean_resume(resume_id: Option<&str>) -> Option<String> {
    resume_id.filter(|id| !id.is_empty()).map(|id| id.to_string())
}

async fn websocket(ws: WebSocketUpgrade, State(app): State<Arc<App>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn handle_socket(socket: WebSocket, app: Arc<App>) {
    let (mut sink, mut stream) = socket.split();
    let first = match tokio::time::timeout(Duration::from_secs(20), stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => text.to_string(),
        _ => return,
    };

    let join = match serde_json::from_str::<ClientMsg>(&first) {
        Ok(ClientMsg::Join { name, passphrase, as_gm, gm_key, resume_id, account_token, table_id }) => {
            if let Some(token) = account_token.filter(|token| !token.is_empty()) {
                account_seat(&app, &token, table_id.as_deref(), resume_id.as_deref()).await
            } else {
                let mut table = app.table.lock().await;
                table.take_seat(&name, &passphrase, as_gm, gm_key.as_deref(), resume_id.as_deref())
            }
        }
        Ok(_) => Err("say who you are before anything else".into()),
        Err(_) => Err("could not read that message".into()),
    };

    let seat = match join {
        Ok(seat) => seat,
        Err(message) => {
            let _ = sink.send(Message::Text(json_error(&message).into())).await;
            return;
        }
    };

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<ServerMsg>();
    let (pong_tx, mut pong_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let attached = {
        let mut table = app.table.lock().await;
        table.attach(&seat.player_id, out_tx)
    };
    let Ok((epoch, session_token)) = attached else {
        let _ = sink.send(Message::Text(json_error("that seat is gone").into())).await;
        return;
    };

    let player_id = seat.player_id.clone();
    let welcome = ServerMsg::Welcome {
        player_id: player_id.clone(),
        role: seat.role,
        session_token,
        server_time: now_ms(),
        dice: dice_meshes(),
    };
    let systems = app.systems.lock().await.list.clone();
    {
        let table = app.table.lock().await;
        table.send_to(&player_id, epoch, welcome);
        table.send_to(&player_id, epoch, ServerMsg::Systems { systems });
        table.broadcast_state();
    }
    tracing::info!("{} sat down as {:?}", seat.player_id, seat.role);

    let writer = tokio::spawn(async move {
        let mut pong_open = true;
        loop {
            tokio::select! {
                message = out_rx.recv() => {
                    let Some(message) = message else { break };
                    let Ok(text) = serde_json::to_string(&message) else { continue };
                    if sink.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                pong = pong_rx.recv(), if pong_open => {
                    match pong {
                        Some(pong) => {
                            if sink.send(Message::Pong(pong.into())).await.is_err() {
                                break;
                            }
                        }
                        None => pong_open = false,
                    }
                }
            }
        }
    });

    while let Some(incoming) = stream.next().await {
        let Ok(incoming) = incoming else { break };
        match incoming {
            Message::Ping(payload) => {
                if pong_tx.send(payload.to_vec()).is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            Message::Text(text) => {
                let message = match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(message) => message,
                    Err(_) => {
                        reply_error(&app, &player_id, epoch, "could not read that message").await;
                        continue;
                    }
                };
                if !dispatch(&app, &player_id, epoch, message).await {
                    break;
                }
            }
            _ => {}
        }
    }

    {
        let mut table = app.table.lock().await;
        table.detach(&player_id, epoch);
        table.broadcast_state();
    }
    app.request_save();
    // The writer drains anything still queued, then sees the closed channel.
    drop(writer);
    tracing::info!("{player_id} left the table");
}

/// Returns false when this socket has been replaced and should close.
async fn dispatch(app: &App, player_id: &str, epoch: u64, message: ClientMsg) -> bool {
    if let ClientMsg::Ping { client_time } = &message {
        let table = app.table.lock().await;
        if table.viewer(player_id, epoch).is_none() {
            return false;
        }
        table.send_to(player_id, epoch, ServerMsg::Pong { client_time: *client_time, server_time: now_ms() });
        return true;
    }

    if let ClientMsg::Roll { formula, visibility, label } = message {
        return finish_roll(app, player_id, epoch, formula, visibility, label, None).await;
    }

    if let ClientMsg::Attack { attacker_id, target_id, plus, minus } = message {
        return finish_attack(app, player_id, epoch, attacker_id, target_id, plus, minus, false).await;
    }

    if let ClientMsg::Strike { attacker_id, target_id, plus, minus } = message {
        return finish_attack(app, player_id, epoch, attacker_id, target_id, plus, minus, true).await;
    }

    if let ClientMsg::RollAction { character_id, action_id, visibility, plus, minus } = message {
        let (system, _) = active_system(app).await;
        let prepared = {
            let table = app.table.lock().await;
            let Some(viewer) = table.viewer(player_id, epoch) else {
                return false;
            };
            table.action_formula(&viewer, &character_id, &action_id, plus, minus, system.as_ref()).map(|formula| {
                (formula, table.action_label(&action_id, system.as_ref()))
            })
        };
        let (formula, label) = match prepared {
            Ok(prepared) => prepared,
            Err(message) => {
                reply_error(app, player_id, epoch, &message).await;
                return true;
            }
        };
        return finish_roll(app, player_id, epoch, formula, visibility, label, None).await;
    }

    if let ClientMsg::SaveSystem { system } = message {
        let allowed = {
            let table = app.table.lock().await;
            match table.viewer(player_id, epoch) {
                None => return false,
                Some(viewer) => viewer.role == crate::net::protocol::Role::Gm,
            }
        };
        if !allowed {
            reply_error(app, player_id, epoch, "only the GM can do that").await;
            return true;
        }
        let saved = match save_system(&app.systems_dir, &system) {
            Ok(saved) => saved,
            Err(message) => {
                reply_error(app, player_id, epoch, &message).await;
                return true;
            }
        };
        let systems = {
            let mut catalog = app.systems.lock().await;
            catalog.upsert(saved);
            catalog.list.clone()
        };
        let table = app.table.lock().await;
        if table.viewer(player_id, epoch).is_none() {
            return false;
        }
        table.broadcast_message(ServerMsg::Systems { systems });
        return true;
    }

    let (system, _) = active_system(app).await;
    let mut table = app.table.lock().await;
    let Some(viewer) = table.viewer(player_id, epoch) else {
        return false;
    };
    match table.command(&viewer, message, system.as_ref()) {
        Ok(CommandEffect::Refresh) => {
            table.broadcast_state();
            drop(table);
            app.request_save();
        }
        Ok(CommandEffect::Quiet) => {}
        Err(message) => {
            table.send_to(player_id, epoch, ServerMsg::Error { message });
        }
    }
    true
}

fn played_die(die: crate::dice::ThrownDie) -> PlayedDie {
    PlayedDie {
        sides: die.sides,
        value: die.value,
        sign: die.sign,
        tag: die.tag,
        keyframes: die
            .keyframes
            .into_iter()
            .map(|frame| Keyframe { p: frame.p, q: frame.q })
            .collect(),
    }
}

async fn active_system(app: &App) -> (Option<crate::systems::SystemDef>, String) {
    let id = app.table.lock().await.campaign.system_id.clone();
    let system = match &id {
        Some(id) => app.systems.lock().await.get(id).cloned(),
        None => None,
    };
    (system, id.unwrap_or_default())
}

async fn finish_attack(
    app: &App,
    player_id: &str,
    epoch: u64,
    attacker_id: String,
    target_id: String,
    plus: i32,
    minus: i32,
    damage: bool,
) -> bool {
    let plan = {
        let table = app.table.lock().await;
        let Some(viewer) = table.viewer(player_id, epoch) else {
            return false;
        };
        if damage {
            table.prepare_strike(&viewer, &attacker_id, &target_id, plus, minus)
        } else {
            table.prepare_attack(&viewer, &attacker_id, &target_id, plus, minus)
        }
    };
    let plan = match plan {
        Ok(plan) => plan,
        Err(message) => {
            reply_error(app, player_id, epoch, &message).await;
            return true;
        }
    };
    let formula = plan.formula.clone();
    let label = plan.label.clone();
    finish_roll(app, player_id, epoch, formula, crate::net::protocol::RollVisibility::Public, Some(label), Some(plan)).await
}

async fn finish_roll(
    app: &App,
    player_id: &str,
    epoch: u64,
    formula: String,
    visibility: crate::net::protocol::RollVisibility,
    label: Option<String>,
    plan: Option<crate::session::state::AttackPlan>,
) -> bool {
    let (system, rules_id) = active_system(app).await;
    let rules = rules_for(&rules_id);
    let job = {
        let table = app.table.lock().await;
        let Some(viewer) = table.viewer(player_id, epoch) else {
            return false;
        };
        table.begin_roll(&viewer, &formula, visibility, label, system.as_ref(), rules.as_ref())
    };
    let job = match job {
        Ok(job) => job,
        Err(message) => {
            reply_error(app, player_id, epoch, &message).await;
            return true;
        }
    };
    let modifier = job.parsed.modifier;
    let parsed = job.parsed.clone();
    let thrown = tokio::task::spawn_blocking(move || {
        let mut rng = rand::rng();
        simulate(&parsed, &mut rng)
    })
    .await;
    let Ok(thrown) = thrown else {
        reply_error(app, player_id, epoch, "the dice tray failed").await;
        return true;
    };
    let total = modifier + thrown.iter().map(|die| die.value as i32 * die.sign as i32).sum::<i32>();
    let outcome = rules.outcome(&thrown, total);
    let mut contest = plan.as_ref().map(|plan| plan.contest.clone());
    if let Some(contest) = contest.as_mut() {
        if !plan.as_ref().is_some_and(|plan| plan.damage) {
            let critical = outcome.as_ref().is_some_and(|item| item.tone == "critical");
            let hit = crate::systems::daggerheart::attack_hits(total, contest.defense, critical);
            contest.verdict = Some(if hit { "hit".into() } else { "miss".into() });
        }
    }
    let mut event = RollEvent {
        id: job.id,
        roller_id: job.roller_id,
        roller_name: job.roller_name,
        formula: job.formula,
        label: job.label,
        visibility: job.visibility,
        modifier,
        total,
        play_at: now_ms().saturating_add(250),
        keyframe_dt: crate::session::Table::keyframe_dt(),
        dice: thrown.into_iter().map(played_die).collect(),
        outcome,
        contest,
    };
    let mut table = app.table.lock().await;
    if table.viewer(player_id, epoch).is_none() {
        return false;
    }
    if plan.as_ref().is_some_and(|plan| plan.damage) {
        let target_id = event.contest.as_ref().map(|contest| contest.target_id.clone()).unwrap_or_default();
        let Some(system) = system.as_ref() else {
            table.send_to(player_id, epoch, ServerMsg::Error { message: "this table has no system".into() });
            return true;
        };
        match table.receive_blow(&target_id, event.total, system) {
            Ok(blow) => {
                if let Some(contest) = event.contest.as_mut() {
                    contest.blow = Some(blow);
                }
            }
            Err(message) => {
                table.send_to(player_id, epoch, ServerMsg::Error { message });
                return true;
            }
        }
    }
    table.commit_roll(&event);
    table.broadcast_roll(&event);
    table.broadcast_state();
    drop(table);
    app.request_save();
    true
}

async fn reply_error(app: &App, player_id: &str, epoch: u64, message: &str) {
    let table = app.table.lock().await;
    table.send_to(player_id, epoch, ServerMsg::Error { message: message.into() });
}

fn json_error(message: &str) -> String {
    serde_json::to_string(&ServerMsg::Error { message: message.into() }).unwrap_or_else(|_| "{}".into())
}

// ------------------------------------------------------------
// HTTP: maps and portraits
// ------------------------------------------------------------

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], self.1).into_response()
    }
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    Some(value.strip_prefix("Bearer ")?.trim().to_string())
}

#[derive(Deserialize)]
struct SessionQuery {
    session: Option<String>,
}

async fn import_map(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = bearer(&headers).ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in first".into()))?;
    {
        let table = app.table.lock().await;
        let viewer = table
            .viewer_for_session(&token)
            .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()))?;
        if viewer.role != crate::net::protocol::Role::Gm {
            return Err(ApiError(StatusCode::FORBIDDEN, "only the GM can lay a map".into()));
        }
    }
    let (name, bytes) = read_file_field(&mut multipart, "map").await?;
    if bytes.starts_with(b"PK") {
        return import_package(app, &token, &name, &bytes).await;
    }
    let parsed = crate::map::parse_uvtt(&bytes).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?;

    let id = Uuid::new_v4().to_string();
    let image_file = if let Some(image) = &parsed.image {
        let (ext, _) = sniff(image).unwrap_or(("png", "image/png"));
        let file_name = format!("{id}.{ext}");
        let path = app.data_dir.join("maps").join(&file_name);
        std::fs::write(&path, image).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
        Some(file_name)
    } else {
        None
    };

    let map_name = std::path::Path::new(&name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Imported map")
        .chars()
        .filter(|ch| !ch.is_control())
        .take(48)
        .collect::<String>();

    let scene = Scene {
        id: id.clone(),
        name: if map_name.is_empty() { "Imported map".into() } else { map_name.clone() },
        width: parsed.width,
        height: parsed.height,
        image_file,
        walls: parsed.walls,
        doors: parsed.doors,
        lights: parsed.lights,
        layout: None,
        move_blockers: Vec::new(),
        sight: Vec::new(),
        presentation: None,
        schema_version: 1,
        visual_profile: DIMETRIC_PROFILE.to_string(),
    };

    let mut table = app.table.lock().await;
    if table.viewer_for_session(&token).is_none() {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()));
    }
    table.install_scene(scene);
    table.broadcast_state();
    drop(table);
    app.request_save();
    Ok(Json(json!({ "id": id, "name": map_name })))
}

async fn import_package(app: Arc<App>, token: &str, name: &str, bytes: &[u8]) -> Result<Json<serde_json::Value>, ApiError> {
    let loaded = crate::map::read_package(bytes).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?;
    let id = Uuid::new_v4().to_string();
    let map_name = std::path::Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Imported map")
        .chars()
        .filter(|ch| !ch.is_control())
        .take(48)
        .collect::<String>();
    let incoming = app.data_dir.join("maps").join(format!(".incoming-{id}"));
    if let Err(err) = write_package_files(&incoming, &loaded.files) {
        let _ = std::fs::remove_dir_all(&incoming);
        return Err(err);
    }
    let final_dir = app.data_dir.join("maps").join(&id);
    if let Err(err) = std::fs::rename(&incoming, &final_dir) {
        let _ = std::fs::remove_dir_all(&incoming);
        return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()));
    }
    let scene = Scene {
        id: id.clone(),
        name: if map_name.is_empty() { "Imported map".into() } else { map_name.clone() },
        width: loaded.parsed.width,
        height: loaded.parsed.height,
        image_file: None,
        walls: loaded.parsed.walls,
        doors: loaded.parsed.doors,
        lights: loaded.parsed.lights,
        layout: None,
        move_blockers: Vec::new(),
        sight: Vec::new(),
        presentation: Some(loaded.presentation),
        schema_version: 1,
        visual_profile: DIMETRIC_PROFILE.to_string(),
    };
    let mut warnings = vec![crate::map::presentation::FILED_VIEW.to_string()];
    if let Some(presentation) = scene.presentation.as_ref() {
        warnings.extend(presentation.warnings.iter().cloned());
    }
    let mut table = app.table.lock().await;
    if table.viewer_for_session(token).is_none() {
        drop(table);
        let _ = std::fs::remove_dir_all(&final_dir);
        return Err(ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()));
    }
    table.install_scene(scene);
    table.broadcast_state();
    drop(table);
    app.request_save();
    Ok(Json(json!({ "id": id, "name": map_name, "warnings": warnings })))
}

fn write_package_files(dir: &std::path::Path, files: &[(String, Vec<u8>)]) -> Result<(), ApiError> {
    std::fs::create_dir_all(dir).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    for (name, bytes) in files {
        let relative = crate::map::safe_relative(name).ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "bad package path".into()))?;
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
        }
        std::fs::write(&path, bytes).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    }
    Ok(())
}

async fn map_asset(
    State(app): State<Arc<App>>,
    Path((id, asset_path)): Path<(String, String)>,
    Query(query): Query<SessionQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer(&headers).or(query.session).ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in first".into()))?;
    let table = app.table.lock().await;
    if table.viewer_for_session(&token).is_none() {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()));
    }
    let allowed = table.scene.as_ref().is_some_and(|scene| scene.id == id);
    drop(table);
    if !allowed {
        return Err(ApiError(StatusCode::NOT_FOUND, "no map image".into()));
    }
    let relative = crate::map::safe_relative(&asset_path).ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "bad package path".into()))?;
    let path = app.data_dir.join("maps").join(safe_name(&id)?).join(relative);
    let bytes = tokio::fs::read(&path).await.map_err(|_| ApiError(StatusCode::NOT_FOUND, "file missing".into()))?;
    let content_type = match path.extension().and_then(|ext| ext.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("exr") => "image/x-exr",
        _ => "application/octet-stream",
    };
    Ok(([(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, "private, max-age=60")], bytes).into_response())
}

async fn map_image(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Query(query): Query<SessionQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer(&headers).or(query.session).ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in first".into()))?;
    let table = app.table.lock().await;
    if table.viewer_for_session(&token).is_none() {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()));
    }
    let file = table
        .scene
        .as_ref()
        .filter(|scene| scene.id == id)
        .and_then(|scene| scene.image_file.clone())
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "no map image".into()))?;
    drop(table);
    read_stored(&app.data_dir.join("maps").join(safe_name(&file)?)).await
}

async fn token_image(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Query(query): Query<SessionQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer(&headers).or(query.session).ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in first".into()))?;
    let table = app.table.lock().await;
    if table.viewer_for_session(&token).is_none() {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()));
    }
    let file = table
        .tokens
        .iter()
        .find(|token| token.id == id)
        .and_then(|token| token.image_file.clone())
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "no portrait".into()))?;
    drop(table);
    read_stored(&app.data_dir.join("tokens").join(safe_name(&file)?)).await
}

async fn upload_token_image(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = bearer(&headers).ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in first".into()))?;
    let (_, bytes) = read_file_field(&mut multipart, "image").await?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "portraits must be under 2 MB".into()));
    }
    let (ext, _) = sniff(&bytes).ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "use a PNG, JPEG, or WebP".into()))?;
    let file_name = format!("{id}.{ext}");
    std::fs::write(app.data_dir.join("tokens").join(&file_name), &bytes)
        .map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let mut table = app.table.lock().await;
    let viewer = table
        .viewer_for_session(&session)
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()))?;
    let allowed = viewer.role == crate::net::protocol::Role::Gm || table.owns_token(&viewer.player_id, &id);
    if !allowed {
        return Err(ApiError(StatusCode::FORBIDDEN, "that token is not yours".into()));
    }
    table.set_token_image(&id, Some(file_name)).map_err(|err| ApiError(StatusCode::NOT_FOUND, err))?;
    table.broadcast_state();
    drop(table);
    app.request_save();
    Ok(Json(json!({ "ok": true })))
}

async fn read_file_field(multipart: &mut Multipart, expected: &str) -> Result<(String, Vec<u8>), ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?
    {
        if field.name() != Some(expected) {
            continue;
        }
        let file_name = field.file_name().unwrap_or(expected).to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err.to_string()))?
            .to_vec();
        if bytes.is_empty() {
            return Err(ApiError(StatusCode::BAD_REQUEST, "that file is empty".into()));
        }
        return Ok((file_name, bytes));
    }
    Err(ApiError(StatusCode::BAD_REQUEST, format!("missing '{expected}' file")))
}

async fn read_stored(path: &std::path::Path) -> Result<Response, ApiError> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "file missing".into()))?;
    let content_type = sniff(&bytes).map(|(_, mime)| mime).unwrap_or("application/octet-stream");
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "private, max-age=60"),
        ],
        bytes,
    )
        .into_response())
}

fn safe_name(name: &str) -> Result<String, ApiError> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "bad file name".into()));
    }
    Ok(name.to_string())
}

fn sniff(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("jpg", "image/jpeg"))
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    use crate::config::Config;

    fn isolated() -> (std::path::PathBuf, Arc<App>) {
        let dir = std::env::temp_dir().join(format!(
            "longtable-import-http-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets").join("packs")).unwrap();
        let config = Config {
            bind: "127.0.0.1:0".into(),
            gm_key: "testkey".into(),
            table_passphrase: "table".into(),
            data_dir: dir.join("data").display().to_string(),
            static_dir: dir.join("none").display().to_string(),
            assets_dir: dir.join("assets").display().to_string(),
            systems_dir: dir.join("systems").display().to_string(),
        };
        (dir, App::boot(config))
    }

    async fn session(app: &App, as_gm: bool) -> String {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut table = app.table.lock().await;
        let seat = table.take_seat("Ada", "table", as_gm, Some("testkey"), None).unwrap();
        table.attach(&seat.player_id, tx).unwrap().1
    }

    #[tokio::test]
    async fn an_unsigned_pack_import_is_refused() {
        let (dir, app) = isolated();
        let response = router(app)
            .oneshot(HttpRequest::builder().method("POST").uri("/api/packs/import").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn an_unsigned_pack_check_is_refused() {
        let (dir, app) = isolated();
        let response = router(app)
            .oneshot(HttpRequest::builder().method("POST").uri("/api/packs/check").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn a_player_cannot_import_a_pack() {
        let (dir, app) = isolated();
        let token = session(&app, false).await;
        let response = router(app)
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/packs/import")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Two live sockets: the GM imports and publishes, the player receives the
    /// scene and the visible cast, and a hidden figure never arrives.
    #[tokio::test]
    async fn a_player_joins_the_published_scene_on_a_second_socket() {
        let (dir, app) = isolated();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = app.clone();
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, router(served).into_make_service()).await;
        });

        let mut gm = Socket::connect(addr).await;
        gm.send_text(r#"{"type":"join","name":"Mina","passphrase":"table","asGm":true,"gmKey":"testkey","resumeId":null}"#)
            .await;
        let welcome = gm.until(|value| value["type"] == "welcome").await;
        let token = welcome["sessionToken"].as_str().unwrap().to_string();
        assert_eq!(welcome["role"], "gm");

        let mut player = Socket::connect(addr).await;
        player
            .send_text(r#"{"type":"join","name":"Ada","passphrase":"table","asGm":false,"gmKey":null,"resumeId":null}"#)
            .await;
        let player_welcome = player.until(|value| value["type"] == "welcome").await;
        let player_id = player_welcome["playerId"].as_str().unwrap().to_string();

        let imported = http(addr, "POST", "/api/packs/import", &token, &multipart(&gate_zip())).await;
        assert_eq!(imported.0, 200, "{}", String::from_utf8_lossy(&imported.1));
        let gm_library = gm.until(|value| value["type"] == "snapshot" && value["catalogRevision"] == "2").await;
        let player_library = player.until(|value| value["type"] == "snapshot" && value["catalogRevision"] == "2").await;
        assert_eq!(gm_library["catalogRevision"], player_library["catalogRevision"]);

        let created = http(
            addr,
            "POST",
            "/api/drafts",
            &token,
            &json_body(br#"{"name":"Gate","width":12,"height":12}"#),
        )
        .await;
        assert_eq!(created.0, 200, "{}", String::from_utf8_lossy(&created.1));
        let created_body = serde_json::from_slice::<serde_json::Value>(&created.1).unwrap();
        let draft_id = created_body["id"].as_str().unwrap().to_string();
        let updated_at = created_body["updatedAt"].as_u64().unwrap();
        let layout = crate::map::MapLayout::blank(12, 12, "gate/floor");
        let mut layout = layout;
        layout.props.push(crate::map::layout::PlacedProp {
            id: "barrel".into(),
            asset: "gate/barrel".into(),
            x: 3.0,
            y: 4.0,
            mirror: false,
        });
        let draft = serde_json::json!({
            "id": draft_id,
            "name": "Gate",
            "updatedAt": updated_at,
            "sceneId": null,
            "layout": layout,
        });
        let written = http(
            addr,
            "PUT",
            &format!("/api/drafts/{draft_id}"),
            &token,
            &json_body(&serde_json::to_vec(&draft).unwrap()),
        )
        .await;
        assert_eq!(written.0, 200, "{}", String::from_utf8_lossy(&written.1));
        let stale = http(
            addr,
            "PUT",
            &format!("/api/drafts/{draft_id}"),
            &token,
            &json_body(&serde_json::to_vec(&draft).unwrap()),
        )
        .await;
        assert_eq!(stale.0, 400, "{}", String::from_utf8_lossy(&stale.1));
        assert!(String::from_utf8_lossy(&stale.1).contains("saved somewhere else"));
        let published = http(addr, "POST", &format!("/api/drafts/{draft_id}/publish"), &token, &json_body(b"")).await;
        assert_eq!(published.0, 204, "{}", String::from_utf8_lossy(&published.1));
        let again = http(addr, "POST", &format!("/api/drafts/{draft_id}/publish"), &token, &json_body(b"")).await;
        assert_eq!(again.0, 204, "{}", String::from_utf8_lossy(&again.1));

        let gm_scene = gm.until(|value| value["type"] == "snapshot" && value["scene"].is_object()).await;
        let player_scene = player.until(|value| value["type"] == "snapshot" && value["scene"].is_object()).await;
        let scene_id = gm_scene["scene"]["id"].as_str().unwrap().to_string();
        assert_eq!(player_scene["scene"]["id"], scene_id);
        assert_eq!(player_scene["scene"]["layout"]["props"][0]["asset"], "gate/barrel");

        gm.send_text(&format!(
            r##"{{"type":"createToken","name":"Ada","x":2,"y":2,"ownerId":"{player_id}","hidden":false,"visionRadius":6,"grantsVision":true,"color":"#d4a656","characterId":null,"castAsset":"gate/bandit","facing":"south"}}"##
        ))
        .await;
        gm.send_text(&format!(
            r##"{{"type":"createToken","name":"Bram","x":6,"y":6,"ownerId":"{player_id}","hidden":false,"visionRadius":6,"grantsVision":true,"color":"#6a8f6a","characterId":null,"castAsset":"gate/scout","facing":"south"}}"##
        ))
        .await;
        gm.send_text(
            r##"{"type":"createToken","name":"Lurker","x":8,"y":8,"ownerId":null,"hidden":true,"visionRadius":0,"grantsVision":false,"color":"#444444","characterId":null,"castAsset":"gate/bandit","facing":"south"}"##,
        )
        .await;
        let seen = player.until(|value| value["type"] == "snapshot" && value["tokens"].as_array().is_some_and(|tokens| tokens.iter().any(|token| token["name"] == "Bram"))).await;
        let names: Vec<&str> = seen["tokens"].as_array().unwrap().iter().filter_map(|token| token["name"].as_str()).collect();
        assert!(names.contains(&"Ada") && names.contains(&"Bram"), "{names:?}");
        assert!(!names.contains(&"Lurker"), "{names:?}");
        let ada = seen["tokens"].as_array().unwrap().iter().find(|token| token["name"] == "Ada").unwrap();
        let bram = seen["tokens"].as_array().unwrap().iter().find(|token| token["name"] == "Bram").unwrap();
        assert_eq!(ada["castAsset"], "gate/bandit");
        assert_eq!(bram["castAsset"], "gate/scout");
        let ada_id = ada["id"].as_str().unwrap();

        gm.send_text(&format!(r#"{{"type":"moveToken","tokenId":"{ada_id}","x":5,"y":2}}"#)).await;
        let moved = player
            .until(|value| {
                value["type"] == "snapshot"
                    && value["tokens"].as_array().is_some_and(|tokens| {
                        tokens.iter().any(|token| token["id"] == ada_id && token["facing"] == "east" && token["x"].as_f64().unwrap_or(0.0) > 4.0)
                    })
            })
            .await;
        assert!(moved["tokens"].as_array().unwrap().iter().all(|token| token["name"] != "Lurker"));
        assert!(moved["tokens"].as_array().unwrap().iter().any(|token| token["name"] == "Bram" && token["facing"] == "south"));

        gm.send_text(&format!(r##"{{"type":"viewAs","playerId":"{player_id}"}}"##)).await;
        let looking = gm
            .until(|value| value["type"] == "snapshot" && value["viewAs"] == player_id)
            .await;
        assert!(looking["tokens"].as_array().unwrap().iter().all(|token| token["name"] != "Lurker"));
        gm.send_text(r##"{"type":"viewAs","playerId":null}"##).await;
        let restored_view = gm.until(|value| value["type"] == "snapshot" && value["viewAs"].is_null() && value["tokens"].as_array().is_some_and(|tokens| tokens.iter().any(|token| token["name"] == "Lurker"))).await;
        assert!(restored_view["tokens"].as_array().unwrap().iter().any(|token| token["name"] == "Lurker"));

        {
            let table = app.table.lock().await;
            crate::storage::write(&table, &app.data_dir).unwrap();
        }
        let mut restored = crate::session::state::Table::new();
        crate::storage::load_into(&mut restored, &app.data_dir);
        assert_eq!(restored.scene.as_ref().map(|scene| scene.id.as_str()), Some(scene_id.as_str()));
        assert!(restored.tokens.iter().any(|token| token.name == "Ada" && token.facing == "east" && token.cast_asset.as_deref() == Some("gate/bandit")));
        assert!(restored.tokens.iter().any(|token| token.hidden && token.name == "Lurker"));

        server.abort();
        let _ = std::fs::remove_dir_all(dir);
    }

    struct Socket {
        stream: tokio::net::TcpStream,
    }

    impl Socket {
        async fn connect(addr: std::net::SocketAddr) -> Self {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let request = format!(
                "GET /ws HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
            );
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut header = Vec::new();
            let mut byte = [0_u8; 1];
            loop {
                stream.read_exact(&mut byte).await.unwrap();
                header.push(byte[0]);
                if header.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&header);
            assert!(text.contains("101"), "{text}");
            Self { stream }
        }

        async fn send_text(&mut self, text: &str) {
            use tokio::io::AsyncWriteExt;
            let mut frame = vec![0x81_u8];
            let length = text.len();
            if length < 126 {
                frame.push(0x80 | length as u8);
            } else if length <= u16::MAX as usize {
                frame.push(0x80 | 126);
                frame.extend_from_slice(&(length as u16).to_be_bytes());
            } else {
                frame.push(0x80 | 127);
                frame.extend_from_slice(&(length as u64).to_be_bytes());
            }
            let mask = [9_u8, 8, 7, 6];
            frame.extend_from_slice(&mask);
            for (index, byte) in text.as_bytes().iter().enumerate() {
                frame.push(byte ^ mask[index % 4]);
            }
            self.stream.write_all(&frame).await.unwrap();
        }

        async fn next_text(&mut self) -> String {
            use tokio::io::AsyncReadExt;
            loop {
                let mut header = [0_u8; 2];
                self.stream.read_exact(&mut header).await.unwrap();
                let opcode = header[0] & 0x0f;
                let mut length = (header[1] & 0x7f) as usize;
                if length == 126 {
                    let mut extra = [0_u8; 2];
                    self.stream.read_exact(&mut extra).await.unwrap();
                    length = u16::from_be_bytes(extra) as usize;
                } else if length == 127 {
                    let mut extra = [0_u8; 8];
                    self.stream.read_exact(&mut extra).await.unwrap();
                    length = u64::from_be_bytes(extra) as usize;
                }
                if header[1] & 0x80 != 0 {
                    let mut mask = [0_u8; 4];
                    self.stream.read_exact(&mut mask).await.unwrap();
                }
                let mut payload = vec![0_u8; length];
                self.stream.read_exact(&mut payload).await.unwrap();
                if opcode == 0x1 {
                    return String::from_utf8(payload).unwrap();
                }
            }
        }

        async fn until(&mut self, ready: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
            for _ in 0..40 {
                let text = tokio::time::timeout(Duration::from_secs(5), self.next_text()).await.expect("the socket went quiet");
                let value: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
                if ready(&value) {
                    return value;
                }
            }
            panic!("the expected message did not arrive");
        }
    }

    fn json_body(bytes: &[u8]) -> (String, Vec<u8>) {
        ("application/json".into(), bytes.to_vec())
    }

    fn multipart(bytes: &[u8]) -> (String, Vec<u8>) {
        let boundary = "longtable-pack-boundary";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"pack\"; filename=\"pack.zip\"\r\nContent-Type: application/zip\r\n\r\n").as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={boundary}"), body)
    }

    async fn http(addr: std::net::SocketAddr, method: &str, path: &str, token: &str, body: &(String, Vec<u8>)) -> (u16, Vec<u8>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let header = format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.0,
            body.1.len()
        );
        stream.write_all(header.as_bytes()).await.unwrap();
        stream.write_all(&body.1).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let text = String::from_utf8_lossy(&response);
        let status = text.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        let split = response.windows(4).position(|window| window == b"\r\n\r\n").unwrap_or(response.len());
        let payload = if split + 4 <= response.len() { response[split + 4..].to_vec() } else { Vec::new() };
        (status, payload)
    }

    fn gate_zip() -> Vec<u8> {
        use std::io::Write;
        let manifest = r#"{
            "schemaVersion": 2,
            "id": "gate",
            "name": "Gate",
            "visualProfile": "dimetric-30-v1",
            "pixelsPerCell": 128,
            "filtering": "linear",
            "tiles": [{
                "id": "floor",
                "name": "Floor",
                "kind": "ground",
                "sourceSizePx": [128, 128],
                "coverageCells": [1, 1],
                "files": ["tiles/floor.png"]
            }],
            "objects": [{
                "id": "barrel",
                "name": "Barrel",
                "kind": "static",
                "placement": "cell",
                "footprintCells": [1, 1],
                "blocksMovement": true,
                "blocksVision": false,
                "pixelsPerWorldUnit": 128,
                "renderLayers": [{
                    "kind": "upright",
                    "file": "objects/barrel.png",
                    "sizePx": [32, 48],
                    "pivotPx": [16, 46]
                }]
            }],
            "cast": [{
                "id": "bandit",
                "name": "Bandit",
                "pixelsPerWorldUnit": 128,
                "views": {
                    "north": {"file": "cast/bandit/north.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "east": {"file": "cast/bandit/east.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "south": {"file": "cast/bandit/south.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "west": {"file": "cast/bandit/west.png", "sizePx": [32, 64], "pivotPx": [16, 60]}
                }
            }, {
                "id": "scout",
                "name": "Scout",
                "pixelsPerWorldUnit": 128,
                "views": {
                    "north": {"file": "cast/scout/north.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "east": {"file": "cast/scout/east.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "south": {"file": "cast/scout/south.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "west": {"file": "cast/scout/west.png", "sizePx": [32, 64], "pivotPx": [16, 60]}
                }
            }]
        }"#;
        let files = [
            ("tiles/floor.png", rgba_png(128, 128, 255)),
            ("objects/barrel.png", rgba_png(32, 48, 0)),
            ("cast/bandit/north.png", rgba_png(32, 64, 0)),
            ("cast/bandit/east.png", rgba_png(32, 64, 0)),
            ("cast/bandit/south.png", rgba_png(32, 64, 0)),
            ("cast/bandit/west.png", rgba_png(32, 64, 0)),
            ("cast/scout/north.png", rgba_png(32, 64, 0)),
            ("cast/scout/east.png", rgba_png(32, 64, 0)),
            ("cast/scout/south.png", rgba_png(32, 64, 0)),
            ("cast/scout/west.png", rgba_png(32, 64, 0)),
        ];
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer.start_file("pack.json", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(manifest.as_bytes()).unwrap();
        for (name, bytes) in files {
            writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn rgba_png(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        let mut pixels = vec![180_u8; (width * height * 4) as usize];
        for pixel in pixels.chunks_mut(4) {
            pixel[3] = 255;
        }
        pixels[3] = alpha;
        let mut bytes = Vec::new();
        use image::ImageEncoder;
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }
}
