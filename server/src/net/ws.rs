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
use crate::map::drafts::{safe_id, Draft, DraftStore};
use crate::map::layout::MapLayout;
use crate::map::{Scene, DIMETRIC_PROFILE};
use crate::net::protocol::{dice_meshes, ClientMsg, Keyframe, PlayedDie, RollEvent, ServerMsg};
use crate::session::state::{now_ms, CommandEffect};

pub fn router(app: Arc<App>) -> Router {
    let import = Router::new()
        .route("/api/maps/import", post(import_map))
        .route("/api/packs/import", post(import_pack))
        .layer(DefaultBodyLimit::max(512 * 1024 * 1024))
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
        .merge(import);

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
    let pack = crate::asset_import::import_bytes(&bytes, &app.assets_dir).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    let mut catalog = crate::assets::load_catalog(&app.assets_dir).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    if !catalog.packs.iter().any(|item| item.id == pack.id) {
        catalog.packs.push(pack);
        catalog.revision = crate::asset_import::revision_of(&catalog.packs);
    }
    let revision = catalog.revision;
    *app.catalog.lock().await = catalog.clone();
    {
        let mut table = app.table.lock().await;
        table.catalog_revision = revision;
        table.broadcast_state();
    }
    Ok(Json(catalog))
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
) -> Result<StatusCode, ApiError> {
    require_gm(&app, &headers).await?;
    draft.id = safe_id(&id).map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    draft.updated_at = now_ms();
    DraftStore::new(&app.data_dir)
        .put(&draft)
        .map_err(|err| ApiError(StatusCode::BAD_REQUEST, err))?;
    Ok(StatusCode::NO_CONTENT)
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
    let compiled = compile(&draft.layout, &catalog);
    let mut table = app.table.lock().await;
    let viewer = table
        .viewer_for_session(&token)
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()))?;
    table
        .publish_built(&viewer, draft.name, draft.layout, compiled)
        .map_err(|err| ApiError(StatusCode::FORBIDDEN, err))?;
    table.broadcast_state();
    drop(table);
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
        presentation: Some(loaded.presentation),
        schema_version: 1,
        visual_profile: String::new(),
    };
    let warnings = scene.presentation.as_ref().map(|item| item.warnings.clone()).unwrap_or_default();
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
}
