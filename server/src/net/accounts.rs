// ------------------------------------------------------------
// Module: accounts
// Purpose: Sign-in, tables, friends, and the settings that
//          follow an account. The websocket still carries play.
// ------------------------------------------------------------

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::App;
use crate::house::AccountSettings;
use crate::net::protocol::Role;
use crate::storage;

pub async fn register(State(app): State<Arc<App>>, Json(body): Json<Credentials>) -> Result<Json<Value>, ApiError> {
    let mut house = app.house.lock().await;
    let (token, account) = house.register(&body.username, &body.password).map_err(bad)?;
    let lobby = house.lobby(&token).map_err(bad)?;
    Ok(Json(json!({ "token": token, "lobby": lobby, "account": account })))
}

pub async fn login(State(app): State<Arc<App>>, Json(body): Json<Credentials>) -> Result<Json<Value>, ApiError> {
    let mut house = app.house.lock().await;
    let (token, account) = house.login(&body.username, &body.password).map_err(bad)?;
    let lobby = house.lobby(&token).map_err(bad)?;
    Ok(Json(json!({ "token": token, "lobby": lobby, "account": account })))
}

pub async fn me(State(app): State<Arc<App>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let house = app.house.lock().await;
    let lobby = house.lobby(&token).map_err(bad)?;
    Ok(Json(json!(lobby)))
}

pub async fn update_me(State(app): State<Arc<App>>, headers: HeaderMap, Json(body): Json<ProfileBody>) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let mut house = app.house.lock().await;
    let account = house
        .update_account(&token, body.username, body.color, body.settings)
        .map_err(bad)?;
    let lobby = house.lobby(&token).map_err(bad)?;
    Ok(Json(json!({ "account": account, "lobby": lobby })))
}

pub async fn create_table(State(app): State<Arc<App>>, headers: HeaderMap, Json(body): Json<NameBody>) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let (id, name) = {
        let mut house = app.house.lock().await;
        let id = house.create_table(&token, &body.name).map_err(bad)?;
        let name = house.table_name(&id).unwrap_or_else(|| "Table".into());
        (id, name)
    };
    let path = app.data_dir.join("tables").join(format!("{id}.json"));
    storage::write_fresh(&path, &name).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let house = app.house.lock().await;
    let lobby = house.lobby(&token).map_err(bad)?;
    Ok(Json(json!(lobby)))
}

pub async fn open_table(State(app): State<Arc<App>>, headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let account = {
        let house = app.house.lock().await;
        let account = house.account(&token).ok_or_else(|| bad("sign in again".into()))?;
        if !house.owns(&account.id, &id) {
            return Err(bad("only the GM can open that table".into()));
        }
        account
    };
    let already = app.house.lock().await.open_id();
    if already.as_deref() == Some(id.as_str()) {
        let house = app.house.lock().await;
        return Ok(Json(json!(house.lobby(&token).map_err(bad)?)));
    }
    let previous = already;
    let path = app.data_dir.join("tables").join(format!("{id}.json"));
    if path.is_file() {
        let mut probe = crate::session::state::Table::new();
        if let Err(err) = storage::load_file(&mut probe, &path) {
            return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()));
        }
    }
    {
        let table = app.table.lock().await;
        if table.save_hold.is_some() {
            return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "the open table is on hold, so it was not switched".into()));
        }
        if let Some(previous) = &previous {
            let path = app.data_dir.join("tables").join(format!("{previous}.json"));
            storage::write_to(&table, &path).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
        }
    }
    {
        let mut table = app.table.lock().await;
        table.vacate();
        let path = app.data_dir.join("tables").join(format!("{id}.json"));
        if !path.is_file() {
            storage::write_fresh(&path, &account.username).map_err(|err| ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
        }
        match storage::load_file(&mut table, &path) {
            Ok(true) => {}
            Ok(false) => return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "that table could not be opened".into())),
            Err(err) => return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
        }
        let passphrase = table.passphrase.clone();
        let gm_key = table.gm_key.clone();
        table.passphrase = passphrase;
        table.gm_key = gm_key;
    }
    {
        let mut house = app.house.lock().await;
        house.set_open_id(Some(id));
    }
    app.request_save();
    let house = app.house.lock().await;
    Ok(Json(json!(house.lobby(&token).map_err(bad)?)))
}

pub async fn request_seat(State(app): State<Arc<App>>, headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    {
        let house = app.house.lock().await;
        if house.open_id().as_deref() != Some(id.as_str()) {
            return Err(bad("that table is not open".into()));
        }
    }
    let gm_here = {
        let table = app.table.lock().await;
        table.players.iter().any(|player| player.role == Role::Gm && player.connected)
    };
    if !gm_here {
        return Err(bad("the GM is not at the table".into()));
    }
    let mut house = app.house.lock().await;
    house.request_seat(&token, &id).map_err(bad)?;
    Ok(Json(json!(house.lobby(&token).map_err(bad)?)))
}

pub async fn approve(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ApproveBody>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let mut house = app.house.lock().await;
    house.approve(&token, &id, &body.account_id, body.always).map_err(bad)?;
    Ok(Json(json!(house.lobby(&token).map_err(bad)?)))
}

pub async fn friend_request(State(app): State<Arc<App>>, headers: HeaderMap, Json(body): Json<NameBody>) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let mut house = app.house.lock().await;
    house.friend_request(&token, &body.name).map_err(bad)?;
    Ok(Json(json!(house.lobby(&token).map_err(bad)?)))
}

pub async fn accept_friend(State(app): State<Arc<App>>, headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let token = bearer(&headers)?;
    let mut house = app.house.lock().await;
    house.accept_friend(&token, &id).map_err(bad)?;
    Ok(Json(json!(house.lobby(&token).map_err(bad)?)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameBody {
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileBody {
    pub username: Option<String>,
    pub color: Option<String>,
    pub settings: Option<AccountSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveBody {
    pub account_id: String,
    #[serde(default)]
    pub always: bool,
}

pub struct ApiError(StatusCode, String);

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = self.0;
        let body = Json(json!({ "message": self.1 }));
        (status, body).into_response()
    }
}

fn bad(message: String) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message)
}

pub fn bearer(headers: &HeaderMap) -> Result<String, ApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "sign in again".into()))?;
    Ok(value.to_string())
}
