// ------------------------------------------------------------
// Module: state
// Purpose: The one table this process hosts. Movement, doors,
//          fog, and tokens are changed here, then a filtered
//          snapshot is pushed to every open connection.
// ------------------------------------------------------------

use std::collections::HashMap;

use tokio::sync::mpsc;
use uuid::Uuid;

use std::collections::{BTreeMap, HashSet};

use crate::dice::{parse_formula, RollFormula, KEYFRAME_DT};
use crate::map::compile::Compiled;
use crate::map::{MapLayout, Scene, Segment, Vec2};
use crate::net::protocol::{
    ClientMsg, ConcealedRoll, DieResult, FogBrushMode, FogCell, RollEvent, RollVisibility, ServerMsg,
};
use crate::systems::{
    apply_modifiers, field_ceiling, fill_formula, threshold_marks, Character, CharacterKind, FieldKind, SystemDef, Campaign,
};
use crate::vision::{cells_for, path_blocked, visibility_polygon, Explored, CELLS_PER_SQUARE};

use super::permissions::{can_see_roll, conceal_notice, snapshot};

const COLORS: [&str; 6] = ["#d4a656", "#7eb8a2", "#d27b66", "#8aa6d4", "#c98bb8", "#d8c7a1"];
pub const DEFAULT_WIDTH: f32 = 40.0;
pub const DEFAULT_HEIGHT: f32 = 30.0;
const ROLL_LOG_LIMIT: usize = 40;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub role: crate::net::protocol::Role,
    pub color: String,
    pub connected: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub image_file: Option<String>,
    pub color: String,
    pub owner_id: Option<String>,
    pub hidden: bool,
    pub vision_radius: f32,
    pub grants_vision: bool,
    #[serde(default)]
    pub character_id: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StoredRoll {
    pub id: String,
    pub roller_id: String,
    pub roller_name: String,
    pub formula: String,
    pub label: Option<String>,
    pub visibility: RollVisibility,
    pub modifier: i32,
    pub total: i32,
    pub dice: Vec<DieResult>,
    pub created_at: u64,
    #[serde(default)]
    pub outcome: Option<crate::systems::RollOutcome>,
}

/// Who is asking, and whose eyes the map should use.
pub struct Viewer {
    pub player_id: String,
    pub role: crate::net::protocol::Role,
    pub view_as: Option<String>,
    pub session_token: String,
}

pub struct Seat {
    pub player_id: String,
    pub role: crate::net::protocol::Role,
}

struct ClientConn {
    player_id: String,
    epoch: u64,
    session_token: String,
    view_as: Option<String>,
    out: mpsc::UnboundedSender<ServerMsg>,
}

/// A roll the server has accepted and is about to simulate.
pub struct RollJob {
    pub id: String,
    pub roller_id: String,
    pub roller_name: String,
    pub formula: String,
    pub label: Option<String>,
    pub visibility: RollVisibility,
    pub parsed: RollFormula,
}

/// A scene that is not the one currently on the table.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SceneShelf {
    pub id: String,
    pub name: String,
    pub scene: Option<Scene>,
    pub tokens: Vec<Token>,
    pub explored: Explored,
}

/// A campaign that is not the one currently being played.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CampaignShelf {
    pub campaign: Campaign,
    pub scene: Option<Scene>,
    pub tokens: Vec<Token>,
    pub explored: Explored,
    pub characters: Vec<Character>,
    pub scenes: Vec<SceneShelf>,
}

pub struct Table {
    pub passphrase: String,
    pub gm_key: String,
    pub players: Vec<Player>,
    pub scene: Option<Scene>,
    pub tokens: Vec<Token>,
    pub explored: Explored,
    pub rolls: Vec<StoredRoll>,
    pub campaign: Campaign,
    /// Scenes in the live campaign, other than the one on the table.
    pub scenes: Vec<SceneShelf>,
    pub shelved: Vec<CampaignShelf>,
    pub characters: Vec<Character>,
    clients: Vec<ClientConn>,
    sessions: HashMap<String, String>,
    epoch: u64,
    next_color: usize,
}

impl Table {
    pub fn new() -> Self {
        let (width, height) = cells_for(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        Self {
            passphrase: "table".into(),
            gm_key: String::new(),
            players: Vec::new(),
            scene: None,
            tokens: Vec::new(),
            explored: Explored::new(width, height),
            rolls: Vec::new(),
            campaign: Campaign {
                id: Uuid::new_v4().to_string(),
                name: "Campaign".into(),
                system_id: None,
            },
            scenes: Vec::new(),
            shelved: Vec::new(),
            characters: Vec::new(),
            clients: Vec::new(),
            sessions: HashMap::new(),
            epoch: 0,
            next_color: 0,
        }
    }

    pub fn bounds(&self) -> (f32, f32) {
        self.scene
            .as_ref()
            .map(|scene| (scene.width, scene.height))
            .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT))
    }

    pub fn sync_fog_size(&mut self) {
        let (width, height) = self.bounds();
        let (cells_w, cells_h) = cells_for(width, height);
        if self.explored.width != cells_w || self.explored.height != cells_h {
            self.explored.resize(cells_w, cells_h);
        }
    }

    // --------------------------------------------------------
    // Seats
    // --------------------------------------------------------

    pub fn take_seat(
        &mut self,
        name: &str,
        passphrase: &str,
        as_gm: bool,
        gm_key: Option<&str>,
        resume_id: Option<&str>,
    ) -> Result<Seat, String> {
        if passphrase.trim() != self.passphrase {
            return Err("that passphrase does not open this table".into());
        }
        if as_gm && !same_key(gm_key.unwrap_or(""), &self.gm_key) {
            return Err("that GM key does not match this table".into());
        }
        let name = clean_name(name);
        let role = if as_gm {
            crate::net::protocol::Role::Gm
        } else {
            crate::net::protocol::Role::Player
        };

        if let Some(id) = resume_id.filter(|id| !id.is_empty()) {
            let same_role = self.players.iter().any(|player| player.id == id && player.role == role);
            if same_role {
                if let Some(player) = self.players.iter_mut().find(|player| player.id == id) {
                    player.name = name;
                }
                return Ok(Seat { player_id: id.to_string(), role });
            }
        }

        let id = Uuid::new_v4().to_string();
        let color = COLORS[self.next_color % COLORS.len()].to_string();
        self.next_color += 1;
        self.players.push(Player { id: id.clone(), name, role, color, connected: false });
        Ok(Seat { player_id: id, role })
    }

    pub fn attach(
        &mut self,
        player_id: &str,
        out: mpsc::UnboundedSender<ServerMsg>,
    ) -> Result<(u64, String), String> {
        if !self.players.iter().any(|player| player.id == player_id) {
            return Err("that seat is gone".into());
        }
        self.epoch += 1;
        let epoch = self.epoch;
        self.clients.retain(|client| client.player_id != player_id);
        self.sessions.retain(|_, id| id != player_id);
        let session_token = Uuid::new_v4().to_string();
        self.sessions.insert(session_token.clone(), player_id.to_string());
        self.clients.push(ClientConn {
            player_id: player_id.to_string(),
            epoch,
            session_token: session_token.clone(),
            view_as: None,
            out,
        });
        if let Some(player) = self.players.iter_mut().find(|player| player.id == player_id) {
            player.connected = true;
        }
        Ok((epoch, session_token))
    }

    pub fn detach(&mut self, player_id: &str, epoch: u64) {
        let token = self
            .clients
            .iter()
            .find(|client| client.player_id == player_id && client.epoch == epoch)
            .map(|client| client.session_token.clone());
        self.clients.retain(|client| !(client.player_id == player_id && client.epoch == epoch));
        if let Some(token) = token {
            self.sessions.remove(&token);
        }
        let still = self.clients.iter().any(|client| client.player_id == player_id);
        if let Some(player) = self.players.iter_mut().find(|player| player.id == player_id) {
            player.connected = still;
        }
    }

    pub fn viewer(&self, player_id: &str, epoch: u64) -> Option<Viewer> {
        let client = self.clients.iter().find(|client| client.player_id == player_id && client.epoch == epoch)?;
        self.viewer_from_client(client)
    }

    pub fn viewer_for_session(&self, token: &str) -> Option<Viewer> {
        let player_id = self.sessions.get(token)?.clone();
        let client = self.clients.iter().find(|client| client.session_token == token && client.player_id == player_id)?;
        self.viewer_from_client(client)
    }

    fn viewer_from_client(&self, client: &ClientConn) -> Option<Viewer> {
        let player = self.players.iter().find(|player| player.id == client.player_id)?;
        Some(Viewer {
            player_id: player.id.clone(),
            role: player.role,
            view_as: client.view_as.clone(),
            session_token: client.session_token.clone(),
        })
    }

    pub fn broadcast_state(&self) {
        for client in &self.clients {
            let Some(viewer) = self.viewer_from_client(client) else {
                continue;
            };
            let _ = client.out.send(ServerMsg::Snapshot(snapshot(self, &viewer)));
        }
    }

    pub fn broadcast_roll(&self, roll: &RollEvent) {
        for client in &self.clients {
            let Some(viewer) = self.viewer_from_client(client) else {
                continue;
            };
            let message = if can_see_roll(&viewer, roll.visibility, &roll.roller_id) {
                ServerMsg::Roll(roll.clone())
            } else {
                ServerMsg::Concealed(ConcealedRoll {
                    id: roll.id.clone(),
                    notice: conceal_notice(roll.visibility, &roll.roller_name),
                    play_at: roll.play_at,
                })
            };
            let _ = client.out.send(message);
        }
    }

    pub fn send_to(&self, player_id: &str, epoch: u64, message: ServerMsg) {
        if let Some(client) = self.clients.iter().find(|client| client.player_id == player_id && client.epoch == epoch) {
            let _ = client.out.send(message);
        }
    }

    // --------------------------------------------------------
    // Commands
    // --------------------------------------------------------

    pub fn command(&mut self, viewer: &Viewer, message: ClientMsg, system: Option<&SystemDef>) -> Result<CommandEffect, String> {
        match message {
            ClientMsg::Join { .. } | ClientMsg::Ping { .. } => Ok(CommandEffect::Quiet),
            ClientMsg::MoveToken { token_id, x, y } => {
                if self.move_token(viewer, &token_id, x, y)? {
                    Ok(CommandEffect::Refresh)
                } else {
                    Ok(CommandEffect::Quiet)
                }
            }
            ClientMsg::Roll { .. } => Err("rolls are started by the socket, not the table".into()),
            ClientMsg::SetDoor { door_id, open } => {
                self.set_door(viewer, &door_id, open)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::SetLighting { lighting } => {
                self.set_lighting(viewer, &lighting)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::FogBrush { mode, cells } => {
                self.fog_brush(viewer, mode, &cells)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::CreateToken { name, x, y, owner_id, hidden, vision_radius, grants_vision, color, character_id } => {
                self.create_token(viewer, name, x, y, owner_id, hidden, vision_radius, grants_vision, color, character_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::UpdateToken {
                token_id,
                name,
                owner_id,
                clear_owner,
                hidden,
                vision_radius,
                grants_vision,
                color,
                character_id,
                clear_character,
            } => {
                self.update_token(
                    viewer,
                    &token_id,
                    name,
                    owner_id,
                    clear_owner,
                    hidden,
                    vision_radius,
                    grants_vision,
                    color,
                    character_id,
                    clear_character,
                )?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::DeleteToken { token_id } => {
                self.delete_token(viewer, &token_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::RemovePlayer { player_id } => {
                self.remove_player(viewer, &player_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::ViewAs { player_id } => {
                self.set_view_as(viewer, player_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::BlankMap { width, height } => {
                self.blank_map(viewer, width, height)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::SetCampaignSystem { system_id } => {
                self.set_campaign_system(viewer, system_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::SaveSystem { .. } | ClientMsg::RollAction { .. } => {
                Err("that request is handled by the socket".into())
            }
            ClientMsg::CreateCharacter { name, kind } => {
                self.create_character(viewer, name, kind, system)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::UpdateCharacter {
                character_id,
                name,
                kind,
                owner_id,
                clear_owner,
                numbers,
                texts,
            } => {
                self.update_character(viewer, &character_id, name, kind, owner_id, clear_owner, numbers, texts)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::DeleteCharacter { character_id } => {
                self.delete_character(viewer, &character_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::AdjustResource { character_id, field, delta } => {
                self.adjust_resource(viewer, &character_id, &field, delta, system)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::ApplyDamage { character_id, field_id, amount } => {
                self.apply_damage(viewer, &character_id, &field_id, amount, system)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::CreateCampaign { name, system_id } => {
                self.create_campaign(viewer, name, system_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::SwitchCampaign { campaign_id } => {
                self.switch_campaign(viewer, &campaign_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::RenameCampaign { campaign_id, name } => {
                self.rename_campaign(viewer, &campaign_id, name)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::DeleteCampaign { campaign_id } => {
                self.delete_campaign(viewer, &campaign_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::CreateScene { name, width, height } => {
                self.create_scene(viewer, name, width, height)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::SwitchScene { scene_id } => {
                self.switch_scene(viewer, &scene_id)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::RenameScene { scene_id, name } => {
                self.rename_scene(viewer, &scene_id, name)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::DeleteScene { scene_id } => {
                self.delete_scene(viewer, &scene_id)?;
                Ok(CommandEffect::Refresh)
            }
        }
    }

    pub fn begin_roll(
        &self,
        viewer: &Viewer,
        formula: &str,
        visibility: RollVisibility,
        label: Option<String>,
        system: Option<&SystemDef>,
        rules: &dyn crate::systems::Rules,
    ) -> Result<RollJob, String> {
        if visibility == RollVisibility::GmSecret && viewer.role != crate::net::protocol::Role::Gm {
            return Err("only the GM rolls in secret".into());
        }
        let parsed = parse_formula(formula).map_err(|err| err.to_string())?;
        for group in &parsed.groups {
            if let Some(letter) = &group.tag {
                let known = system.is_some_and(|system| system.die_tags.iter().any(|tag| &tag.letter == letter));
                if !known {
                    return Err(format!("'{letter}' is not a die on this system"));
                }
            }
        }
        rules.check_roll(&parsed)?;
        let roller_name = self
            .players
            .iter()
            .find(|player| player.id == viewer.player_id)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| "Someone".into());
        Ok(RollJob {
            id: Uuid::new_v4().to_string(),
            roller_id: viewer.player_id.clone(),
            roller_name,
            formula: formula.split_whitespace().collect::<Vec<_>>().join(" "),
            label: clean_label(label),
            visibility,
            parsed,
        })
    }

    pub fn commit_roll(&mut self, roll: &RollEvent) {
        self.rolls.push(StoredRoll {
            id: roll.id.clone(),
            roller_id: roll.roller_id.clone(),
            roller_name: roll.roller_name.clone(),
            formula: roll.formula.clone(),
            label: roll.label.clone(),
            visibility: roll.visibility,
            modifier: roll.modifier,
            total: roll.total,
            dice: roll
                .dice
                .iter()
                .map(|die| DieResult { sides: die.sides, value: die.value, sign: die.sign, tag: die.tag.clone() })
                .collect(),
            created_at: now_ms(),
            outcome: roll.outcome.clone(),
        });
        if self.rolls.len() > ROLL_LOG_LIMIT {
            let extra = self.rolls.len() - ROLL_LOG_LIMIT;
            self.rolls.drain(0..extra);
        }
    }

    pub fn keyframe_dt() -> f32 {
        KEYFRAME_DT
    }

    fn move_token(&mut self, viewer: &Viewer, token_id: &str, x: f32, y: f32) -> Result<bool, String> {
        let (width, height) = self.bounds();
        let x = snap_cell(x, width).ok_or_else(|| "that square is off the map".to_string())?;
        let y = snap_cell(y, height).ok_or_else(|| "that square is off the map".to_string())?;
        let index = self.token_index(token_id).ok_or_else(|| "that token is not on the table".to_string())?;
        let token = &self.tokens[index];
        let gm = viewer.role == crate::net::protocol::Role::Gm;
        if !gm && token.owner_id.as_deref() != Some(viewer.player_id.as_str()) {
            return Err("you can only move your own tokens".into());
        }
        if !gm && token.hidden {
            return Err("that token is not yours to move".into());
        }
        if (token.x - x).abs() < 0.01 && (token.y - y).abs() < 0.01 {
            return Ok(false);
        }
        if !gm {
            let from = Vec2::new(token.x, token.y);
            let to = Vec2::new(x, y);
            let walls = self.blocking_walls();
            if path_blocked(from, to, &walls) {
                return Err("a wall is in the way".into());
            }
            let blockers = self.scene.as_ref().map(|scene| scene.move_blockers.as_slice()).unwrap_or(&[]);
            if path_blocked(from, to, blockers) {
                return Err("something is in the way".into());
            }
        }
        let token = &mut self.tokens[index];
        token.x = x;
        token.y = y;
        self.reveal_from_tokens();
        Ok(true)
    }

    fn set_door(&mut self, viewer: &Viewer, door_id: &str, open: bool) -> Result<(), String> {
        self.require_gm(viewer)?;
        let door = self
            .scene
            .as_mut()
            .and_then(|scene| scene.doors.iter_mut().find(|door| door.id == door_id))
            .ok_or_else(|| "that door is not on this map".to_string())?;
        door.open = open;
        self.reveal_from_tokens();
        Ok(())
    }

    fn set_lighting(&mut self, viewer: &Viewer, lighting: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        let scene = self.scene.as_mut().ok_or_else(|| "there is no map".to_string())?;
        let presentation = scene.presentation.as_mut().ok_or_else(|| "this map has no lighting to change".to_string())?;
        if !presentation.lightings.iter().any(|item| item.id == lighting) {
            return Err("that lighting is not in this package".into());
        }
        if !presentation.views.iter().all(|view| view.variants.iter().any(|variant| variant.id == lighting)) {
            return Err("that lighting is missing a picture".into());
        }
        presentation.active_lighting = lighting.to_string();
        Ok(())
    }

    fn fog_brush(&mut self, viewer: &Viewer, mode: FogBrushMode, cells: &[FogCell]) -> Result<(), String> {
        self.require_gm(viewer)?;
        if cells.len() > 500 {
            return Err("that brush stroke is too large".into());
        }
        let reveal = mode == FogBrushMode::Reveal;
        for cell in cells {
            self.explored.set(cell.x, cell.y, reveal);
        }
        Ok(())
    }

    fn create_token(
        &mut self,
        viewer: &Viewer,
        name: String,
        x: f32,
        y: f32,
        owner_id: Option<String>,
        hidden: bool,
        vision_radius: f32,
        grants_vision: bool,
        color: String,
        character_id: Option<String>,
    ) -> Result<(), String> {
        self.require_gm(viewer)?;
        let (width, height) = self.bounds();
        let x = snap_cell(x, width).unwrap_or(1.5);
        let y = snap_cell(y, height).unwrap_or(1.5);
        let owner_id = self.known_owner(owner_id)?;
        let character_id = match character_id {
            Some(id) if !id.is_empty() => {
                if self.characters.iter().any(|character| character.id == id) {
                    Some(id)
                } else {
                    return Err("that character is not at the table".into());
                }
            }
            _ => None,
        };
        let grants_vision = grants_vision && !hidden;
        self.tokens.push(Token {
            id: Uuid::new_v4().to_string(),
            name: clean_name(&name),
            x,
            y,
            image_file: None,
            color: clean_color(&color),
            owner_id,
            hidden,
            vision_radius: vision_radius.clamp(0.0, 80.0),
            grants_vision,
            character_id,
        });
        self.reveal_from_tokens();
        Ok(())
    }

    fn update_token(
        &mut self,
        viewer: &Viewer,
        token_id: &str,
        name: Option<String>,
        owner_id: Option<String>,
        clear_owner: bool,
        hidden: Option<bool>,
        vision_radius: Option<f32>,
        grants_vision: Option<bool>,
        color: Option<String>,
        character_id: Option<String>,
        clear_character: bool,
    ) -> Result<(), String> {
        self.require_gm(viewer)?;
        let owner_id = self.known_owner(owner_id)?;
        if !clear_character {
            if let Some(id) = &character_id {
                if !self.characters.iter().any(|character| &character.id == id) {
                    return Err("that character is not at the table".into());
                }
            }
        }
        let token = self
            .tokens
            .iter_mut()
            .find(|token| token.id == token_id)
            .ok_or_else(|| "that token is not on the table".to_string())?;
        if let Some(name) = name {
            token.name = clean_name(&name);
        }
        if clear_owner {
            token.owner_id = None;
        } else if owner_id.is_some() {
            token.owner_id = owner_id;
        }
        if let Some(hidden) = hidden {
            token.hidden = hidden;
            if hidden {
                token.grants_vision = false;
            }
        }
        if let Some(radius) = vision_radius {
            token.vision_radius = radius.clamp(0.0, 80.0);
        }
        if let Some(grants) = grants_vision {
            token.grants_vision = grants && !token.hidden;
        }
        if let Some(color) = color {
            token.color = clean_color(&color);
        }
        if clear_character {
            token.character_id = None;
        } else if let Some(id) = character_id {
            token.character_id = Some(id);
        }
        self.reveal_from_tokens();
        Ok(())
    }

    fn delete_token(&mut self, viewer: &Viewer, token_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        let before = self.tokens.len();
        self.tokens.retain(|token| token.id != token_id);
        if self.tokens.len() == before {
            return Err("that token is not on the table".into());
        }
        Ok(())
    }

    fn remove_player(&mut self, viewer: &Viewer, player_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if player_id == viewer.player_id {
            return Err("you cannot dismiss yourself".into());
        }
        let player = self
            .players
            .iter()
            .find(|player| player.id == player_id)
            .ok_or_else(|| "that person is not at the table".to_string())?;
        if player.connected {
            return Err("wait until they disconnect".into());
        }
        self.players.retain(|player| player.id != player_id);
        for token in &mut self.tokens {
            if token.owner_id.as_deref() == Some(player_id) {
                token.owner_id = None;
            }
        }
        Ok(())
    }

    fn set_view_as(&mut self, viewer: &Viewer, player_id: Option<String>) -> Result<(), String> {
        self.require_gm(viewer)?;
        if let Some(id) = &player_id {
            if !self.players.iter().any(|player| player.id == *id && player.role == crate::net::protocol::Role::Player) {
                return Err("pick a player to look through".into());
            }
        }
        if let Some(client) = self.clients.iter_mut().find(|client| client.player_id == viewer.player_id) {
            client.view_as = player_id;
        }
        Ok(())
    }

    fn blank_map(&mut self, viewer: &Viewer, width: u32, height: u32) -> Result<(), String> {
        self.require_gm(viewer)?;
        if !(5..=200).contains(&width) || !(5..=200).contains(&height) {
            return Err("a map is between 5 and 200 squares on a side".into());
        }
        self.install_scene(Scene {
            id: Uuid::new_v4().to_string(),
            name: "Open table".into(),
            width: width as f32,
            height: height as f32,
            image_file: None,
            walls: Vec::new(),
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            presentation: None,
        });
        Ok(())
    }

    /// Put a built map on the table. Only the GM can, and the layout is checked first.
    pub fn publish_built(&mut self, viewer: &Viewer, name: String, layout: MapLayout, compiled: Compiled) -> Result<(), String> {
        self.require_gm(viewer)?;
        layout.validate()?;
        let name = {
            let trimmed = name.trim();
            if trimmed.is_empty() { "Built map".into() } else { trimmed.chars().take(48).collect() }
        };
        self.install_scene(Scene {
            id: Uuid::new_v4().to_string(),
            name,
            width: layout.width as f32,
            height: layout.height as f32,
            image_file: None,
            walls: compiled.walls,
            doors: compiled.doors,
            lights: Vec::new(),
            layout: Some(layout),
            move_blockers: compiled.move_blockers,
            presentation: None,
        });
        Ok(())
    }

    pub fn install_scene(&mut self, scene: Scene) {
        let (width, height) = (scene.width, scene.height);
        for token in &mut self.tokens {
            token.x = token.x.clamp(0.5, (width - 0.5).max(0.5));
            token.y = token.y.clamp(0.5, (height - 0.5).max(0.5));
        }
        self.scene = Some(scene);
        let (cells_w, cells_h) = cells_for(width, height);
        self.explored.resize(cells_w, cells_h);
        self.reveal_from_tokens();
    }

    pub fn set_token_image(&mut self, token_id: &str, file: Option<String>) -> Result<(), String> {
        let token = self
            .tokens
            .iter_mut()
            .find(|token| token.id == token_id)
            .ok_or_else(|| "that token is not on the table".to_string())?;
        token.image_file = file;
        Ok(())
    }

    pub fn owns_token(&self, player_id: &str, token_id: &str) -> bool {
        self.tokens
            .iter()
            .any(|token| token.id == token_id && token.owner_id.as_deref() == Some(player_id))
    }

    // --------------------------------------------------------
    // Vision
    // --------------------------------------------------------

    pub fn reveal_from_tokens(&mut self) {
        let walls = self.blocking_walls();
        let polygons: Vec<Vec<Vec2>> = self
            .tokens
            .iter()
            .filter(|token| token.grants_vision && !token.hidden)
            .map(|token| visibility_polygon(Vec2::new(token.x, token.y), token.vision_radius, &walls))
            .collect();
        for polygon in polygons {
            self.explored.paint_polygon(&polygon);
        }
    }

    pub fn vision_polygons(&self, player_id: &str) -> Vec<Vec<Vec2>> {
        let walls = self.blocking_walls();
        self.tokens
            .iter()
            .filter(|token| {
                token.grants_vision
                    && !token.hidden
                    && (token.owner_id.is_none() || token.owner_id.as_deref() == Some(player_id))
            })
            .map(|token| visibility_polygon(Vec2::new(token.x, token.y), token.vision_radius, &walls))
            .collect()
    }

    pub fn explored_at(&self, x: f32, y: f32) -> bool {
        let scale = CELLS_PER_SQUARE as f32;
        self.explored.get((x * scale).floor() as i32, (y * scale).floor() as i32)
    }

    fn blocking_walls(&self) -> Vec<Segment> {
        let Some(scene) = &self.scene else {
            return Vec::new();
        };
        let mut walls = scene.walls.clone();
        for door in &scene.doors {
            if !door.open {
                walls.push(Segment { a: door.a, b: door.b });
            }
        }
        walls
    }

    fn token_index(&self, token_id: &str) -> Option<usize> {
        self.tokens.iter().position(|token| token.id == token_id)
    }

    fn require_gm(&self, viewer: &Viewer) -> Result<(), String> {
        if viewer.role == crate::net::protocol::Role::Gm {
            Ok(())
        } else {
            Err("only the GM can do that".into())
        }
    }

    fn known_owner(&self, owner_id: Option<String>) -> Result<Option<String>, String> {
        match owner_id {
            None => Ok(None),
            Some(id) if id.is_empty() => Ok(None),
            Some(id) => {
                if self.players.iter().any(|player| player.id == id) {
                    Ok(Some(id))
                } else {
                    Err("that player is not at the table".into())
                }
            }
        }
    }

    pub fn broadcast_message(&self, message: ServerMsg) {
        for client in &self.clients {
            let _ = client.out.send(message.clone());
        }
    }

    pub fn action_formula(
        &self,
        viewer: &Viewer,
        character_id: &str,
        action_id: &str,
        plus: i32,
        minus: i32,
        system: Option<&SystemDef>,
    ) -> Result<String, String> {
        let character = self
            .characters
            .iter()
            .find(|character| character.id == character_id)
            .ok_or_else(|| "that character is not at the table".to_string())?;
        self.require_character(viewer, character)?;
        let system = system.ok_or_else(|| "this table has no system".to_string())?;
        let formula = if let Some(action) = system.actions.iter().find(|action| action.id == action_id) {
            action.formula.clone()
        } else if let Some(field) = system.fields.iter().find(|field| field.id == action_id && field.kind == FieldKind::Dice) {
            character
                .texts
                .get(&field.id)
                .filter(|text| !text.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| field.formula.clone())
        } else {
            return Err("that roll is not on this system".into());
        };
        let filled = fill_formula(&formula, &character.numbers)?;
        Ok(apply_modifiers(&filled, plus, minus))
    }

    pub fn action_label(&self, action_id: &str, system: Option<&SystemDef>) -> Option<String> {
        system.and_then(|system| {
            system
                .actions
                .iter()
                .find(|action| action.id == action_id)
                .map(|action| action.name.clone())
                .or_else(|| {
                    system
                        .fields
                        .iter()
                        .find(|field| field.id == action_id && field.kind == FieldKind::Dice)
                        .map(|field| field.name.clone())
                })
        })
    }

    pub fn ensure_campaign(&mut self) {
        if self.campaign.id.is_empty() {
            self.campaign.id = Uuid::new_v4().to_string();
        }
        if self.campaign.name.trim().is_empty() {
            self.campaign.name = "Campaign".into();
        }
    }

    fn pack_live_scene(&mut self) {
        let Some(scene) = self.scene.take() else { return };
        let shelf = SceneShelf {
            id: scene.id.clone(),
            name: scene.name.clone(),
            scene: Some(scene),
            tokens: std::mem::take(&mut self.tokens),
            explored: self.explored.clone(),
        };
        if let Some(existing) = self.scenes.iter_mut().find(|item| item.id == shelf.id) {
            *existing = shelf;
        } else {
            self.scenes.push(shelf);
        }
    }

    fn apply_scene(&mut self, shelf: SceneShelf) {
        self.scene = shelf.scene;
        self.tokens = shelf.tokens;
        self.explored = shelf.explored;
        self.sync_fog_size();
        self.reveal_from_tokens();
    }

    fn shelf_campaign(&self) -> CampaignShelf {
        CampaignShelf {
            campaign: self.campaign.clone(),
            scene: self.scene.clone(),
            tokens: self.tokens.clone(),
            explored: self.explored.clone(),
            characters: self.characters.clone(),
            scenes: self.scenes.clone(),
        }
    }

    fn restore_campaign(&mut self, shelf: CampaignShelf) {
        self.campaign = shelf.campaign;
        self.scene = shelf.scene;
        self.tokens = shelf.tokens;
        self.explored = shelf.explored;
        self.characters = shelf.characters;
        self.scenes = shelf.scenes;
        self.ensure_campaign();
        self.sync_fog_size();
        self.reveal_from_tokens();
    }

    fn clear_table(&mut self) {
        self.scene = None;
        self.tokens.clear();
        self.characters.clear();
        self.scenes.clear();
        let (width, height) = cells_for(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        self.explored = Explored::new(width, height);
    }

    fn create_campaign(&mut self, viewer: &Viewer, name: String, system_id: Option<String>) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.shelved.len() >= 40 {
            return Err("the table has enough campaigns".into());
        }
        if let Some(id) = &system_id {
            if !crate::systems::system_id_ok(id) {
                return Err("that is not a system".into());
            }
        }
        let name = title(&name)?;
        self.shelved.push(self.shelf_campaign());
        self.campaign = Campaign { id: Uuid::new_v4().to_string(), name, system_id };
        self.clear_table();
        Ok(())
    }

    fn switch_campaign(&mut self, viewer: &Viewer, campaign_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.campaign.id == campaign_id {
            return Ok(());
        }
        let index = self
            .shelved
            .iter()
            .position(|item| item.campaign.id == campaign_id)
            .ok_or_else(|| "that campaign is not on this table".to_string())?;
        let incoming = self.shelved.remove(index);
        let outgoing = self.shelf_campaign();
        self.shelved.push(outgoing);
        self.restore_campaign(incoming);
        Ok(())
    }

    fn rename_campaign(&mut self, viewer: &Viewer, campaign_id: &str, name: String) -> Result<(), String> {
        self.require_gm(viewer)?;
        let name = title(&name)?;
        if self.campaign.id == campaign_id {
            self.campaign.name = name;
            return Ok(());
        }
        let campaign = self
            .shelved
            .iter_mut()
            .find(|item| item.campaign.id == campaign_id)
            .ok_or_else(|| "that campaign is not on this table".to_string())?;
        campaign.campaign.name = name;
        Ok(())
    }

    fn delete_campaign(&mut self, viewer: &Viewer, campaign_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.campaign.id == campaign_id {
            return Err("that campaign is the one being played".into());
        }
        let before = self.shelved.len();
        self.shelved.retain(|item| item.campaign.id != campaign_id);
        if self.shelved.len() == before {
            return Err("that campaign is not on this table".into());
        }
        Ok(())
    }

    fn create_scene(&mut self, viewer: &Viewer, name: String, width: u32, height: u32) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.scenes.len() >= 40 {
            return Err("this campaign has enough scenes".into());
        }
        if !(5..=200).contains(&width) || !(5..=200).contains(&height) {
            return Err("a map is between 5 and 200 squares on a side".into());
        }
        let name = title(&name)?;
        self.pack_live_scene();
        self.install_scene(Scene {
            id: Uuid::new_v4().to_string(),
            name,
            width: width as f32,
            height: height as f32,
            image_file: None,
            walls: Vec::new(),
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            presentation: None,
        });
        Ok(())
    }

    fn switch_scene(&mut self, viewer: &Viewer, scene_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.scene.as_ref().is_some_and(|scene| scene.id == scene_id) {
            return Ok(());
        }
        self.pack_live_scene();
        let index = self
            .scenes
            .iter()
            .position(|shelf| shelf.id == scene_id)
            .ok_or_else(|| "that scene is not in this campaign".to_string())?;
        let shelf = self.scenes.remove(index);
        self.apply_scene(shelf);
        Ok(())
    }

    fn rename_scene(&mut self, viewer: &Viewer, scene_id: &str, name: String) -> Result<(), String> {
        self.require_gm(viewer)?;
        let name = title(&name)?;
        if let Some(scene) = self.scene.as_mut().filter(|scene| scene.id == scene_id) {
            scene.name = name;
            return Ok(());
        }
        let shelf = self
            .scenes
            .iter_mut()
            .find(|shelf| shelf.id == scene_id)
            .ok_or_else(|| "that scene is not in this campaign".to_string())?;
        shelf.name = name.clone();
        if let Some(scene) = shelf.scene.as_mut() {
            scene.name = name;
        }
        Ok(())
    }

    fn delete_scene(&mut self, viewer: &Viewer, scene_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.scene.as_ref().is_some_and(|scene| scene.id == scene_id) {
            if self.scenes.is_empty() {
                self.scene = None;
                self.tokens.clear();
                let (width, height) = cells_for(DEFAULT_WIDTH, DEFAULT_HEIGHT);
                self.explored = Explored::new(width, height);
                return Ok(());
            }
            let next = self.scenes.remove(0);
            self.apply_scene(next);
            return Ok(());
        }
        let before = self.scenes.len();
        self.scenes.retain(|shelf| shelf.id != scene_id);
        if self.scenes.len() == before {
            return Err("that scene is not in this campaign".into());
        }
        Ok(())
    }

    fn set_campaign_system(&mut self, viewer: &Viewer, system_id: Option<String>) -> Result<(), String> {
        self.require_gm(viewer)?;
        if let Some(id) = &system_id {
            if !crate::systems::system_id_ok(id) {
                return Err("that is not a system".into());
            }
        }
        self.campaign.system_id = system_id;
        Ok(())
    }

    fn create_character(
        &mut self,
        viewer: &Viewer,
        name: String,
        kind: CharacterKind,
        system: Option<&SystemDef>,
    ) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.characters.len() >= 200 {
            return Err("this table is full of characters".into());
        }
        let mut numbers = BTreeMap::new();
        let mut texts = BTreeMap::new();
        if let Some(system) = system {
            let placed: HashSet<&str> = system
                .sheet_for(kind)
                .sections
                .iter()
                .flat_map(|section| section.fields.iter().map(String::as_str))
                .collect();
            for field in &system.fields {
                if !placed.contains(field.id.as_str()) {
                    continue;
                }
                match field.kind {
                    FieldKind::Text => {
                        texts.insert(field.id.clone(), field.text_default.clone());
                    }
                    FieldKind::Choice => {
                        let value = if field.options.iter().any(|option| option == &field.text_default) {
                            field.text_default.clone()
                        } else {
                            field.options.first().cloned().unwrap_or_default()
                        };
                        texts.insert(field.id.clone(), value);
                    }
                    FieldKind::Dice => {
                        texts.insert(field.id.clone(), field.formula.clone());
                    }
                    FieldKind::Number | FieldKind::Pool => {
                        numbers.insert(field.id.clone(), field.default);
                    }
                    FieldKind::Thresholds => {
                        for step in &field.steps {
                            numbers.insert(step.id.clone(), step.default);
                        }
                    }
                }
            }
        }
        self.characters.push(Character {
            id: Uuid::new_v4().to_string(),
            name: clean_name(&name),
            kind,
            owner_id: None,
            system_id: system.map(|system| system.id.clone()),
            numbers,
            texts,
        });
        Ok(())
    }

    fn update_character(
        &mut self,
        viewer: &Viewer,
        character_id: &str,
        name: Option<String>,
        kind: Option<CharacterKind>,
        owner_id: Option<String>,
        clear_owner: bool,
        numbers: BTreeMap<String, i32>,
        texts: BTreeMap<String, String>,
    ) -> Result<(), String> {
        let gm = viewer.role == crate::net::protocol::Role::Gm;
        let owner_id = if gm { self.known_owner(owner_id)? } else { None };
        let character = self
            .characters
            .iter_mut()
            .find(|character| character.id == character_id)
            .ok_or_else(|| "that character is not at the table".to_string())?;
        if !gm && character.owner_id.as_deref() != Some(viewer.player_id.as_str()) {
            return Err("that character is not yours".into());
        }
        if let Some(name) = name {
            character.name = clean_name(&name);
        }
        if gm {
            if let Some(kind) = kind {
                character.kind = kind;
            }
            if clear_owner {
                character.owner_id = None;
            } else if owner_id.is_some() {
                character.owner_id = owner_id;
            }
        }
        for (key, value) in numbers {
            character.numbers.insert(key, value);
        }
        for (key, value) in texts {
            if value.chars().count() > 4000 {
                return Err("that note is too long".into());
            }
            character.texts.insert(key, value);
        }
        Ok(())
    }

    fn delete_character(&mut self, viewer: &Viewer, character_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        let before = self.characters.len();
        self.characters.retain(|character| character.id != character_id);
        if self.characters.len() == before {
            return Err("that character is not at the table".into());
        }
        for token in &mut self.tokens {
            if token.character_id.as_deref() == Some(character_id) {
                token.character_id = None;
            }
        }
        Ok(())
    }

    fn adjust_resource(
        &mut self,
        viewer: &Viewer,
        character_id: &str,
        field_id: &str,
        delta: i32,
        system: Option<&SystemDef>,
    ) -> Result<(), String> {
        let system = system.ok_or_else(|| "this table has no system".to_string())?;
        let field = system
            .fields
            .iter()
            .find(|field| field.id == field_id)
            .ok_or_else(|| "that is not a field on this system".to_string())?;
        if !field.kind.holds_number() {
            return Err("that field is not a number or a pool".into());
        }
        let ceiling = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == character_id)
                .ok_or_else(|| "that character is not at the table".to_string())?;
            self.require_character(viewer, character)?;
            field_ceiling(field, &character.numbers)
        };
        let character = self.characters.iter_mut().find(|character| character.id == character_id).unwrap();
        let current = character.numbers.get(field_id).copied().unwrap_or(0);
        character.numbers.insert(field_id.to_string(), (current + delta).clamp(0, ceiling));
        Ok(())
    }

    fn apply_damage(
        &mut self,
        viewer: &Viewer,
        character_id: &str,
        field_id: &str,
        amount: i32,
        system: Option<&SystemDef>,
    ) -> Result<(), String> {
        if amount < 0 {
            return Err("damage cannot be negative".into());
        }
        let system = system.ok_or_else(|| "this table has no system".to_string())?;
        let field = system
            .fields
            .iter()
            .find(|field| field.id == field_id)
            .ok_or_else(|| "that is not a thing on this system".to_string())?;
        if field.kind != FieldKind::Thresholds {
            return Err("that is not a damage threshold".into());
        }
        let pool_id = field.pool.clone().ok_or_else(|| "those thresholds are not linked to a pool".to_string())?;
        let pool = system
            .fields
            .iter()
            .find(|item| item.id == pool_id && item.kind == FieldKind::Pool)
            .ok_or_else(|| "those thresholds are not linked to a pool".to_string())?;
        let (marks, ceiling, current) = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == character_id)
                .ok_or_else(|| "that character is not at the table".to_string())?;
            self.require_character(viewer, character)?;
            let Some(marks) = threshold_marks(field, amount, &character.numbers) else {
                return Ok(());
            };
            let ceiling = field_ceiling(pool, &character.numbers);
            let current = character.numbers.get(&pool_id).copied().unwrap_or(0);
            (marks, ceiling, current)
        };
        let character = self.characters.iter_mut().find(|character| character.id == character_id).unwrap();
        character.numbers.insert(pool_id, (current + marks).clamp(0, ceiling));
        Ok(())
    }

    fn require_character(&self, viewer: &Viewer, character: &Character) -> Result<(), String> {
        if viewer.role == crate::net::protocol::Role::Gm || character.owner_id.as_deref() == Some(viewer.player_id.as_str()) {
            Ok(())
        } else {
            Err("that character is not yours".into())
        }
    }
}

pub enum CommandEffect {
    Quiet,
    Refresh,
}

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn snap_cell(value: f32, limit: f32) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    let cell = value.floor();
    if cell < 0.0 || cell >= limit {
        return None;
    }
    Some(cell + 0.5)
}

fn same_key(given: &str, expected: &str) -> bool {
    let fold = |value: &str| {
        value.chars().filter(|ch| !ch.is_whitespace() && *ch != '-').flat_map(|ch| ch.to_lowercase()).collect::<String>()
    };
    !expected.is_empty() && fold(given) == fold(expected)
}

fn title(name: &str) -> Result<String, String> {
    let cleaned: String = name.chars().filter(|ch| !ch.is_control()).take(48).collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() { Err("it needs a name".into()) } else { Ok(cleaned) }
}

fn clean_name(name: &str) -> String {
    let cleaned: String = name.chars().filter(|ch| !ch.is_control()).take(24).collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() { "Guest".into() } else { cleaned.to_string() }
}

fn clean_label(label: Option<String>) -> Option<String> {
    let label = label?;
    let cleaned: String = label.chars().filter(|ch| !ch.is_control()).take(40).collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

fn clean_color(color: &str) -> String {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        format!("#{hex}")
    } else {
        "#d4a656".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Door, Scene, Segment, Vec2};
    use crate::net::protocol::Role;

    fn gm() -> Viewer {
        Viewer { player_id: "gm".into(), role: Role::Gm, view_as: None, session_token: "t".into() }
    }

    #[test]
    fn the_gm_key_ignores_case_and_spaces() {
        let mut table = Table::new();
        table.passphrase = "table".into();
        table.gm_key = "AB12CD".into();
        let seat = table.take_seat("Ada", "table", true, Some("ab12 cd"), None).unwrap();
        assert_eq!(seat.role, Role::Gm);
        let again = table.take_seat("Ada", "table", true, Some("nope"), Some(&seat.player_id));
        assert!(again.is_err());
    }

    #[test]
    fn a_player_seat_does_not_block_becoming_gm() {
        let mut table = Table::new();
        table.passphrase = "table".into();
        table.gm_key = "AB12CD".into();
        let player = table.take_seat("Ada", "table", false, None, None).unwrap();
        let gm_seat = table.take_seat("Ada", "table", true, Some("ab12cd"), Some(&player.player_id)).unwrap();
        assert_eq!(gm_seat.role, Role::Gm);
        assert_ne!(gm_seat.player_id, player.player_id);
    }

    #[test]
    fn players_cannot_walk_through_walls() {
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.install_scene(Scene {
            id: "map".into(),
            name: "Room".into(),
            width: 10.0,
            height: 10.0,
            image_file: None,
            walls: vec![Segment { a: Vec2::new(0.0, 5.0), b: Vec2::new(10.0, 5.0) }],
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            presentation: None,
        });
        table.tokens.push(Token {
            id: "ada".into(),
            name: "Ada".into(),
            x: 1.5,
            y: 1.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: Some("p".into()),
            hidden: false,
            vision_radius: 6.0,
            grants_vision: true,
            character_id: None,
        });
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let blocked = table.move_token(&player, "ada", 1.5, 6.5);
        assert!(blocked.is_err());
        assert!(table.move_token(&gm(), "ada", 1.5, 6.5).unwrap());
        assert!((table.tokens[0].y - 6.5).abs() < 0.01);
    }

    #[test]
    fn opening_a_door_lets_a_player_through() {
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.install_scene(Scene {
            id: "map".into(),
            name: "Room".into(),
            width: 10.0,
            height: 10.0,
            image_file: None,
            walls: Vec::new(),
            doors: vec![Door { id: "door-0".into(), a: Vec2::new(0.0, 5.0), b: Vec2::new(10.0, 5.0), open: false }],
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            presentation: None,
        });
        table.tokens.push(Token {
            id: "ada".into(),
            name: "Ada".into(),
            x: 2.5,
            y: 2.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: Some("p".into()),
            hidden: false,
            vision_radius: 4.0,
            grants_vision: true,
            character_id: None,
        });
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        assert!(table.move_token(&player, "ada", 2.5, 7.5).is_err());
        table.set_door(&gm(), "door-0", true).unwrap();
        assert!(table.move_token(&player, "ada", 2.5, 7.5).unwrap());
    }

    #[test]
    fn a_table_blocks_walking_but_not_sight() {
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.install_scene(Scene {
            id: "map".into(),
            name: "Room".into(),
            width: 12.0,
            height: 12.0,
            image_file: None,
            walls: Vec::new(),
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: vec![
                Segment { a: Vec2::new(1.0, 5.0), b: Vec2::new(3.0, 5.0) },
                Segment { a: Vec2::new(3.0, 5.0), b: Vec2::new(3.0, 6.0) },
                Segment { a: Vec2::new(3.0, 6.0), b: Vec2::new(1.0, 6.0) },
                Segment { a: Vec2::new(1.0, 6.0), b: Vec2::new(1.0, 5.0) },
            ],
            presentation: None,
        });
        table.tokens.push(Token {
            id: "ada".into(),
            name: "Ada".into(),
            x: 2.5,
            y: 2.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: Some("p".into()),
            hidden: false,
            vision_radius: 8.0,
            grants_vision: true,
            character_id: None,
        });
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        assert!(table.move_token(&player, "ada", 2.5, 7.5).is_err());
        let south = table
            .vision_polygons("p")
            .into_iter()
            .flatten()
            .map(|point| point.y)
            .fold(0.0_f32, f32::max);
        assert!(south > 7.0, "sight should pass a table, reached {south}");
    }

    #[test]
    fn only_the_gm_can_publish() {
        let mut table = Table::new();
        let layout = MapLayout::blank(8, 8, "core/grass");
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let err = table.publish_built(&player, "Glade".into(), layout.clone(), Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new() });
        assert!(err.is_err());
        table
            .publish_built(&gm(), "Glade".into(), layout, Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new() })
            .unwrap();
        assert_eq!(table.scene.as_ref().unwrap().name, "Glade");
        assert!(table.scene.as_ref().unwrap().layout.is_some());
    }

    #[test]
    fn resources_clamp_and_a_roll_uses_the_servers_stat() {
        use crate::net::protocol::ClientMsg;
        use crate::systems::{load_systems, Character, CharacterKind};
        let catalog = load_systems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../systems").as_path());
        let system = catalog.get("daggerheart").unwrap();
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Ada".into(), kind: CharacterKind::Pc }, Some(system))
            .unwrap();
        table.characters[0].owner_id = Some("p".into());
        table.characters[0].numbers.insert("agility".into(), -1);
        table.characters[0].numbers.insert("hpmax".into(), 6);
        table.characters[0].numbers.insert("hp".into(), 1);
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let id = table.characters[0].id.clone();
        table
            .command(&player, ClientMsg::AdjustResource { character_id: id.clone(), field: "hp".into(), delta: 100 }, Some(system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(6));
        table
            .command(&player, ClientMsg::AdjustResource { character_id: id.clone(), field: "hp".into(), delta: -100 }, Some(system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));
        let formula = table.action_formula(&player, &id, "agility", 0, 0, Some(system)).unwrap();
        assert_eq!(formula, "1d12h+1d12f-1");
        assert!(!formula.contains("+-"));
        table.characters.push(Character {
            id: "foe".into(),
            name: "Foe".into(),
            kind: CharacterKind::Adversary,
            owner_id: None,
            system_id: None,
            numbers: Default::default(),
            texts: Default::default(),
        });
        let err = table.command(
            &player,
            ClientMsg::UpdateCharacter {
                character_id: "foe".into(),
                name: Some("no".into()),
                kind: None,
                owner_id: None,
                clear_owner: false,
                numbers: Default::default(),
                texts: Default::default(),
            },
            Some(system),
        );
        assert!(err.is_err());
    }

    #[test]
    fn damage_marks_the_linked_pool_and_a_new_character_only_gets_its_sheet() {
        use crate::net::protocol::ClientMsg;
        use crate::systems::{CharacterKind, SystemDef};
        let system: SystemDef = serde_json::from_str(
            r#"{
                "id": "sample",
                "name": "Sample",
                "fields": [
                    {"id": "hpmax", "name": "HP max", "kind": "number", "default": 6},
                    {"id": "hp", "name": "HP", "kind": "pool", "max": 6, "maxFrom": "hpmax"},
                    {"id": "armor", "name": "Armor", "kind": "pool", "max": 3},
                    {"id": "str", "name": "Strength", "kind": "number", "default": 2},
                    {"id": "thresholds", "name": "Thresholds", "kind": "thresholds", "pool": "hp", "steps": [
                        {"id": "minor", "name": "Minor", "default": 1, "marks": 1},
                        {"id": "major", "name": "Major", "default": 7, "marks": 2},
                        {"id": "severe", "name": "Severe", "default": 13, "marks": 3}
                    ]},
                    {"id": "damage", "name": "Damage", "kind": "dice", "formula": "1d6+@str"}
                ],
                "sheets": {
                    "pc": {
                        "sections": [{"title": "Vitals", "columns": 2, "fields": ["hpmax", "hp", "armor", "str", "thresholds", "damage"]}],
                        "toolbar": {"resources": ["hp"], "actions": ["damage"]}
                    },
                    "adversary": {
                        "sections": [{"title": "Card", "columns": 1, "fields": ["hpmax", "hp", "thresholds"]}],
                        "toolbar": {"resources": ["hp"], "actions": []}
                    }
                }
            }"#,
        )
        .unwrap();
        let mut table = Table::new();
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Ada".into(), kind: CharacterKind::Pc }, Some(&system))
            .unwrap();
        assert!(table.characters[0].numbers.contains_key("armor"));
        assert_eq!(table.characters[0].numbers.get("minor").copied(), Some(1));
        assert_eq!(table.characters[0].texts.get("damage").map(String::as_str), Some("1d6+@str"));
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Foe".into(), kind: CharacterKind::Adversary }, Some(&system))
            .unwrap();
        let foe = &table.characters[1];
        assert!(!foe.numbers.contains_key("armor"));
        assert!(!foe.numbers.contains_key("str"));
        assert!(foe.numbers.contains_key("hp"));
        assert!(foe.numbers.contains_key("major"));

        let id = table.characters[0].id.clone();
        table.characters[0].numbers.insert("hp".into(), 0);
        table
            .command(&gm(), ClientMsg::ApplyDamage { character_id: id.clone(), field_id: "thresholds".into(), amount: 0 }, Some(&system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));
        table
            .command(&gm(), ClientMsg::ApplyDamage { character_id: id.clone(), field_id: "thresholds".into(), amount: 1 }, Some(&system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(1));
        table
            .command(&gm(), ClientMsg::ApplyDamage { character_id: id.clone(), field_id: "thresholds".into(), amount: 7 }, Some(&system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(3));
        table
            .command(&gm(), ClientMsg::ApplyDamage { character_id: id.clone(), field_id: "thresholds".into(), amount: 13 }, Some(&system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(6));
        table
            .command(&gm(), ClientMsg::ApplyDamage { character_id: id.clone(), field_id: "thresholds".into(), amount: 20 }, Some(&system))
            .unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(6));

        table.characters[0].texts.insert("damage".into(), "2d6+@str".into());
        let formula = table.action_formula(&gm(), &id, "damage", 0, 0, Some(&system)).unwrap();
        assert_eq!(formula, "2d6+2");
    }

    #[test]
    fn switching_scene_or_campaign_keeps_tokens_and_fog() {
        use crate::net::protocol::ClientMsg;
        let mut table = Table::new();
        table.scene = Some(Scene {
            id: "hall".into(),
            name: "Hall".into(),
            width: 20.0,
            height: 20.0,
            image_file: None,
            walls: Vec::new(),
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            presentation: None,
        });
        table.tokens.push(Token {
            id: "hero".into(),
            name: "Ada".into(),
            x: 2.5,
            y: 2.5,
            image_file: None,
            color: "#d4a656".into(),
            owner_id: None,
            hidden: false,
            vision_radius: 8.0,
            grants_vision: true,
            character_id: None,
        });
        table.explored.set(0, 0, true);
        table
            .command(&gm(), ClientMsg::CreateScene { name: "Cellar".into(), width: 10, height: 10 }, None)
            .unwrap();
        assert_eq!(table.scene.as_ref().unwrap().name, "Cellar");
        assert!(table.tokens.is_empty());
        assert!(!table.explored.get(0, 0));
        table.command(&gm(), ClientMsg::SwitchScene { scene_id: "hall".into() }, None).unwrap();
        assert_eq!(table.tokens[0].id, "hero");
        assert!(table.explored.get(0, 0));

        table
            .command(&gm(), ClientMsg::CreateCampaign { name: "Next".into(), system_id: None }, None)
            .unwrap();
        assert_eq!(table.campaign.name, "Next");
        assert!(table.tokens.is_empty());
        let previous = table.shelved[0].campaign.id.clone();
        table.command(&gm(), ClientMsg::SwitchCampaign { campaign_id: previous }, None).unwrap();
        assert_eq!(table.campaign.name, "Campaign");
        assert_eq!(table.tokens[0].id, "hero");
        assert!(table.explored.get(0, 0));
    }

    #[test]
    fn lighting_changes_the_picture_without_resetting_the_table() {
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        let picture = |lighting: &str| crate::map::presentation::SavedVariant {
            id: lighting.into(),
            file: format!("views/view-0/{lighting}.png"),
        };
        table.install_scene(Scene {
            id: "camp".into(),
            name: "Camp".into(),
            width: 20.0,
            height: 20.0,
            image_file: None,
            walls: vec![Segment { a: Vec2::new(1.0, 1.0), b: Vec2::new(4.0, 1.0) }],
            doors: Vec::new(),
            lights: Vec::new(),
            layout: None,
            move_blockers: Vec::new(),
            presentation: Some(crate::map::Prerendered {
                active_lighting: "day".into(),
                default_view: "view-0".into(),
                view_order: vec!["view-0".into(), "view-1".into()],
                lightings: vec![
                    crate::map::presentation::LightingDef { id: "day".into(), label: "Day".into() },
                    crate::map::presentation::LightingDef { id: "night".into(), label: "Night".into() },
                ],
                views: vec![crate::map::presentation::SavedView {
                    id: "view-0".into(),
                    azimuth_deg: 0.0,
                    elevation_deg: 53.13,
                    ortho_half_width: 12.0,
                    ortho_half_height: 10.0,
                    target: [10.0, 1.0, 10.0],
                    image_size: [32, 32],
                    ground_to_image: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    height_to_image: [0.0, -1.0],
                    pull: 40.0,
                    near: 0.05,
                    far: 200.0,
                    pixels_per_square: 32.0,
                    depth_file: None,
                    variants: vec![picture("day"), picture("night")],
                }],
                doors: Vec::new(),
                capabilities: crate::map::presentation::Capabilities {
                    depth: true,
                    doors: false,
                    move_blockers: false,
                    walls: true,
                    lights: false,
                    snow: false,
                },
                warnings: vec!["Furniture does not block movement.".into()],
            }),
        });
        table.tokens.push(Token {
            id: "ada".into(),
            name: "Ada".into(),
            x: 2.5,
            y: 2.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: Some("p".into()),
            hidden: false,
            vision_radius: 2.0,
            grants_vision: true,
            character_id: None,
        });
        table.explored.set(19, 19, true);
        let player = Viewer {
            player_id: "p".into(),
            role: Role::Player,
            view_as: None,
            session_token: "seat".into(),
        };
        assert!(table.command(&player, ClientMsg::SetLighting { lighting: "night".into() }, None).is_err());
        table.command(&gm(), ClientMsg::SetLighting { lighting: "night".into() }, None).unwrap();
        let scene = table.scene.as_ref().unwrap();
        assert_eq!(scene.id, "camp");
        assert_eq!(scene.presentation.as_ref().unwrap().active_lighting, "night");
        assert_eq!(scene.walls.len(), 1);
        assert_eq!(table.tokens.len(), 1);
        assert!(table.explored.get(19, 19));
    }
}
