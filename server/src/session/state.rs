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
use crate::map::{MapLayout, Scene, Segment, Vec2, DIMETRIC_PROFILE};
use crate::net::protocol::{
    Blow, ClientMsg, ConcealedRoll, Contest, DieResult, FogBrushMode, FogCell, RollEvent, RollVisibility, ServerMsg, TalkLine,
};
use crate::systems::{
    apply_modifiers, field_ceiling, fill_formula, threshold_blow, threshold_marks, Character, CharacterKind, FieldDef, FieldKind,
    SystemDef,
    Campaign,
};
use crate::vision::{cells_for, path_blocked, visibility_polygon, Explored, CELLS_PER_SQUARE};

use super::permissions::{can_see_roll, conceal_notice, snapshot};

const COLORS: [&str; 6] = ["#d4a656", "#7eb8a2", "#d27b66", "#8aa6d4", "#c98bb8", "#d8c7a1"];
pub const DEFAULT_WIDTH: f32 = 40.0;
pub const DEFAULT_HEIGHT: f32 = 30.0;
const ROLL_LOG_LIMIT: usize = 40;
const FEAR_CAP: u8 = 12;

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
    /// When true, `cast_asset` replaces the character default, even if it is empty.
    #[serde(default)]
    pub cast_override: bool,
    #[serde(default)]
    pub cast_asset: Option<String>,
    #[serde(default = "facing_south")]
    pub facing: String,
}

fn facing_south() -> String {
    "south".into()
}

pub fn cardinal_facing(from_x: f32, from_y: f32, to_x: f32, to_y: f32, previous: &str) -> String {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let ax = dx.abs();
    let ay = dy.abs();
    let previous = match previous {
        "north" | "east" | "south" | "west" => previous,
        _ => "south",
    };
    if ax < 0.01 && ay < 0.01 {
        return previous.to_string();
    }
    // A tie keeps the current facing so a diagonal step does not flicker.
    if (ax - ay).abs() <= 0.01 {
        return previous.to_string();
    }
    if ax > ay {
        if dx > 0.0 { "east".into() } else { "west".into() }
    } else if dy > 0.0 {
        "south".into()
    } else {
        "north".into()
    }
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
    #[serde(default)]
    pub contest: Option<Contest>,
}

/// Damage that is waiting for an armor answer.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PendingStrike {
    pub target_id: String,
    pub step: String,
    pub marks: i32,
}

/// One number, as it was before the last mark.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MarkSlot {
    Field { character_id: String, field: String, before: i32 },
    Fear { before: u8 },
}

/// The last mark. One step, not a history.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UndoStep {
    pub changes: Vec<MarkSlot>,
}

/// A targeted roll the socket is about to throw.
pub struct AttackPlan {
    pub formula: String,
    pub label: String,
    pub contest: Contest,
    pub damage: bool,
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
    pub fear: u8,
    pub strike: Option<PendingStrike>,
    pub undo: Option<UndoStep>,
    pub talk: Vec<TalkLine>,
    pub catalog_revision: String,
    voices: HashMap<String, String>,
    clients: Vec<ClientConn>,
    sessions: HashMap<String, String>,
    epoch: u64,
    next_color: usize,
}

fn roll_speech(roll: &RollEvent) -> String {
    if let Some(contest) = &roll.contest {
        if let Some(verdict) = &contest.verdict {
            let word = if verdict == "hit" { "Hit" } else { "Miss" };
            return format!(
                "attacks {} — {} ({} vs {} {})",
                contest.target_name, word, roll.total, contest.defense_name, contest.defense
            );
        }
        if let Some(blow) = &contest.blow {
            if blow.marks > 0 {
                return format!("damages {} — {} · {} HP", contest.target_name, blow.step, blow.marks);
            }
            return format!("damages {} — no mark", contest.target_name);
        }
    }
    let what = roll.label.clone().filter(|label| !label.is_empty()).unwrap_or_else(|| roll.formula.clone());
    let tone = roll.outcome.as_ref().map(|outcome| format!(" {}", outcome.label)).unwrap_or_default();
    format!("{what} {}{tone}", roll.total)
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
            fear: 0,
            strike: None,
            undo: None,
            talk: Vec::new(),
            catalog_revision: String::new(),
            voices: HashMap::new(),
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
            ClientMsg::Roll { .. } | ClientMsg::Attack { .. } | ClientMsg::Strike { .. } => {
                Err("rolls are started by the socket, not the table".into())
            }
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
            ClientMsg::CreateToken { name, x, y, owner_id, hidden, vision_radius, grants_vision, color, character_id, cast_asset, facing } => {
                self.create_token(viewer, name, x, y, owner_id, hidden, vision_radius, grants_vision, color, character_id, cast_asset, facing)?;
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
                set_cast_asset,
                cast_asset,
                facing,
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
                    set_cast_asset,
                    cast_asset,
                    facing,
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
                set_cast_asset,
                cast_asset_id,
            } => {
                self.update_character(
                    viewer,
                    &character_id,
                    name,
                    kind,
                    owner_id,
                    clear_owner,
                    numbers,
                    texts,
                    set_cast_asset,
                    cast_asset_id,
                )?;
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
            ClientMsg::AdjustFear { delta } => {
                self.adjust_fear(viewer, delta)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::ApplyDamage { character_id, field_id, amount } => {
                self.apply_damage(viewer, &character_id, &field_id, amount, system)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::AnswerStrike { use_armor } => {
                self.answer_strike(viewer, use_armor, system)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::Undo => {
                self.undo_mark(viewer)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::Voice { character_id } => {
                self.set_voice(viewer, character_id)?;
                Ok(CommandEffect::Quiet)
            }
            ClientMsg::Say { text } => {
                self.say(viewer, text)?;
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
            contest: roll.contest.clone(),
        });
        if self.rolls.len() > ROLL_LOG_LIMIT {
            let extra = self.rolls.len() - ROLL_LOG_LIMIT;
            self.rolls.drain(0..extra);
        }
        if self.campaign.system_id.as_deref() == Some("daggerheart")
            && roll.visibility == RollVisibility::Public
            && roll.outcome.as_ref().is_some_and(|outcome| outcome.tone == "fear")
        {
            self.fear = self.fear.saturating_add(1).min(FEAR_CAP);
        }
        if roll.visibility == RollVisibility::Public {
            let speaker = roll
                .contest
                .as_ref()
                .map(|contest| contest.attacker_name.clone())
                .unwrap_or_else(|| self.speaker_for(&roll.roller_id));
            self.push_talk(speaker, roll_speech(roll));
        }
    }

    pub fn set_voice(&mut self, viewer: &Viewer, character_id: Option<String>) -> Result<(), String> {
        let Some(character_id) = character_id.filter(|id| !id.is_empty()) else {
            self.voices.remove(&viewer.player_id);
            return Ok(());
        };
        let character = self
            .characters
            .iter()
            .find(|character| character.id == character_id)
            .ok_or_else(|| "that character is not at the table".to_string())?;
        let gm = viewer.role == crate::net::protocol::Role::Gm;
        if !gm && character.owner_id.as_deref() != Some(viewer.player_id.as_str()) {
            return Err("you can only speak as your own character".into());
        }
        self.voices.insert(viewer.player_id.clone(), character_id);
        Ok(())
    }

    pub fn say(&mut self, viewer: &Viewer, text: String) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("say something first".into());
        }
        let body: String = text.chars().take(280).collect();
        self.push_talk(self.speaker_for(&viewer.player_id), body);
        Ok(())
    }

    fn speaker_for(&self, player_id: &str) -> String {
        if let Some(character_id) = self.voices.get(player_id) {
            if let Some(character) = self.characters.iter().find(|character| &character.id == character_id) {
                return character.name.clone();
            }
        }
        self.players
            .iter()
            .find(|player| player.id == player_id)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| "Someone".into())
    }

    fn push_talk(&mut self, speaker: String, body: String) {
        self.talk.push(TalkLine {
            id: Uuid::new_v4().to_string(),
            speaker,
            body,
            at: now_ms(),
        });
        if self.talk.len() > 80 {
            let extra = self.talk.len() - 80;
            self.talk.drain(0..extra);
        }
    }

    /// Drop every socket. Used when the open table changes.
    pub fn vacate(&mut self) {
        let notice = ServerMsg::Kicked {
            message: "The table changed. Sit down again.".into(),
        };
        for client in &self.clients {
            let _ = client.out.send(notice.clone());
        }
        self.clients.clear();
        self.sessions.clear();
        for player in &mut self.players {
            player.connected = false;
        }
    }

    fn adjust_fear(&mut self, viewer: &Viewer, delta: i32) -> Result<(), String> {
        self.require_gm(viewer)?;
        let before = self.fear;
        let next = self.fear as i32 + delta;
        self.fear = next.clamp(0, FEAR_CAP as i32) as u8;
        if self.fear != before {
            self.remember(vec![MarkSlot::Fear { before }]);
        }
        Ok(())
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
        let facing = cardinal_facing(token.x, token.y, x, y, &token.facing);
        token.x = x;
        token.y = y;
        token.facing = facing;
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
        cast_asset: Option<String>,
        facing: Option<String>,
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
            cast_override: cast_asset.is_some(),
            cast_asset,
            facing: facing.unwrap_or_else(|| "south".into()),
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
        set_cast_asset: bool,
        cast_asset: Option<String>,
        facing: Option<String>,
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
        if set_cast_asset {
            token.cast_override = true;
            token.cast_asset = cast_asset.filter(|asset| !asset.is_empty());
        }
        if let Some(facing) = facing {
            token.facing = cardinal_facing(0.0, 0.0, 0.0, 0.0, &facing);
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
        if !self.players.iter().any(|player| player.id == player_id) {
            return Err("that person is not at the table".into());
        }
        let notice = ServerMsg::Kicked {
            message: "The GM asked you to leave.".into(),
        };
        for client in self.clients.iter().filter(|client| client.player_id == player_id) {
            let _ = client.out.send(notice.clone());
        }
        self.clients.retain(|client| client.player_id != player_id);
        for client in &mut self.clients {
            if client.view_as.as_deref() == Some(player_id) {
                client.view_as = None;
            }
        }
        self.sessions.retain(|_, id| id != player_id);
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
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
        });
        Ok(())
    }

    /// Put a built map on the table. `update_id` keeps that scene when it is the one open.
    pub fn publish_built(
        &mut self,
        viewer: &Viewer,
        name: String,
        layout: MapLayout,
        compiled: Compiled,
        update_id: Option<String>,
    ) -> Result<String, String> {
        self.require_gm(viewer)?;
        layout.validate()?;
        let name = {
            let trimmed = name.trim();
            if trimmed.is_empty() { "Built map".into() } else { trimmed.chars().take(48).collect() }
        };
        let id = if let Some(id) = update_id {
            if self.scene.as_ref().is_some_and(|scene| scene.id == id) {
                id
            } else {
                return Err("that scene is not the one on the table".into());
            }
        } else {
            Uuid::new_v4().to_string()
        };
        let mut doors = compiled.doors;
        let preserve = self.scene.as_ref().is_some_and(|scene| {
            scene.id == id && (scene.width - layout.width as f32).abs() < 0.01 && (scene.height - layout.height as f32).abs() < 0.01
        });
        if let Some(existing) = self.scene.as_ref().filter(|scene| scene.id == id) {
            for door in &mut doors {
                if let Some(previous) = existing.doors.iter().find(|item| item.id == door.id) {
                    door.open = previous.open;
                }
            }
        }
        let memory = if preserve { Some(self.explored.clone()) } else { None };
        let width = layout.width as f32;
        let height = layout.height as f32;
        self.install_scene(Scene {
            id: id.clone(),
            name,
            width,
            height,
            image_file: None,
            walls: compiled.walls,
            doors,
            lights: compiled.lights,
            layout: Some(layout),
            move_blockers: compiled.move_blockers,
            presentation: None,
            schema_version: 2,
            visual_profile: DIMETRIC_PROFILE.to_string(),
        });
        if let Some(memory) = memory {
            self.explored = memory;
            self.reveal_from_tokens();
        }
        Ok(id)
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
        self.strike = None;
        self.undo = None;
        self.ensure_campaign();
        self.sync_fog_size();
        self.reveal_from_tokens();
    }

    fn clear_table(&mut self) {
        self.scene = None;
        self.tokens.clear();
        self.characters.clear();
        self.scenes.clear();
        self.strike = None;
        self.undo = None;
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
            if let Some(next) = self.shelved.pop() {
                self.restore_campaign(next);
            } else {
                self.campaign = Campaign {
                    id: Uuid::new_v4().to_string(),
                    name: "Campaign".into(),
                    system_id: None,
                };
                self.clear_table();
            }
            return Ok(());
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
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
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
            cast_asset_id: None,
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
        set_cast_asset: bool,
        cast_asset_id: Option<String>,
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
        if set_cast_asset {
            let asset = cast_asset_id.filter(|asset| !asset.is_empty());
            if let Some(asset) = &asset {
                if crate::assets::split_asset(asset).is_none() {
                    return Err("cast art is named pack/id".into());
                }
            }
            character.cast_asset_id = asset;
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
        let current = self
            .characters
            .iter()
            .find(|character| character.id == character_id)
            .and_then(|character| character.numbers.get(field_id).copied())
            .unwrap_or(0);
        let next = (current + delta).clamp(0, ceiling);
        if let Some(character) = self.characters.iter_mut().find(|character| character.id == character_id) {
            character.numbers.insert(field_id.to_string(), next);
        }
        if next != current {
            self.remember(vec![MarkSlot::Field {
                character_id: character_id.to_string(),
                field: field_id.to_string(),
                before: current,
            }]);
        }
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
        let next = (current + marks).clamp(0, ceiling);
        if let Some(character) = self.characters.iter_mut().find(|character| character.id == character_id) {
            character.numbers.insert(pool_id.clone(), next);
        }
        if next != current {
            self.remember(vec![MarkSlot::Field { character_id: character_id.to_string(), field: pool_id, before: current }]);
        }
        Ok(())
    }

    pub fn prepare_attack(
        &self,
        viewer: &Viewer,
        attacker_id: &str,
        target_id: &str,
        plus: i32,
        minus: i32,
    ) -> Result<AttackPlan, String> {
        self.engagement(viewer, attacker_id, target_id, plus, minus, false)
    }

    pub fn prepare_strike(
        &self,
        viewer: &Viewer,
        attacker_id: &str,
        target_id: &str,
        plus: i32,
        minus: i32,
    ) -> Result<AttackPlan, String> {
        if self.strike.is_some() {
            return Err("answer the last blow first".into());
        }
        self.engagement(viewer, attacker_id, target_id, plus, minus, true)
    }

    fn engagement(
        &self,
        viewer: &Viewer,
        attacker_id: &str,
        target_id: &str,
        plus: i32,
        minus: i32,
        damage: bool,
    ) -> Result<AttackPlan, String> {
        let attacker = self
            .characters
            .iter()
            .find(|character| character.id == attacker_id)
            .ok_or_else(|| "that character is not at the table".to_string())?;
        self.require_character(viewer, attacker)?;
        let target = self
            .characters
            .iter()
            .find(|character| character.id == target_id)
            .ok_or_else(|| "that target is not at the table".to_string())?;
        let (defense_name, defense) = crate::systems::daggerheart::defense_of(target);
        let raw = if damage {
            crate::systems::daggerheart::damage_formula(attacker)
        } else {
            crate::systems::daggerheart::attack_formula(attacker)
        };
        let filled = fill_formula(&raw, &attacker.numbers)?;
        let verb = if damage { "damages" } else { "attacks" };
        Ok(AttackPlan {
            formula: apply_modifiers(&filled, plus, minus),
            label: format!("{} {} {}", attacker.name, verb, target.name),
            contest: Contest {
                attacker_id: attacker.id.clone(),
                attacker_name: attacker.name.clone(),
                target_id: target.id.clone(),
                target_name: target.name.clone(),
                defense_name: defense_name.into(),
                defense,
                verdict: None,
                blow: None,
            },
            damage,
        })
    }

    /// Compare a damage total with the target's thresholds. Armor, when a slot is free, waits for an answer.
    pub fn receive_blow(&mut self, target_id: &str, amount: i32, system: &SystemDef) -> Result<Blow, String> {
        if self.strike.is_some() {
            return Err("answer the last blow first".into());
        }
        let field = system
            .fields
            .iter()
            .find(|field| field.kind == FieldKind::Thresholds)
            .ok_or_else(|| "this system has no damage thresholds".to_string())?;
        let (step, marks, open) = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == target_id)
                .ok_or_else(|| "that target is not at the table".to_string())?;
            let (step, marks) = threshold_blow(field, amount, &character.numbers).unwrap_or_else(|| ("None".into(), 0));
            (step, marks, armor_open(character))
        };
        let blow = Blow { step: step.clone(), marks };
        if marks <= 0 {
            return Ok(blow);
        }
        if open > 0 {
            self.strike = Some(PendingStrike { target_id: target_id.to_string(), step, marks });
            return Ok(blow);
        }
        if let Some(slot) = self.add_pool(target_id, field, marks, system)? {
            self.remember(vec![slot]);
        }
        Ok(blow)
    }

    fn answer_strike(&mut self, viewer: &Viewer, use_armor: bool, system: Option<&SystemDef>) -> Result<(), String> {
        let pending = self.strike.clone().ok_or_else(|| "there is no blow waiting".to_string())?;
        let system = system.ok_or_else(|| "this table has no system".to_string())?;
        let field = system
            .fields
            .iter()
            .find(|field| field.kind == FieldKind::Thresholds)
            .ok_or_else(|| "this system has no damage thresholds".to_string())?
            .clone();
        let (open, hp_before, armor_before) = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == pending.target_id)
                .ok_or_else(|| "that target is not at the table".to_string())?;
            let gm = viewer.role == crate::net::protocol::Role::Gm;
            let owner = character.owner_id.as_deref() == Some(viewer.player_id.as_str());
            if !gm && !owner {
                return Err("that character is not yours".into());
            }
            (
                armor_open(character),
                character.numbers.get("hp").copied().unwrap_or(0),
                character.numbers.get("armor").copied().unwrap_or(0),
            )
        };
        let spend = use_armor && open > 0 && pending.marks > 0;
        let hp_marks = if spend { (pending.marks - 1).max(0) } else { pending.marks };
        let mut changes = Vec::new();
        if hp_marks > 0 {
            if let Some(slot) = self.add_pool(&pending.target_id, &field, hp_marks, system)? {
                changes.push(slot);
            } else {
                changes.push(MarkSlot::Field { character_id: pending.target_id.clone(), field: "hp".into(), before: hp_before });
            }
        }
        if spend {
            if let Some(slot) = self.add_number(&pending.target_id, "armor", 1, system)? {
                changes.push(slot);
            } else {
                changes.push(MarkSlot::Field {
                    character_id: pending.target_id.clone(),
                    field: "armor".into(),
                    before: armor_before,
                });
            }
        }
        self.strike = None;
        self.remember(changes);
        Ok(())
    }

    fn add_pool(
        &mut self,
        character_id: &str,
        field: &FieldDef,
        marks: i32,
        system: &SystemDef,
    ) -> Result<Option<MarkSlot>, String> {
        let pool_id = field.pool.clone().ok_or_else(|| "those thresholds are not linked to a pool".to_string())?;
        let pool = system
            .fields
            .iter()
            .find(|item| item.id == pool_id && item.kind == FieldKind::Pool)
            .ok_or_else(|| "those thresholds are not linked to a pool".to_string())?;
        let (ceiling, current) = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == character_id)
                .ok_or_else(|| "that character is not at the table".to_string())?;
            (field_ceiling(pool, &character.numbers), character.numbers.get(&pool_id).copied().unwrap_or(0))
        };
        let next = (current + marks).clamp(0, ceiling);
        if let Some(character) = self.characters.iter_mut().find(|character| character.id == character_id) {
            character.numbers.insert(pool_id.clone(), next);
        }
        if next != current {
            return Ok(Some(MarkSlot::Field { character_id: character_id.to_string(), field: pool_id, before: current }));
        }
        Ok(None)
    }

    fn add_number(&mut self, character_id: &str, field_id: &str, delta: i32, system: &SystemDef) -> Result<Option<MarkSlot>, String> {
        let field = system
            .fields
            .iter()
            .find(|field| field.id == field_id)
            .ok_or_else(|| "that is not a field on this system".to_string())?;
        let (ceiling, current) = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == character_id)
                .ok_or_else(|| "that character is not at the table".to_string())?;
            (field_ceiling(field, &character.numbers), character.numbers.get(field_id).copied().unwrap_or(0))
        };
        let next = (current + delta).clamp(0, ceiling);
        if let Some(character) = self.characters.iter_mut().find(|character| character.id == character_id) {
            character.numbers.insert(field_id.to_string(), next);
        }
        if next != current {
            return Ok(Some(MarkSlot::Field {
                character_id: character_id.to_string(),
                field: field_id.to_string(),
                before: current,
            }));
        }
        Ok(None)
    }

    fn undo_mark(&mut self, viewer: &Viewer) -> Result<(), String> {
        let step = self.undo.clone().ok_or_else(|| "nothing to undo".to_string())?;
        let gm = viewer.role == crate::net::protocol::Role::Gm;
        for change in &step.changes {
            match change {
                MarkSlot::Fear { .. } if !gm => return Err("only the GM can undo that".into()),
                MarkSlot::Field { character_id, .. } => {
                    let character = self
                        .characters
                        .iter()
                        .find(|character| &character.id == character_id)
                        .ok_or_else(|| "that character is not at the table".to_string())?;
                    let owner = character.owner_id.as_deref() == Some(viewer.player_id.as_str());
                    if !gm && !owner {
                        return Err("that character is not yours".into());
                    }
                }
                MarkSlot::Fear { .. } => {}
            }
        }
        for change in &step.changes {
            match change {
                MarkSlot::Fear { before } => self.fear = *before,
                MarkSlot::Field { character_id, field, before } => {
                    if let Some(character) = self.characters.iter_mut().find(|character| &character.id == character_id) {
                        character.numbers.insert(field.clone(), *before);
                    }
                }
            }
        }
        self.undo = None;
        Ok(())
    }

    fn remember(&mut self, changes: Vec<MarkSlot>) {
        if changes.is_empty() {
            return;
        }
        self.undo = Some(UndoStep { changes });
    }

    fn require_character(&self, viewer: &Viewer, character: &Character) -> Result<(), String> {
        if viewer.role == crate::net::protocol::Role::Gm || character.owner_id.as_deref() == Some(viewer.player_id.as_str()) {
            Ok(())
        } else {
            Err("that character is not yours".into())
        }
    }
}

fn armor_open(character: &Character) -> i32 {
    let score = character.numbers.get("armorscore").copied().unwrap_or(0);
    let marked = character.numbers.get("armor").copied().unwrap_or(0);
    (score - marked).max(0)
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
    let cleaned: String = name.chars().filter(|ch| !ch.is_control()).take(48).collect();
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
    fn the_gm_can_kick_a_seat_that_is_still_connected() {
        let mut table = Table::new();
        table.passphrase = "table".into();
        table.gm_key = "AB12CD".into();
        let gm_seat = table.take_seat("Ada", "table", true, Some("AB12CD"), None).unwrap();
        let player = table.take_seat("Bri", "table", false, None, None).unwrap();
        let (gm_tx, _gm_rx) = mpsc::unbounded_channel();
        let (gm_epoch, _) = table.attach(&gm_seat.player_id, gm_tx).unwrap();
        let (player_tx, mut player_rx) = mpsc::unbounded_channel();
        let (_, player_token) = table.attach(&player.player_id, player_tx).unwrap();
        table.tokens.push(Token {
            id: "bri".into(),
            name: "Bri".into(),
            x: 1.5,
            y: 1.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: Some(player.player_id.clone()),
            hidden: false,
            vision_radius: 6.0,
            grants_vision: true,
            character_id: None,
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
            });

        let gm_viewer = table.viewer(&gm_seat.player_id, gm_epoch).unwrap();
        table
            .command(&gm_viewer, ClientMsg::ViewAs { player_id: Some(player.player_id.clone()) }, None)
            .unwrap();
        let gm_viewer = table.viewer(&gm_seat.player_id, gm_epoch).unwrap();
        table
            .command(&gm_viewer, ClientMsg::RemovePlayer { player_id: player.player_id.clone() }, None)
            .unwrap();

        match player_rx.try_recv().unwrap() {
            ServerMsg::Kicked { message } => assert_eq!(message, "The GM asked you to leave."),
            other => panic!("expected a kick, got {other:?}"),
        }
        assert!(table.players.iter().all(|seat| seat.id != player.player_id));
        assert!(table.viewer_for_session(&player_token).is_none());
        assert_eq!(table.tokens[0].owner_id, None);
        assert!(table.viewer(&gm_seat.player_id, gm_epoch).unwrap().view_as.is_none());

        let again = table.take_seat("Bri", "table", false, None, Some(&player.player_id)).unwrap();
        assert_ne!(again.player_id, player.player_id);
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
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
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
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
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
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
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
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
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
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
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
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
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
        let err = table.publish_built(&player, "Glade".into(), layout.clone(), Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new() }, None);
        assert!(err.is_err());
        table
            .publish_built(&gm(), "Glade".into(), layout, Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new() }, None)
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
            cast_asset_id: None,
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
                set_cast_asset: false,
                cast_asset_id: None,
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
    fn armor_softens_one_step_and_undo_puts_both_back() {
        use crate::net::protocol::ClientMsg;
        use crate::systems::{load_systems, CharacterKind};
        let catalog = load_systems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../systems").as_path());
        let system = catalog.get("daggerheart").unwrap();
        let mut table = Table::new();
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Ada".into(), kind: CharacterKind::Pc }, Some(system))
            .unwrap();
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Bandit".into(), kind: CharacterKind::Adversary }, Some(system))
            .unwrap();
        let ada = table.characters[0].id.clone();
        let bandit = table.characters[1].id.clone();
        table.characters[0].numbers.insert("evasion".into(), 10);
        table.characters[0].numbers.insert("hp".into(), 0);
        table.characters[0].numbers.insert("armorscore".into(), 2);
        table.characters[0].numbers.insert("armor".into(), 0);
        table.characters[0].numbers.insert("major".into(), 7);
        table.characters[0].numbers.insert("severe".into(), 13);
        table.characters[1].numbers.insert("attack".into(), 1);
        table.characters[1].texts.insert("atkdamage".into(), "1d8".into());

        let attack = table.prepare_attack(&gm(), &bandit, &ada, 2, 0).unwrap();
        assert_eq!(attack.formula, "1d20+1+2");
        assert_eq!(attack.label, "Bandit attacks Ada");
        assert_eq!(attack.contest.defense_name, "Evasion");
        assert_eq!(attack.contest.defense, 10);
        assert!(!attack.damage);

        let blow = table.receive_blow(&ada, 12, system).unwrap();
        assert_eq!(blow.step, "Major");
        assert_eq!(blow.marks, 2);
        assert!(table.strike.is_some());
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));

        table
            .command(&gm(), ClientMsg::AnswerStrike { use_armor: true }, Some(system))
            .unwrap();
        assert!(table.strike.is_none());
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(1));
        assert_eq!(table.characters[0].numbers.get("armor").copied(), Some(1));

        table.command(&gm(), ClientMsg::Undo, Some(system)).unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));
        assert_eq!(table.characters[0].numbers.get("armor").copied(), Some(0));
        assert!(table.undo.is_none());

        table.characters[0].numbers.insert("armorscore".into(), 0);
        let bare = table.receive_blow(&ada, 7, system).unwrap();
        assert_eq!(bare.marks, 2);
        assert!(table.strike.is_none());
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(2));
        table.command(&gm(), ClientMsg::Undo, Some(system)).unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));
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
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
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
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
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
    fn deleting_the_open_campaign_drops_its_scenes_and_cast() {
        use crate::net::protocol::ClientMsg;
        let mut table = Table::new();
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Ada".into(), kind: CharacterKind::Pc }, None)
            .unwrap();
        table
            .command(&gm(), ClientMsg::CreateScene { name: "Hall".into(), width: 12, height: 12 }, None)
            .unwrap();
        let open = table.campaign.id.clone();
        table
            .command(&gm(), ClientMsg::CreateCampaign { name: "Next".into(), system_id: None }, None)
            .unwrap();
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Marker".into(),
                    x: 1.0,
                    y: 1.0,
                    owner_id: None,
                    hidden: false,
                    vision_radius: 0.0,
                    grants_vision: false,
                    color: "#d4a656".into(),
                    character_id: None,
                    cast_asset: None,
                    facing: None,
                },
                None,
            )
            .unwrap();
        table.command(&gm(), ClientMsg::SwitchCampaign { campaign_id: open.clone() }, None).unwrap();
        assert_eq!(table.characters.len(), 1);
        assert_eq!(table.scene.as_ref().unwrap().name, "Hall");

        table.command(&gm(), ClientMsg::DeleteCampaign { campaign_id: open.clone() }, None).unwrap();
        assert_eq!(table.campaign.name, "Next");
        assert_eq!(table.tokens.len(), 1);
        assert!(table.characters.is_empty());
        assert!(table.scene.is_none());
        assert!(table.shelved.iter().all(|item| item.campaign.id != open));

        let only = table.campaign.id.clone();
        table.command(&gm(), ClientMsg::DeleteCampaign { campaign_id: only.clone() }, None).unwrap();
        assert_ne!(table.campaign.id, only);
        assert_eq!(table.campaign.name, "Campaign");
        assert!(table.campaign.system_id.is_none());
        assert!(table.shelved.is_empty());
        assert!(table.characters.is_empty());
        assert!(table.scene.is_none());
        assert!(table.tokens.is_empty());
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
            schema_version: 1,
            visual_profile: String::new(),
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
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
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
