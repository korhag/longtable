// ------------------------------------------------------------
// Module: state
// Purpose: The one table this process hosts. Movement, doors,
//          fog, and tokens are changed here, then a filtered
//          snapshot is pushed to every open connection.
// ------------------------------------------------------------

use std::cell::RefCell;
use std::collections::HashMap;

use tokio::sync::mpsc;
use uuid::Uuid;

use std::collections::{BTreeMap, HashSet};

use crate::dice::{parse_formula, RollFormula, KEYFRAME_DT};
use crate::map::compile::Compiled;
use crate::map::{MapLayout, Scene, Segment, SightBlock, Vec2, DIMETRIC_PROFILE};
use crate::net::protocol::{
    Blow, ClientMsg, ConcealedRoll, Contest, DieResult, FogBrushMode, FogCell, RollEvent, RollVisibility, ServerMsg, TalkLine,
};
use crate::systems::{
    apply_modifiers, field_ceiling, fill_formula, threshold_blow, threshold_marks, Character, CharacterKind, FieldDef, FieldKind,
    SystemDef,
    Campaign,
};
use crate::vision::{cells_for, point_in_poly, reveal_height, sees_actor, visibility_polygon, Explored, TALL_WALL};

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
    /// Gameplay size in cells. A picture does not change this.
    #[serde(default = "one_by_one")]
    pub footprint: [u32; 2],
}

fn facing_south() -> String {
    "south".into()
}

fn one_by_one() -> [u32; 2] {
    [1, 1]
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
    #[serde(default)]
    pub damage_roll_id: Option<String>,
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
    pub direct: bool,
    pub adversary: bool,
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

/// Explored terrain belongs to a seat, not to the whole party. Explicit GM
/// brush reveals are shared. Entries are keyed by scene so switching maps or
/// campaigns never exchanges one map's memory with another's.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SceneFog {
    pub revealed: Explored,
    pub seats: HashMap<String, Explored>,
}

impl SceneFog {
    fn new(width: u32, height: u32) -> Self {
        Self { revealed: Explored::new(width, height), seats: HashMap::new() }
    }
}

pub struct Table {
    pub passphrase: String,
    pub gm_key: String,
    pub players: Vec<Player>,
    pub scene: Option<Scene>,
    pub tokens: Vec<Token>,
    pub explored: Explored,
    pub fog_memory: HashMap<String, SceneFog>,
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
    /// Set when a save could not be opened. Writes refuse to replace that file.
    pub save_hold: Option<String>,
    /// What an older save kept. Not written back into the file.
    pub review: Option<String>,
    voices: HashMap<String, String>,
    clients: Vec<ClientConn>,
    sessions: HashMap<String, String>,
    epoch: u64,
    next_color: usize,
    /// Reused sight masks. Not part of the save. A dice roll does not recompute them.
    sight_cache: RefCell<SightCache>,
    /// Cast id to gameplay footprint. Not saved. Filled when the library loads.
    cast_footprints: HashMap<String, [u32; 2]>,
    /// Claims while dice simulate. Persist the consumed flag only with the resulting damage.
    damage_in_flight: HashSet<String>,
}

struct SightCache {
    explored_stamp: Option<u64>,
    masks: HashMap<String, (u64, Explored)>,
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

fn figures_outside(tokens: &[Token], width: f32, height: f32) -> Option<String> {
    let names: Vec<&str> = tokens
        .iter()
        .filter(|token| token.x < 0.5 || token.y < 0.5 || token.x > width - 0.5 || token.y > height - 0.5)
        .map(|token| token.name.as_str())
        .collect();
    occupy_message(&names, "outside the new map", "Move them onto it, then put the map on the table again.")
}

fn figures_inside(tokens: &[Token], blockers: &[Segment]) -> Option<String> {
    let loops = closed_loops(blockers);
    let names: Vec<&str> = tokens
        .iter()
        .filter(|token| loops.iter().any(|polygon| point_in_poly(Vec2::new(token.x, token.y), polygon)))
        .map(|token| token.name.as_str())
        .collect();
    occupy_message(&names, "standing in something on the new map", "Move them clear of it, then put the map on the table again.")
}

fn figures_straddling(tokens: &[Token], layout: &MapLayout) -> Option<String> {
    let names: Vec<&str> = tokens
        .iter()
        .filter(|token| matches!(support_footprint(layout, token.x, token.y, token.footprint), Err(message) if message.contains("one level")))
        .map(|token| token.name.as_str())
        .collect();
    occupy_message(&names, "standing across two levels", "Move them onto one level, then put the map on the table again.")
}

fn support_footprint(layout: &MapLayout, x: f32, y: f32, footprint: [u32; 2]) -> Result<(), String> {
    let mut height = None;
    for (cell_x, cell_y) in occupied_cells(x, y, footprint) {
        if cell_x < 0 || cell_y < 0 || cell_x >= layout.width as i32 || cell_y >= layout.height as i32 {
            return Err("that square is off the map".into());
        }
        let level = layout.level_at(cell_x as f32 + 0.5, cell_y as f32 + 0.5) as u8;
        match height {
            None => height = Some(level),
            Some(previous) if previous != level => return Err("that body does not fit on one level".into()),
            _ => {}
        }
    }
    Ok(())
}

fn occupied_cells(x: f32, y: f32, footprint: [u32; 2]) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    let xs = axis_cells(x, footprint[0]);
    let ys = axis_cells(y, footprint[1]);
    for cell_y in ys {
        for cell_x in xs.clone() {
            cells.push((cell_x, cell_y));
        }
    }
    cells
}

fn axis_cells(center: f32, span: u32) -> Vec<i32> {
    let span = span.max(1);
    let start = (center - span as f32 / 2.0).floor() as i32;
    (0..span as i32).map(|step| start + step).collect()
}

fn snap_footprint(x: f32, y: f32, footprint: [u32; 2], width: f32, height: f32) -> Option<(f32, f32)> {
    Some((snap_axis(x, footprint[0], width)?, snap_axis(y, footprint[1], height)?))
}

fn snap_axis(value: f32, span: u32, limit: f32) -> Option<f32> {
    let span = span.max(1);
    if span % 2 == 1 {
        return snap_cell(value, limit);
    }
    if !value.is_finite() {
        return None;
    }
    let center = value.round();
    let half = span as f32 / 2.0;
    if center < half || center + half > limit + 0.001 {
        return None;
    }
    Some(center)
}

fn occupy_message(names: &[&str], where_they_are: &str, repair: &str) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let list = match names {
        [one] => (*one).to_string(),
        [first, second] => format!("{first} and {second}"),
        many => {
            let last = many.last().copied().unwrap_or("");
            let rest = many[..many.len() - 1].join(", ");
            format!("{rest}, and {last}")
        }
    };
    let verb = if names.len() == 1 { "is" } else { "are" };
    Some(format!("{list} {verb} {where_they_are}. {repair}"))
}

/// Object footprints are stored as consecutive closed rings of edges.
fn closed_loops(segments: &[Segment]) -> Vec<Vec<Vec2>> {
    let mut loops = Vec::new();
    let mut current: Vec<Vec2> = Vec::new();
    for segment in segments {
        if current.last().is_some_and(|last| !same_point(last, &segment.a)) {
            current.clear();
        }
        if current.is_empty() {
            current.push(segment.a);
        }
        current.push(segment.b);
        if current.len() >= 4 && same_point(current.last().unwrap(), &current[0]) {
            current.pop();
            loops.push(std::mem::take(&mut current));
        }
    }
    loops
}

fn same_point(a: &Vec2, b: &Vec2) -> bool {
    (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001
}

fn grants_for(token: &Token, viewer_id: &str) -> bool {
    token.grants_vision && !token.hidden && token.owner_id.as_deref() == Some(viewer_id)
}

fn same_edge(span: &SightBlock, wall: &Segment) -> bool {
    let close = |left: Vec2, right: Vec2| (left.x - right.x).abs() < 0.02 && (left.y - right.y).abs() < 0.02;
    (close(span.a, wall.a) && close(span.b, wall.b)) || (close(span.a, wall.b) && close(span.b, wall.a))
}

fn edge_base(layout: &MapLayout, a: Vec2, b: Vec2) -> f32 {
    let mid_x = (a.x + b.x) * 0.5;
    let mid_y = (a.y + b.y) * 0.5;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = (dx * dx + dy * dy).sqrt().max(0.001);
    let nx = -dy / length * 0.05;
    let ny = dx / length * 0.05;
    layout.level_at(mid_x + nx, mid_y + ny).max(layout.level_at(mid_x - nx, mid_y - ny))
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
            fog_memory: HashMap::new(),
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
            save_hold: None,
            review: None,
            voices: HashMap::new(),
            clients: Vec::new(),
            sessions: HashMap::new(),
            epoch: 0,
            next_color: 0,
            sight_cache: RefCell::new(SightCache { explored_stamp: None, masks: HashMap::new() }),
            cast_footprints: HashMap::new(),
            damage_in_flight: HashSet::new(),
        }
    }

    /// Remember each cast picture's gameplay size. The picture itself stays a picture.
    pub fn note_cast_footprints(&mut self, catalog: &crate::assets::Catalog) {
        self.cast_footprints.clear();
        for pack in &catalog.packs {
            for actor in &pack.cast {
                let width = actor.suggested_footprint_cells[0].clamp(1, 8);
                let depth = actor.suggested_footprint_cells[1].clamp(1, 8);
                self.cast_footprints.insert(format!("{}/{}", pack.id, actor.id), [width, depth]);
            }
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
            self.sight_cache.borrow_mut().explored_stamp = None;
            self.sight_cache.borrow_mut().masks.clear();
        }
        self.scene_fog_mut();
    }

    fn fog_key(&self) -> String {
        self.scene.as_ref().map(|scene| scene.id.clone())
            .unwrap_or_else(|| format!("unmapped:{}", self.campaign.id))
    }

    fn scene_fog_mut(&mut self) -> &mut SceneFog {
        let key = self.fog_key();
        let (width, height) = (self.explored.width, self.explored.height);
        let mut reset = !self.fog_memory.contains_key(&key);
        let memory = self.fog_memory.entry(key).or_insert_with(|| SceneFog::new(width, height));
        if memory.revealed.width != width || memory.revealed.height != height {
            *memory = SceneFog::new(width, height);
            reset = true;
        }
        if reset {
            self.sight_cache.borrow_mut().explored_stamp = None;
        }
        memory
    }

    pub(crate) fn explored_for(&self, viewer_id: &str) -> Explored {
        let mut explored = Explored::new(self.explored.width, self.explored.height);
        if let Some(memory) = self.fog_memory.get(&self.fog_key()) {
            explored.merge(&memory.revealed);
            if let Some(personal) = memory.seats.get(viewer_id) {
                explored.merge(personal);
            }
        }
        explored
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
            ClientMsg::TeleportToken { token_id, x, y } => {
                self.require_gm(viewer)?;
                if self.relocate_token(viewer, &token_id, x, y, true)? { Ok(CommandEffect::Refresh) } else { Ok(CommandEffect::Quiet) }
            }
            ClientMsg::Roll { .. } | ClientMsg::Attack { .. } | ClientMsg::Strike { .. } => {
                Err("rolls are started by the socket, not the table".into())
            }
            ClientMsg::SetDoor { door_id, open } => {
                self.set_door(viewer, &door_id, open)?;
                Ok(CommandEffect::Refresh)
            }
            ClientMsg::SetEnvironment { environment } => {
                self.require_gm(viewer)?;
                environment.validate()?;
                let scene = self.scene.as_mut().ok_or_else(|| "there is no map".to_string())?;
                scene.environment = environment.clone();
                if let Some(layout) = scene.layout.as_mut() { layout.environment = Some(environment); }
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
            ClientMsg::DuplicateScene { scene_id } => {
                self.duplicate_scene(viewer, &scene_id)?;
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
        if let Some(contest) = roll.contest.as_ref().filter(|contest| contest.blow.is_some()) {
            if let Some(id) = &contest.attack_roll_id {
                self.damage_in_flight.remove(id);
                if let Some(attack) = self.rolls.iter_mut().find(|roll| &roll.id == id).and_then(|roll| roll.contest.as_mut()) {
                    attack.damage_rolled = true;
                }
            }
        }
        if let (Some(pending), Some(contest)) = (&mut self.strike, &roll.contest) {
            if pending.damage_roll_id.is_none() && pending.target_id == contest.target_id
                && contest.blow.as_ref().is_some_and(|blow| blow.pending_armor) {
                pending.damage_roll_id = Some(roll.id.clone());
            }
        }
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
        self.relocate_token(viewer, token_id, x, y, false)
    }

    fn relocate_token(&mut self, viewer: &Viewer, token_id: &str, x: f32, y: f32, teleport: bool) -> Result<bool, String> {
        if !x.is_finite() || !y.is_finite() { return Err("choose a square on the map".into()); }
        let (width, height) = self.bounds();
        let index = self.token_index(token_id).ok_or_else(|| "that token is not on the table".to_string())?;
        let footprint = self.tokens[index].footprint;
        let (x, y) = snap_footprint(x, y, footprint, width, height).ok_or_else(|| "that square is off the map".to_string())?;
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
        if let Some(layout) = self.scene.as_ref().and_then(|scene| scene.layout.as_ref()) {
            support_footprint(layout, x, y, footprint)?;
        }
        let mut last_step = (Vec2::new(token.x, token.y), Vec2::new(x, y));
        if !teleport {
            let from = Vec2::new(token.x, token.y);
            let to = Vec2::new(x, y);
            let walls = self.blocking_walls();
            let blockers = self.scene.as_ref().map(|scene| scene.move_blockers.as_slice()).unwrap_or_default();
            let flat;
            let layout = if let Some(layout) = self.scene.as_ref().and_then(|scene| scene.layout.as_ref()) { layout } else {
                if width.ceil() * height.ceil() > 250_000.0 { return Err("this map is too large to find a walking route".into()); }
                flat = crate::map::layout::MapLayout::blank(width.ceil() as u32, height.ceil() as u32, "");
                &flat
            };
            let route = crate::map::navigation::find_route(layout, from, to, &walls, blockers, footprint)?;
            if route.len() >= 2 {
                let before = route[route.len()-2]; let after = route[route.len()-1];
                last_step = (Vec2::new(before.0 as f32, before.1 as f32), Vec2::new(after.0 as f32, after.1 as f32));
            }
        }
        let token = &mut self.tokens[index];
        let facing = cardinal_facing(last_step.0.x, last_step.0.y, last_step.1.x, last_step.1.y, &token.facing);
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

    fn set_lighting(&mut self, viewer: &Viewer, _lighting: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.scene.is_none() {
            return Err("there is no map".into());
        }
        Err("the table uses one fixed view".into())
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
        let memory = self.scene_fog_mut();
        for cell in cells {
            memory.revealed.set(cell.x, cell.y, reveal);
            if !reveal {
                for explored in memory.seats.values_mut() {
                    explored.set(cell.x, cell.y, false);
                }
            }
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
        let resolved = cast_asset.clone().filter(|asset| !asset.is_empty()).or_else(|| {
            character_id.as_ref().and_then(|id| {
                self.characters.iter().find(|character| character.id == *id).and_then(|character| character.cast_asset_id.clone())
            })
        });
        let footprint = resolved.as_ref().and_then(|asset| self.cast_footprints.get(asset).copied()).unwrap_or([1, 1]);
        let (x, y) = snap_footprint(x, y, footprint, width, height).unwrap_or((1.5, 1.5));
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
            footprint,
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
        let facing_only = facing.is_some()
            && name.is_none()
            && owner_id.is_none()
            && !clear_owner
            && hidden.is_none()
            && vision_radius.is_none()
            && grants_vision.is_none()
            && color.is_none()
            && character_id.is_none()
            && !clear_character
            && !set_cast_asset;
        let owns = self.tokens.iter().any(|token| token.id == token_id && token.owner_id.as_deref() == Some(viewer.player_id.as_str()));
        let own_turn = facing_only && owns && viewer.role != crate::net::protocol::Role::Gm;
        if !own_turn {
            self.require_gm(viewer)?;
        }
        let owner_id = self.known_owner(owner_id)?;
        let owner_changed = clear_owner || owner_id.is_some();
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
            // A new picture does not change how many squares the body occupies.
            token.cast_asset = cast_asset.filter(|asset| !asset.is_empty());
        }
        if let Some(facing) = facing {
            token.facing = cardinal_facing(0.0, 0.0, 0.0, 0.0, &facing);
        }
        let linked_owner = if owner_changed {
            token.character_id.clone().map(|id| (id, token.owner_id.clone()))
        } else { None };
        if let Some((character_id, owner_id)) = linked_owner {
            self.assign_character_owner(&character_id, owner_id);
        }
        if !own_turn {
            self.reveal_from_tokens();
        }
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
            environment: Default::default(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
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
        let width = layout.width as f32;
        let height = layout.height as f32;
        let updating = self.scene.as_ref().is_some_and(|scene| scene.id == id);
        if updating || self.scene.is_none() {
            if let Some(message) = figures_outside(&self.tokens, width, height) {
                return Err(message);
            }
            if let Some(message) = figures_inside(&self.tokens, &compiled.move_blockers) {
                return Err(message);
            }
            if let Some(message) = figures_straddling(&self.tokens, &layout) {
                return Err(message);
            }
        }
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
        if !updating && self.scene.is_some() {
            self.pack_live_scene();
        }
        self.install_scene(Scene {
            id: id.clone(),
            name,
            width,
            height,
            image_file: None,
            walls: compiled.walls,
            doors,
            lights: compiled.lights,
            environment: layout.environment.clone().unwrap_or_default(),
            layout: Some(layout),
            move_blockers: compiled.move_blockers,
            sight: compiled.sight,
            presentation: None,
            schema_version: 2,
            visual_profile: DIMETRIC_PROFILE.to_string(),
        });
        if let Some(memory) = memory {
            self.explored.merge(&memory);
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
        self.sight_cache.borrow_mut().explored_stamp = None;
        self.sight_cache.borrow_mut().masks.clear();
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
        self.sync_fog_size();
        let stamp = self.sight_inputs();
        if self.sight_cache.borrow().explored_stamp == Some(stamp) {
            return;
        }
        let layout = self.uses_height_sight().then(|| self.sight_layout());
        let spans = if layout.is_some() { self.sight_spans() } else { Vec::new() };
        let walls = if layout.is_none() { self.blocking_walls() } else { Vec::new() };
        let (width, height) = (self.explored.width, self.explored.height);
        let mut masks: HashMap<String, Explored> = HashMap::new();
        for eye in self.tokens.iter().filter(|token| token.grants_vision && !token.hidden) {
            let mut visible = Explored::new(width, height);
            if let Some(layout) = &layout {
                reveal_height(&mut self.explored, Some(&mut visible), eye.x, eye.y, eye.vision_radius, layout, &spans);
            } else {
                visible.paint_polygon(&visibility_polygon(Vec2::new(eye.x, eye.y), eye.vision_radius, &walls));
                self.explored.merge(&visible);
            }
            if let Some(owner) = &eye.owner_id {
                masks.entry(owner.clone()).or_insert_with(|| Explored::new(width, height)).merge(&visible);
            }
        }
        let memory = self.scene_fog_mut();
        for (owner, visible) in &masks {
            memory.seats.entry(owner.clone()).or_insert_with(|| Explored::new(width, height)).merge(visible);
        }
        // The same current masks serve snapshots, so a move does one sight pass
        // per eye, rather than recomputing the ray field for every connection.
        let mut cache = self.sight_cache.borrow_mut();
        cache.masks = masks.into_iter().map(|(owner, mask)| (owner, (stamp, mask))).collect();
        cache.explored_stamp = Some(stamp);
    }

    pub(crate) fn uses_height_sight(&self) -> bool {
        let Some(scene) = &self.scene else { return false };
        if !scene.sight.is_empty() {
            return true;
        }
        scene.layout.as_ref().is_some_and(|layout| layout.has_raised_ground())
    }

    pub(crate) fn viewer_sees(&self, viewer_id: &str, token: &Token) -> bool {
        let layout = self.sight_layout();
        let spans = self.sight_spans();
        self.tokens.iter().filter(|eye| grants_for(eye, viewer_id)).any(|eye| {
            sees_actor(eye.x, eye.y, eye.vision_radius, token.x, token.y, &layout, &spans)
        })
    }

    pub(crate) fn visible_bits(&self, viewer_id: &str) -> Option<String> {
        if !self.uses_height_sight() {
            return None;
        }
        let stamp = self.sight_inputs();
        if let Some(bits) = self
            .sight_cache
            .borrow()
            .masks
            .get(viewer_id)
            .filter(|(cached, _)| *cached == stamp)
            .map(|(_, mask)| mask.to_base64())
        {
            return Some(bits);
        }
        let layout = self.sight_layout();
        let spans = self.sight_spans();
        let mut mask = Explored::new(self.explored.width, self.explored.height);
        let eyes: Vec<(f32, f32, f32)> = self
            .tokens
            .iter()
            .filter(|token| grants_for(token, viewer_id))
            .map(|token| (token.x, token.y, token.vision_radius))
            .collect();
        let mut ignored = Explored::new(self.explored.width, self.explored.height);
        for (x, y, radius) in eyes {
            reveal_height(&mut ignored, Some(&mut mask), x, y, radius, &layout, &spans);
        }
        let bits = mask.to_base64();
        self.sight_cache.borrow_mut().masks.insert(viewer_id.to_string(), (stamp, mask));
        Some(bits)
    }

    fn sight_layout(&self) -> MapLayout {
        if let Some(layout) = self.scene.as_ref().and_then(|scene| scene.layout.clone()) {
            return layout;
        }
        let (width, height) = self.bounds();
        MapLayout::blank(width.max(5.0) as u32, height.max(5.0) as u32, "core/grass")
    }

    fn sight_spans(&self) -> Vec<SightBlock> {
        let Some(scene) = &self.scene else { return Vec::new() };
        let mut spans = scene.sight.clone();
        for wall in self.blocking_walls() {
            if spans.iter().any(|span| same_edge(span, &wall)) {
                continue;
            }
            let base = scene.layout.as_ref().map(|layout| edge_base(layout, wall.a, wall.b)).unwrap_or(0.0);
            spans.push(SightBlock { a: wall.a, b: wall.b, base, top: base + TALL_WALL });
        }
        spans
    }

    fn sight_inputs(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        let mix = |hash: &mut u64, value: u64| {
            *hash ^= value;
            *hash = hash.wrapping_mul(0x100000001b3);
        };
        let mix_f = |hash: &mut u64, value: f32| mix(hash, value.to_bits() as u64);
        let mix_str = |hash: &mut u64, value: &str| {
            mix(hash, value.len() as u64);
            for byte in value.bytes() {
                mix(hash, byte as u64);
            }
        };
        mix_str(&mut hash, &self.campaign.id);
        if let Some(scene) = &self.scene {
            mix_str(&mut hash, &scene.id);
            mix_f(&mut hash, scene.width);
            mix_f(&mut hash, scene.height);
            for wall in &scene.walls {
                mix_f(&mut hash, wall.a.x);
                mix_f(&mut hash, wall.a.y);
                mix_f(&mut hash, wall.b.x);
                mix_f(&mut hash, wall.b.y);
            }
            for door in &scene.doors {
                mix(&mut hash, if door.open { 1 } else { 2 });
                mix_f(&mut hash, door.a.x);
                mix_f(&mut hash, door.a.y);
                mix_f(&mut hash, door.b.x);
                mix_f(&mut hash, door.b.y);
            }
            for block in &scene.sight {
                mix_f(&mut hash, block.base);
                mix_f(&mut hash, block.top);
                mix_f(&mut hash, block.a.x);
                mix_f(&mut hash, block.a.y);
                mix_f(&mut hash, block.b.x);
                mix_f(&mut hash, block.b.y);
            }
            if let Some(levels) = scene.layout.as_ref().and_then(|layout| layout.levels.as_deref()) {
                for byte in levels.bytes() {
                    mix(&mut hash, byte as u64);
                }
            }
        }
        for token in &self.tokens {
            mix_str(&mut hash, &token.id);
            mix_str(&mut hash, token.owner_id.as_deref().unwrap_or(""));
            mix_f(&mut hash, token.x);
            mix_f(&mut hash, token.y);
            mix_f(&mut hash, token.vision_radius);
            mix(&mut hash, u64::from(token.grants_vision));
            mix(&mut hash, u64::from(token.hidden));
        }
        hash
    }

    pub fn vision_polygons(&self, player_id: &str) -> Vec<Vec<Vec2>> {
        let walls = self.blocking_walls();
        self.tokens
            .iter()
            .filter(|token| grants_for(token, player_id))
            .map(|token| visibility_polygon(Vec2::new(token.x, token.y), token.vision_radius, &walls))
            .collect()
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
            self.fog_memory.remove(&format!("unmapped:{campaign_id}"));
            if let Some(scene) = &self.scene {
                self.fog_memory.remove(&scene.id);
            }
            for scene in &self.scenes {
                self.fog_memory.remove(&scene.id);
            }
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
        let index = self.shelved.iter().position(|item| item.campaign.id == campaign_id)
            .ok_or_else(|| "that campaign is not on this table".to_string())?;
        let removed = self.shelved.remove(index);
        self.fog_memory.remove(&format!("unmapped:{campaign_id}"));
        if let Some(scene) = &removed.scene {
            self.fog_memory.remove(&scene.id);
        }
        for scene in &removed.scenes {
            self.fog_memory.remove(&scene.id);
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
            environment: Default::default(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
            presentation: None,
            schema_version: 1,
            visual_profile: DIMETRIC_PROFILE.to_string(),
        });
        Ok(())
    }

    fn duplicate_scene(&mut self, viewer: &Viewer, scene_id: &str) -> Result<(), String> {
        self.require_gm(viewer)?;
        if self.scenes.len() >= 40 {
            return Err("this campaign has enough scenes".into());
        }
        let source = if let Some(scene) = self.scene.as_ref().filter(|scene| scene.id == scene_id) {
            scene.clone()
        } else {
            self.scenes
                .iter()
                .find(|shelf| shelf.id == scene_id)
                .and_then(|shelf| shelf.scene.clone())
                .ok_or_else(|| "that scene is not in this campaign".to_string())?
        };
        let mut copy = source;
        copy.id = Uuid::new_v4().to_string();
        let stem: String = copy.name.chars().take(42).collect();
        copy.name = title(&format!("{stem} copy"))?;
        let (cells_w, cells_h) = cells_for(copy.width, copy.height);
        let shelf = SceneShelf {
            id: copy.id.clone(),
            name: copy.name.clone(),
            scene: Some(copy),
            tokens: Vec::new(),
            explored: Explored::new(cells_w, cells_h),
        };
        self.scenes.push(shelf);
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
            self.fog_memory.remove(scene_id);
            if self.scenes.is_empty() {
                self.scene = None;
                self.tokens.clear();
                let (width, height) = cells_for(DEFAULT_WIDTH, DEFAULT_HEIGHT);
                self.explored = Explored::new(width, height);
                self.fog_memory.remove(&self.fog_key());
                self.sight_cache.borrow_mut().explored_stamp = None;
                self.sight_cache.borrow_mut().masks.clear();
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
        self.fog_memory.remove(scene_id);
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
        let owner_changed = gm && (clear_owner || owner_id.is_some());
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
        if owner_changed {
            self.assign_character_owner(character_id, if clear_owner { None } else { owner_id });
            self.reveal_from_tokens();
        }
        Ok(())
    }

    /// One assignment grants the sheet, game actions and sight from every
    /// linked figure in this campaign. Exploration remains with its old seat.
    fn assign_character_owner(&mut self, character_id: &str, owner_id: Option<String>) {
        if let Some(character) = self.characters.iter_mut().find(|character| character.id == character_id) {
            character.owner_id = owner_id.clone();
        }
        for token in self.tokens.iter_mut().chain(self.scenes.iter_mut().flat_map(|scene| scene.tokens.iter_mut())) {
            if token.character_id.as_deref() == Some(character_id) {
                token.owner_id = owner_id.clone();
            }
        }
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
        attack_roll_id: &str,
        attacker_id: &str,
        target_id: &str,
        plus: i32,
        minus: i32,
    ) -> Result<AttackPlan, String> {
        if self.strike.is_some() {
            return Err("answer the last blow first".into());
        }
        let mut plan = self.engagement(viewer, attacker_id, target_id, plus, minus, true)?;
        let attack = self.rolls.iter().find(|roll| roll.id == attack_roll_id)
            .ok_or_else(|| "that attack is no longer in the roll history".to_string())?;
        let contest = attack.contest.as_ref()
            .ok_or_else(|| "roll an attack before rolling damage".to_string())?;
        if contest.verdict.as_deref() != Some("hit") || contest.blow.is_some()
            || contest.attacker_id != attacker_id || contest.target_id != target_id {
            return Err("damage requires a hit against this target".into());
        }
        if contest.damage_rolled || self.damage_in_flight.contains(attack_roll_id) {
            return Err("damage has already been rolled for this attack".into());
        }
        if attack.outcome.as_ref().is_some_and(|outcome| outcome.tone == "critical") {
            plan.formula = crate::systems::daggerheart::critical_damage_formula(&plan.formula)?;
        }
        plan.contest.attack_roll_id = Some(attack_roll_id.into());
        Ok(plan)
    }

    /// Reserve the hit while dice simulate outside the table lock. Replayed clicks cannot reuse it.
    pub fn claim_damage(&mut self, viewer: &Viewer, plan: &AttackPlan) -> Result<(), String> {
        let id = plan.contest.attack_roll_id.as_deref()
            .ok_or_else(|| "damage requires an attack roll".to_string())?;
        self.prepare_strike(viewer, id, &plan.contest.attacker_id, &plan.contest.target_id, 0, 0)?;
        self.damage_in_flight.insert(id.into());
        Ok(())
    }

    pub fn damage_in_flight(&self, attack_roll_id: &str) -> bool {
        self.damage_in_flight.contains(attack_roll_id)
    }

    /// Failed dice jobs release their reservation so the explicit button can be retried.
    pub fn cancel_damage(&mut self, plan: &AttackPlan) {
        if let Some(id) = plan.contest.attack_roll_id.as_deref() {
            self.damage_in_flight.remove(id);
        }
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
        let formula = if filled.starts_with("flat:") {
            let parsed = parse_formula(&filled).map_err(|error| error.to_string())?;
            format!("flat:{}", parsed.modifier + plus.clamp(0, 1000) - minus.clamp(0, 1000))
        } else {
            apply_modifiers(&filled, plus, minus)
        };
        Ok(AttackPlan {
            formula,
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
                attack_roll_id: None,
                damage_rolled: false,
            },
            damage,
            direct: damage && crate::systems::daggerheart::deals_direct_damage(attacker),
            adversary: attacker.kind == CharacterKind::Adversary,
        })
    }

    /// Compare a damage total with the target's thresholds. Armor, when a slot is free, waits for an answer.
    pub fn receive_blow(&mut self, target_id: &str, amount: i32, direct: bool, system: &SystemDef) -> Result<Blow, String> {
        if self.strike.is_some() {
            return Err("answer the last blow first".into());
        }
        let field = system
            .fields
            .iter()
            .find(|field| field.kind == FieldKind::Thresholds)
            .ok_or_else(|| "this system has no damage thresholds".to_string())?;
        let (step, marks, open, hp_before, major_threshold, severe_threshold) = {
            let character = self
                .characters
                .iter()
                .find(|character| character.id == target_id)
                .ok_or_else(|| "that target is not at the table".to_string())?;
            let (step, marks) = if amount <= 0 { ("None".into(), 0) } else {
                threshold_blow(field, amount, &character.numbers).unwrap_or_else(|| ("None".into(), 0))
            };
            let open = if direct || character.kind == CharacterKind::Adversary { 0 } else { armor_open(character) };
            let pool_id = field.pool.as_deref().unwrap_or("hp");
            let threshold = |id: &str| field.steps.iter().find(|step| step.id == id)
                .map(|step| character.numbers.get(id).copied().unwrap_or(step.default));
            (step, marks, open, character.numbers.get(pool_id).copied().unwrap_or(0), threshold("major"), threshold("severe"))
        };
        let mut blow = Blow {
            step: step.clone(), marks, applied_marks: 0, pending_armor: open > 0 && marks > 0, major_threshold, severe_threshold,
        };
        if marks <= 0 {
            return Ok(blow);
        }
        if open > 0 {
            self.strike = Some(PendingStrike { target_id: target_id.to_string(), step, marks, damage_roll_id: None });
            return Ok(blow);
        }
        if let Some(slot) = self.add_pool(target_id, field, marks, system)? {
            self.remember(vec![slot]);
        }
        let pool_id = field.pool.as_deref().unwrap_or("hp");
        let hp_after = self.characters.iter().find(|character| character.id == target_id)
            .and_then(|character| character.numbers.get(pool_id)).copied().unwrap_or(hp_before);
        blow.applied_marks = (hp_after - hp_before).max(0);
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
        if let Some(id) = pending.damage_roll_id.as_deref() {
            let hp_after = self.characters.iter().find(|character| character.id == pending.target_id)
                .and_then(|character| character.numbers.get("hp")).copied().unwrap_or(hp_before);
            if let Some(blow) = self.rolls.iter_mut().find(|roll| roll.id == id)
                .and_then(|roll| roll.contest.as_mut()).and_then(|contest| contest.blow.as_mut()) {
                blow.pending_armor = false;
                blow.applied_marks = (hp_after - hp_before).max(0);
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

    fn empty_compiled() -> Compiled {
        Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new(), sight: Vec::new() }
    }

    fn named_token(name: &str, x: f32, y: f32) -> Token {
        Token {
            id: name.to_string(),
            name: name.to_string(),
            x,
            y,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: None,
            hidden: false,
            vision_radius: 8.0,
            grants_vision: true,
            character_id: None,
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
            footprint: [1, 1],
        }
    }

    #[test]
    fn duplicating_a_scene_leaves_the_figures_where_they_are() {
        let mut table = Table::new();
        table.scene = Some(Scene {
            id: "hall".into(),
            name: "Hall".into(),
            width: 12.0,
            height: 12.0,
            image_file: None,
            walls: Vec::new(),
            doors: Vec::new(),
            lights: Vec::new(),
            environment: Default::default(),
            layout: Some(crate::map::layout::MapLayout::blank(12, 12, "core/grass")),
            move_blockers: Vec::new(),
            sight: Vec::new(),
            presentation: None,
            schema_version: 2,
            visual_profile: crate::map::DIMETRIC_PROFILE.to_string(),
        });
        table.tokens.push(named_token("Ada", 4.5, 4.5));
        table
            .command(&gm(), crate::net::protocol::ClientMsg::DuplicateScene { scene_id: "hall".into() }, None)
            .unwrap();
        assert_eq!(table.scene.as_ref().unwrap().id, "hall");
        assert_eq!(table.tokens[0].name, "Ada");
        assert_eq!(table.scenes.len(), 1);
        assert!(table.scenes[0].tokens.is_empty());
        assert_ne!(table.scenes[0].id, "hall");
        assert_eq!(table.scenes[0].scene.as_ref().unwrap().layout.as_ref().unwrap().width, 12);
    }

    #[test]
    fn turning_a_figure_does_not_move_them() {
        let mut table = Table::new();
        table.tokens.push(named_token("Ada", 4.5, 6.5));
        table.command(&gm(), crate::net::protocol::ClientMsg::UpdateToken {
            token_id: "Ada".into(),
            name: None,
            owner_id: None,
            clear_owner: false,
            hidden: None,
            vision_radius: None,
            grants_vision: None,
            color: None,
            character_id: None,
            clear_character: false,
            set_cast_asset: false,
            cast_asset: None,
            facing: Some("east".into()),
        }, None).unwrap();
        assert_eq!(table.tokens[0].facing, "east");
        assert_eq!(table.tokens[0].x, 4.5);
        assert_eq!(table.tokens[0].y, 6.5);
    }

    #[test]
    fn a_player_can_turn_their_own_figure() {
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        let mut mine = named_token("Ada", 4.5, 6.5);
        mine.owner_id = Some("p".into());
        table.tokens.push(mine);
        table.tokens.push(named_token("Bram", 2.5, 2.5));
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let turn = |token_id: &str, facing: Option<String>, name: Option<String>| crate::net::protocol::ClientMsg::UpdateToken {
            token_id: token_id.into(),
            name,
            owner_id: None,
            clear_owner: false,
            hidden: None,
            vision_radius: None,
            grants_vision: None,
            color: None,
            character_id: None,
            clear_character: false,
            set_cast_asset: false,
            cast_asset: None,
            facing,
        };
        table.command(&player, turn("Ada", Some("west".into()), None), None).unwrap();
        assert_eq!(table.tokens[0].facing, "west");
        assert_eq!(table.tokens[0].x, 4.5);
        let refused = table.command(&player, turn("Bram", Some("north".into()), None), None);
        assert!(refused.is_err(), "someone else's figure should stay put");
        assert_eq!(table.tokens[1].facing, "south");
        let renamed = table.command(&player, turn("Ada", Some("north".into()), Some("Hero".into())), None);
        assert!(renamed.is_err(), "a player should not rename a figure from the turn");
        assert_eq!(table.tokens[0].name, "Ada");
        assert_eq!(table.tokens[0].facing, "west");
    }

    #[test]
    fn assigning_a_figure_grants_its_sheet_actions_and_personal_sight_across_scenes() {
        use crate::session::permissions::snapshot;
        let catalog = crate::systems::load_systems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../systems").as_path());
        let system = catalog.get("daggerheart").unwrap();
        let mut table = Table::new();
        for id in ["p1", "p2"] {
            table.players.push(Player { id: id.into(), name: id.into(), role: Role::Player, color: "#ffffff".into(), connected: true });
        }
        let p1 = Viewer { player_id: "p1".into(), role: Role::Player, view_as: None, session_token: "p1".into() };
        let p2 = Viewer { player_id: "p2".into(), role: Role::Player, view_as: None, session_token: "p2".into() };
        table.command(&gm(), ClientMsg::CreateScene { name: "Split room".into(), width: 12, height: 12 }, None).unwrap();
        table.scene.as_mut().unwrap().walls.push(Segment { a: Vec2::new(6.0, 0.0), b: Vec2::new(6.0, 12.0) });
        table.command(&gm(), ClientMsg::CreateCharacter { name: "Adventurer".into(), kind: CharacterKind::Pc }, Some(system)).unwrap();
        table.command(&gm(), ClientMsg::CreateCharacter { name: "Bandit".into(), kind: CharacterKind::Adversary }, Some(system)).unwrap();
        let hero = table.characters[0].id.clone();
        let target = table.characters[1].id.clone();
        for (name, x, owner_id, character_id) in [
            ("Hero", 2.5, None, Some(hero.clone())),
            ("Hero companion", 3.5, None, Some(hero.clone())),
            ("Independent scout", 9.5, Some("p2".into()), None),
        ] {
            table.command(&gm(), ClientMsg::CreateToken {
                name: name.into(), x, y: 5.5, owner_id, hidden: false, vision_radius: 12.0, grants_vision: true,
                color: "#ffffff".into(), character_id, cast_asset: None, facing: None,
            }, Some(system)).unwrap();
        }
        let figure = table.tokens[0].id.clone();
        let mut shelved = table.tokens[0].clone();
        shelved.id = "shelved-hero".into();
        table.scenes.push(SceneShelf { id: "other-scene".into(), name: "Other scene".into(), scene: None,
            tokens: vec![shelved], explored: table.explored.clone() });
        let assign = |owner: Option<&str>| ClientMsg::UpdateToken {
            token_id: figure.clone(), name: None, owner_id: owner.map(str::to_string), clear_owner: owner.is_none(),
            hidden: None, vision_radius: None, grants_vision: None, color: None, character_id: None,
            clear_character: false, set_cast_asset: false, cast_asset: None, facing: None,
        };
        assert!(table.command(&p1, assign(Some("p1")), Some(system)).is_err(), "a player cannot claim an unassigned figure");
        table.command(&gm(), assign(Some("p1")), Some(system)).unwrap();
        assert_eq!(table.characters[0].owner_id.as_deref(), Some("p1"));
        assert_eq!(table.tokens[1].owner_id.as_deref(), Some("p1"));
        assert_eq!(table.scenes[0].tokens[0].owner_id.as_deref(), Some("p1"));
        assert_eq!(table.tokens[2].owner_id.as_deref(), Some("p2"), "unlinked figures stay independent");
        let left = snapshot(&table, &p1);
        let right = snapshot(&table, &p2);
        assert_eq!(left.characters.len(), 1, "assignment exposes the playable sheet to its owner");
        assert_eq!(left.characters[0].id, hero);
        assert_eq!(left.tokens.len(), 2, "the wall hides the other seat's figure");
        assert_eq!(right.tokens.len(), 1);
        assert!(right.characters.is_empty());
        assert!(table.prepare_attack(&p1, &hero, &target, 0, 0).is_ok(), "the assigned player can use attacks");
        assert!(table.prepare_attack(&p2, &hero, &target, 0, 0).is_err());
        let edit_owner = |owner: Option<&str>| ClientMsg::UpdateCharacter {
            character_id: hero.clone(), name: None, kind: None, owner_id: owner.map(str::to_string), clear_owner: owner.is_none(),
            numbers: BTreeMap::new(), texts: BTreeMap::new(), set_cast_asset: false, cast_asset_id: None,
        };
        table.command(&p1, edit_owner(Some("p2")), Some(system)).unwrap();
        assert_eq!(table.characters[0].owner_id.as_deref(), Some("p1"), "player sheet edits cannot transfer ownership");
        table.command(&gm(), edit_owner(Some("p2")), Some(system)).unwrap();
        assert_eq!(table.tokens[0].owner_id.as_deref(), Some("p2"));
        assert_eq!(table.tokens[1].owner_id.as_deref(), Some("p2"));
        assert_eq!(table.scenes[0].tokens[0].owner_id.as_deref(), Some("p2"));
        assert!(snapshot(&table, &p1).tokens.is_empty(), "reassignment removes old current sight immediately");
        assert!(table.explored_for("p1").get(10, 22), "old exploration remains with the old seat");
        assert!(table.explored_for("p2").get(10, 22), "the new seat receives current sight immediately");
        assert!(table.prepare_attack(&p1, &hero, &target, 0, 0).is_err());
        assert!(table.prepare_attack(&p2, &hero, &target, 0, 0).is_ok());
        table.command(&gm(), edit_owner(None), Some(system)).unwrap();
        assert!(table.characters[0].owner_id.is_none());
        assert!(table.tokens[..2].iter().all(|token| token.owner_id.is_none()));
        assert!(table.scenes[0].tokens[0].owner_id.is_none());
        assert_eq!(snapshot(&table, &p2).tokens.len(), 1);
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
            footprint: [1, 1],
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
            environment: Default::default(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
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
            footprint: [1, 1],
            });
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let blocked = table.move_token(&player, "ada", 1.5, 6.5);
        assert!(blocked.is_err());
        assert!(table.move_token(&gm(), "ada", 1.5, 6.5).is_err(), "GM walking respects walls too");
        assert!(table.command(&player, ClientMsg::TeleportToken { token_id: "ada".into(), x: 1.5, y: 6.5 }, None).is_err());
        table.command(&gm(), ClientMsg::TeleportToken { token_id: "ada".into(), x: 1.5, y: 6.5 }, None).unwrap();
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
            environment: Default::default(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
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
            footprint: [1, 1],
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
            environment: Default::default(),
            layout: None,
            move_blockers: vec![
                Segment { a: Vec2::new(1.0, 5.0), b: Vec2::new(3.0, 5.0) },
                Segment { a: Vec2::new(3.0, 5.0), b: Vec2::new(3.0, 6.0) },
                Segment { a: Vec2::new(3.0, 6.0), b: Vec2::new(1.0, 6.0) },
                Segment { a: Vec2::new(1.0, 6.0), b: Vec2::new(1.0, 5.0) },
            ],
            sight: Vec::new(),
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
            footprint: [1, 1],
            });
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        assert!(table.move_token(&player, "ada", 2.5, 7.5).unwrap(), "the walk routes around the table");
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
        let err = table.publish_built(&player, "Glade".into(), layout.clone(), Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new(), sight: Vec::new() }, None);
        assert!(err.is_err());
        table
            .publish_built(&gm(), "Glade".into(), layout, Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new(), sight: Vec::new() }, None)
            .unwrap();
        assert_eq!(table.scene.as_ref().unwrap().name, "Glade");
        assert!(table.scene.as_ref().unwrap().layout.is_some());
    }

    #[test]
    fn a_smaller_map_does_not_slide_a_figure_inward() {
        let mut table = Table::new();
        let wide = MapLayout::blank(8, 8, "core/grass");
        let id = table
            .publish_built(&gm(), "Glade".into(), wide, Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new(), sight: Vec::new() }, None)
            .unwrap();
        table.tokens.push(Token {
            id: "ada".into(),
            name: "Ada".into(),
            x: 7.5,
            y: 7.5,
            image_file: None,
            color: "#7eb8a2".into(),
            owner_id: None,
            hidden: false,
            vision_radius: 8.0,
            grants_vision: true,
            character_id: None,
            cast_override: false,
            cast_asset: None,
            facing: "south".into(),
            footprint: [1, 1],
        });
        let small = MapLayout::blank(6, 6, "core/grass");
        let err = table
            .publish_built(&gm(), "Glade".into(), small, Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new(), sight: Vec::new() }, Some(id))
            .unwrap_err();
        assert!(err.contains("Ada"), "{err}");
        assert!(err.contains("outside"), "{err}");
        assert!((table.tokens[0].x - 7.5).abs() < 0.01, "the figure stayed put");
        assert_eq!(table.scene.as_ref().unwrap().width, 8.0);
    }

    #[test]
    fn a_figure_inside_an_object_blocks_the_update() {
        let mut table = Table::new();
        let layout = MapLayout::blank(8, 8, "core/grass");
        let id = table
            .publish_built(&gm(), "Glade".into(), layout.clone(), empty_compiled(), None)
            .unwrap();
        table.tokens.push(named_token("Ada", 3.5, 4.5));
        let blocked = Compiled {
            walls: Vec::new(),
            doors: Vec::new(),
            move_blockers: vec![
                Segment { a: Vec2::new(3.0, 4.0), b: Vec2::new(4.0, 4.0) },
                Segment { a: Vec2::new(4.0, 4.0), b: Vec2::new(4.0, 5.0) },
                Segment { a: Vec2::new(4.0, 5.0), b: Vec2::new(3.0, 5.0) },
                Segment { a: Vec2::new(3.0, 5.0), b: Vec2::new(3.0, 4.0) },
            ],
            lights: Vec::new(),
            sight: Vec::new(),
        };
        let err = table.publish_built(&gm(), "Glade".into(), layout, blocked, Some(id)).unwrap_err();
        assert!(err.contains("Ada"), "{err}");
        assert!(err.contains("standing in something"), "{err}");
        assert!((table.tokens[0].x - 3.5).abs() < 0.01);
        assert!(table.scene.as_ref().unwrap().move_blockers.is_empty());
    }

    #[test]
    fn a_new_map_keeps_figures_with_the_map_they_were_on() {
        let mut table = Table::new();
        let wide = MapLayout::blank(8, 8, "core/grass");
        let first = table.publish_built(&gm(), "Glade".into(), wide, empty_compiled(), None).unwrap();
        table.tokens.push(named_token("Ada", 2.5, 2.5));
        let small = MapLayout::blank(6, 6, "core/grass");
        let second = table.publish_built(&gm(), "Yard".into(), small, empty_compiled(), None).unwrap();
        assert_ne!(first, second);
        assert!(table.tokens.is_empty(), "the new map does not inherit the figures");
        let shelf = table.scenes.iter().find(|scene| scene.id == first).unwrap();
        assert_eq!(shelf.tokens[0].name, "Ada");
        assert_eq!(table.scene.as_ref().unwrap().name, "Yard");
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
    fn damage_requires_a_controlled_hit_and_each_hit_is_consumed_once() {
        use crate::systems::{load_systems, CharacterKind, RollOutcome};
        let catalog = load_systems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../systems").as_path());
        let system = catalog.get("daggerheart").unwrap();
        let mut table = Table::new();
        for (name, kind) in [("Ada", CharacterKind::Pc), ("Bandit", CharacterKind::Adversary)] {
            table.command(&gm(), ClientMsg::CreateCharacter { name: name.into(), kind }, Some(system)).unwrap();
        }
        let attacker = table.characters[0].id.clone();
        let target = table.characters[1].id.clone();
        table.characters[0].owner_id = Some("p".into());
        table.characters[0].texts.insert("damage".into(), "2d8+1 phy".into());
        let owner = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "p".into() };
        let stranger = Viewer { player_id: "other".into(), role: Role::Player, view_as: None, session_token: "other".into() };
        assert!(table.prepare_strike(&owner, "absent", &attacker, &target, 0, 0).is_err());
        let mut contest = table.prepare_attack(&owner, &attacker, &target, 0, 0).unwrap().contest;
        contest.verdict = Some("miss".into());
        table.rolls.push(StoredRoll {
            id: "attack".into(), roller_id: "p".into(), roller_name: "Player".into(), formula: "1d12h+1d12f".into(),
            label: None, visibility: RollVisibility::Public, modifier: 0, total: 4, dice: Vec::new(), created_at: 0,
            outcome: None, contest: Some(contest),
        });
        assert!(table.prepare_strike(&owner, "attack", &attacker, &target, 0, 0).is_err());
        table.rolls[0].contest.as_mut().unwrap().verdict = Some("hit".into());
        table.rolls[0].outcome = Some(RollOutcome { label: "Critical success".into(), tone: "critical".into() });
        assert!(table.prepare_strike(&stranger, "attack", &attacker, &target, 0, 0).is_err());
        assert!(table.prepare_strike(&owner, "attack", &attacker, &attacker, 0, 0).is_err());
        let plan = table.prepare_strike(&owner, "attack", &attacker, &target, 0, 0).unwrap();
        let replay = table.prepare_strike(&owner, "attack", &attacker, &target, 0, 0).unwrap();
        assert_eq!(plan.formula, "2d8+1+16");
        assert_eq!(plan.contest.attack_roll_id.as_deref(), Some("attack"));
        assert_eq!(table.characters[1].numbers["hp"], 0, "a hit does not roll or mark damage by itself");
        table.claim_damage(&owner, &plan).unwrap();
        assert!(table.claim_damage(&owner, &replay).is_err(), "concurrent clicks cannot reserve the same hit");
        let saved_during_simulation: Vec<StoredRoll> = serde_json::from_str(&serde_json::to_string(&table.rolls).unwrap()).unwrap();
        assert!(!saved_during_simulation[0].contest.as_ref().unwrap().damage_rolled,
            "an unrelated save while dice simulate must not consume an unapplied hit");
        table.cancel_damage(&plan);
        table.claim_damage(&owner, &plan).unwrap();
        let blow = table.receive_blow(&target, 13, false, system).unwrap();
        assert_eq!(blow.applied_marks, 3);
        assert_eq!(table.characters[1].numbers["hp"], 3);
        let mut damage_contest = plan.contest.clone();
        damage_contest.blow = Some(blow);
        table.commit_roll(&RollEvent {
            id: "damage".into(), roller_id: "p".into(), roller_name: "Player".into(), formula: plan.formula.clone(),
            label: None, visibility: RollVisibility::Public, modifier: 0, total: 13, dice: Vec::new(), play_at: 0,
            keyframe_dt: 0.0, outcome: None, contest: Some(damage_contest),
        });
        assert!(!table.damage_in_flight("attack"));
        // Save/load keeps the consumed flag, so replay after a reconnect cannot deal another blow.
        table.rolls = serde_json::from_str(&serde_json::to_string(&table.rolls).unwrap()).unwrap();
        assert!(table.prepare_strike(&owner, "attack", &attacker, &target, 0, 0).is_err());
        assert_eq!(table.characters[1].numbers["hp"], 3);
    }

    #[test]
    fn adversary_damage_uses_threshold_boundaries_and_clamps_marked_hp() {
        use crate::systems::{load_systems, CharacterKind};
        let catalog = load_systems(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../systems").as_path());
        let system = catalog.get("daggerheart").unwrap();
        let mut table = Table::new();
        table.command(&gm(), ClientMsg::CreateCharacter { name: "Bandit".into(), kind: CharacterKind::Adversary }, Some(system)).unwrap();
        let target = table.characters[0].id.clone();
        table.characters[0].numbers.extend(BTreeMap::from([
            ("minor".into(), 1), ("major".into(), 7), ("severe".into(), 13), ("hpmax".into(), 6),
            ("armorscore".into(), 2), // Adversaries do not spend PC Armor Slots.
        ]));
        for (amount, marks, step) in [(-1, 0, "None"), (0, 0, "None"), (1, 1, "Minor"), (6, 1, "Minor"),
            (7, 2, "Major"), (12, 2, "Major"), (13, 3, "Severe"), (99, 3, "Severe")] {
            table.characters[0].numbers.insert("hp".into(), 0);
            let blow = table.receive_blow(&target, amount, false, system).unwrap();
            assert_eq!((blow.step.as_str(), blow.marks, blow.applied_marks), (step, marks, marks));
            assert_eq!((blow.major_threshold, blow.severe_threshold), (Some(7), Some(13)));
            assert!(!blow.pending_armor);
            assert!(table.strike.is_none());
            assert_eq!(table.characters[0].numbers["hp"], marks);
        }
        table.characters[0].numbers.insert("hp".into(), 5);
        let lethal = table.receive_blow(&target, 20, false, system).unwrap();
        assert_eq!(lethal.marks, 3);
        assert_eq!(lethal.applied_marks, 1);
        assert_eq!(table.characters[0].numbers["hp"], 6);
        assert_eq!(table.receive_blow(&target, 20, false, system).unwrap().applied_marks, 0);
        table.characters[0].kind = CharacterKind::Pc;
        table.characters[0].numbers.insert("hp".into(), 0);
        let direct = table.receive_blow(&target, 7, true, system).unwrap();
        assert!(!direct.pending_armor);
        assert_eq!(direct.applied_marks, 2);
        assert!(table.strike.is_none());
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

        let blow = table.receive_blow(&ada, 12, false, system).unwrap();
        assert_eq!(blow.step, "Major");
        assert_eq!(blow.marks, 2);
        assert!(table.strike.is_some());
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));
        let mut damage_contest = attack.contest.clone();
        damage_contest.blow = Some(blow);
        table.commit_roll(&RollEvent {
            id: "damage".into(), roller_id: "gm".into(), roller_name: "GM".into(), formula: "1d8".into(),
            label: None, visibility: RollVisibility::Public, modifier: 0, total: 12, dice: Vec::new(), play_at: 0,
            keyframe_dt: 0.0, outcome: None, contest: Some(damage_contest),
        });

        table
            .command(&gm(), ClientMsg::AnswerStrike { use_armor: true }, Some(system))
            .unwrap();
        assert!(table.strike.is_none());
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(1));
        assert_eq!(table.characters[0].numbers.get("armor").copied(), Some(1));
        let receipt = table.rolls.last().unwrap().contest.as_ref().unwrap().blow.as_ref().unwrap();
        assert_eq!(receipt.applied_marks, 1);
        assert!(!receipt.pending_armor, "the saved damage receipt follows the armor answer");

        table.command(&gm(), ClientMsg::Undo, Some(system)).unwrap();
        assert_eq!(table.characters[0].numbers.get("hp").copied(), Some(0));
        assert_eq!(table.characters[0].numbers.get("armor").copied(), Some(0));
        assert!(table.undo.is_none());

        table.characters[0].numbers.insert("armorscore".into(), 0);
        let bare = table.receive_blow(&ada, 7, false, system).unwrap();
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
            environment: Default::default(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
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
            footprint: [1, 1],
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
    fn lighting_cannot_change_the_fixed_view() {
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
            environment: Default::default(),
            layout: None,
            move_blockers: Vec::new(),
            sight: Vec::new(),
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
            footprint: [1, 1],
            });
        table.explored.set(19, 19, true);
        let player = Viewer {
            player_id: "p".into(),
            role: Role::Player,
            view_as: None,
            session_token: "seat".into(),
        };
        assert!(table.command(&player, ClientMsg::SetLighting { lighting: "night".into() }, None).is_err());
        let refused = match table.command(&gm(), ClientMsg::SetLighting { lighting: "night".into() }, None) {
            Err(message) => message,
            Ok(_) => panic!("lighting should not change the view"),
        };
        assert!(refused.contains("fixed view"), "{refused}");
        let scene = table.scene.as_ref().unwrap();
        assert_eq!(scene.id, "camp");
        assert_eq!(scene.presentation.as_ref().unwrap().active_lighting, "day");
        assert_eq!(scene.walls.len(), 1);
        assert_eq!(table.tokens.len(), 1);
        assert!(table.explored.get(19, 19));
        let seen = crate::session::permissions::snapshot(&table, &player);
        let notice = seen.scene.unwrap().presentation.unwrap();
        assert!(notice.views.is_empty());
        assert!(notice.view_order.is_empty());
        assert!(notice.warnings.iter().any(|line| line.contains("filed away")));
        assert!(notice.warnings.iter().all(|line| !line.contains(".png")));
    }

    #[test]
    fn a_player_climbs_stairs_and_not_a_cliff() {
        use crate::map::compile::Compiled;
        use crate::map::layout::{encode_cells, StairLink};
        use crate::net::protocol::ClientMsg;

        let mut levels = vec![0_u8; 36];
        levels[1] = 1;
        levels[2] = 2;
        levels[6] = 1;
        let mut layout = crate::map::MapLayout::blank(6, 6, "core/grass");
        layout.levels = Some(encode_cells(&levels));
        layout.stairs = Some(vec![StairLink { id: "up".into(), asset: "steps/stair".into(), x: 0, y: 0, x2: 1, y2: 0 }]);
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        let compiled = Compiled { walls: Vec::new(), doors: Vec::new(), move_blockers: Vec::new(), lights: Vec::new(), sight: Vec::new() };
        table.publish_built(&gm(), "Rise".into(), layout, compiled, None).unwrap();
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Ada".into(),
                    x: 0.5,
                    y: 0.5,
                    owner_id: Some("p".into()),
                    hidden: false,
                    vision_radius: 6.0,
                    grants_vision: true,
                    color: "#d4a656".into(),
                    character_id: None,
                    cast_asset: None,
                    facing: Some("east".into()),
                },
                None,
            )
            .unwrap();
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let id = table.tokens[0].id.clone();
        let cliff = table.command(&player, ClientMsg::MoveToken { token_id: id.clone(), x: 0.5, y: 1.5 }, None).err().expect("a cliff should stop the move");
        assert!(cliff.contains("no stairs"), "{cliff}");
        assert!((table.tokens[0].x - 0.5).abs() < 0.01);
        table.command(&player, ClientMsg::MoveToken { token_id: id.clone(), x: 1.5, y: 0.5 }, None).expect("the stair should let the player climb");
        assert!((table.tokens[0].x - 1.5).abs() < 0.01);
        let high = table.command(&player, ClientMsg::MoveToken { token_id: id.clone(), x: 2.5, y: 0.5 }, None).err().expect("the next rise has no stair");
        assert!(high.contains("no stairs"), "{high}");
        table.command(&gm(), ClientMsg::TeleportToken { token_id: id, x: 2.5, y: 0.5 }, None).unwrap();
        assert!((table.tokens[0].x - 2.5).abs() < 0.01);
    }

    #[test]
    fn a_player_walks_around_a_wall() {
        use crate::net::protocol::ClientMsg;

        let layout = MapLayout::blank(5, 5, "core/grass");
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        let mut compiled = empty_compiled();
        compiled.walls.push(Segment { a: Vec2::new(1.0, 0.0), b: Vec2::new(1.0, 1.0) });
        table.publish_built(&gm(), "Yard".into(), layout, compiled, None).unwrap();
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Ada".into(),
                    x: 0.5,
                    y: 0.5,
                    owner_id: Some("p".into()),
                    hidden: false,
                    vision_radius: 6.0,
                    grants_vision: true,
                    color: "#d4a656".into(),
                    character_id: None,
                    cast_asset: None,
                    facing: Some("east".into()),
                },
                None,
            )
            .unwrap();
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let id = table.tokens[0].id.clone();
        table.command(&player, ClientMsg::MoveToken { token_id: id, x: 1.5, y: 0.5 }, None).expect("the walk goes around the wall");
        assert!((table.tokens[0].x - 1.5).abs() < 0.01);
        assert!((table.tokens[0].y - 0.5).abs() < 0.01);
    }

    #[test]
    fn a_wide_body_must_stand_on_one_level() {
        use crate::map::layout::encode_cells;
        use crate::net::protocol::ClientMsg;

        let mut levels = vec![0_u8; 64];
        for y in 0..8 {
            for x in 4..8 {
                levels[y * 8 + x] = 1;
            }
        }
        let mut layout = MapLayout::blank(8, 8, "core/grass");
        layout.levels = Some(encode_cells(&levels));
        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.cast_footprints.insert("ruin/brute".into(), [2, 2]);
        let id = table.publish_built(&gm(), "Yard".into(), layout.clone(), empty_compiled(), None).unwrap();
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Brute".into(),
                    x: 3.0,
                    y: 3.0,
                    owner_id: Some("p".into()),
                    hidden: false,
                    vision_radius: 6.0,
                    grants_vision: true,
                    color: "#d4a656".into(),
                    character_id: None,
                    cast_asset: Some("ruin/brute".into()),
                    facing: Some("south".into()),
                },
                None,
            )
            .unwrap();
        assert_eq!(table.tokens[0].footprint, [2, 2]);
        assert!((table.tokens[0].x - 3.0).abs() < 0.01, "an even body sits on the corner");
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let token_id = table.tokens[0].id.clone();
        let split = match table.command(&player, ClientMsg::MoveToken { token_id: token_id.clone(), x: 4.0, y: 3.0 }, None) {
            Err(message) => message,
            Ok(_) => panic!("a split body should stay put"),
        };
        assert!(split.contains("one level"), "{split}");
        assert!((table.tokens[0].x - 3.0).abs() < 0.01);
        table.command(&player, ClientMsg::MoveToken { token_id: token_id.clone(), x: 3.0, y: 5.0 }, None).unwrap();
        assert!((table.tokens[0].y - 5.0).abs() < 0.01, "the same level is a legal step");
        let climb = match table.command(&player, ClientMsg::MoveToken { token_id: token_id.clone(), x: 5.0, y: 5.0 }, None) {
            Err(message) => message,
            Ok(_) => panic!("a wide body should not cross onto the rise"),
        };
        assert!(climb.contains("one level"), "{climb}");
        assert!((table.tokens[0].x - 3.0).abs() < 0.01);
        assert!((table.tokens[0].y - 5.0).abs() < 0.01);
        table.tokens[0].x = 3.0;
        table.tokens[0].y = 3.0;
        let mut raised = layout;
        let mut stepped = vec![0_u8; 64];
        stepped[3 * 8 + 3] = 1;
        raised.levels = Some(encode_cells(&stepped));
        let err = table.publish_built(&gm(), "Yard".into(), raised, empty_compiled(), Some(id)).unwrap_err();
        assert!(err.contains("Brute"), "{err}");
        assert!(err.contains("two levels"), "{err}");
        assert!((table.tokens[0].x - 3.0).abs() < 0.01);
    }

    #[test]
    fn the_largest_map_publishes_and_a_larger_one_does_not() {
        use crate::net::protocol::ClientMsg;

        let mut table = Table::new();
        table.players.push(Player {
            id: "p".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        let layout = MapLayout::blank(200, 200, "core/grass");
        layout.validate().unwrap();
        let id = table.publish_built(&gm(), "Field".into(), layout, empty_compiled(), None).unwrap();
        assert_eq!(table.scene.as_ref().unwrap().width, 200.0);
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Ada".into(),
                    x: 1.5,
                    y: 1.5,
                    owner_id: Some("p".into()),
                    hidden: false,
                    vision_radius: 8.0,
                    grants_vision: true,
                    color: "#d4a656".into(),
                    character_id: None,
                    cast_asset: None,
                    facing: Some("south".into()),
                },
                None,
            )
            .unwrap();
        let token_id = table.tokens[0].id.clone();
        table
            .command(&gm(), ClientMsg::MoveToken { token_id, x: 199.5, y: 199.5 }, None)
            .unwrap();
        assert!((table.tokens[0].x - 199.5).abs() < 0.01);
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "t".into() };
        let seen = crate::session::permissions::snapshot(&table, &player);
        assert_eq!(seen.scene.as_ref().unwrap().width, 200.0);
        assert!(seen.tokens.iter().any(|token| token.name == "Ada"));
        let too_big = MapLayout::blank(201, 200, "core/grass");
        let err = table.publish_built(&gm(), "Field".into(), too_big, empty_compiled(), Some(id)).unwrap_err();
        assert!(err.contains("200"), "{err}");
        assert_eq!(table.scene.as_ref().unwrap().width, 200.0);
        assert!((table.tokens[0].x - 199.5).abs() < 0.01);
    }

    #[test]
    fn a_linked_figure_follows_the_character_picture() {
        use crate::net::protocol::ClientMsg;
        use crate::systems::CharacterKind;
        use std::collections::BTreeMap;

        let mut table = Table::new();
        table.players.push(Player {
            id: "gm".into(),
            name: "Host".into(),
            role: Role::Gm,
            color: "#f4f1ea".into(),
            connected: true,
        });
        table.cast_footprints.insert("ruin/brute".into(), [2, 2]);
        table
            .command(&gm(), ClientMsg::CreateCharacter { name: "Ada".into(), kind: CharacterKind::Pc }, None)
            .unwrap();
        let character_id = table.characters[0].id.clone();
        fn picture(table: &mut Table, character_id: &str, asset: &str) {
            table
                .command(
                    &gm(),
                    ClientMsg::UpdateCharacter {
                        character_id: character_id.to_string(),
                        name: None,
                        kind: None,
                        owner_id: None,
                        clear_owner: false,
                        numbers: BTreeMap::new(),
                        texts: BTreeMap::new(),
                        set_cast_asset: true,
                        cast_asset_id: Some(asset.into()),
                    },
                    None,
                )
                .unwrap();
        }
        picture(&mut table, &character_id, "ruin/brute");
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Ada".into(),
                    x: 2.5,
                    y: 2.5,
                    owner_id: None,
                    hidden: false,
                    vision_radius: 8.0,
                    grants_vision: true,
                    color: "#d4a656".into(),
                    character_id: Some(character_id.clone()),
                    cast_asset: None,
                    facing: Some("south".into()),
                },
                None,
            )
            .unwrap();
        table
            .command(
                &gm(),
                ClientMsg::CreateToken {
                    name: "Shade".into(),
                    x: 4.5,
                    y: 2.5,
                    owner_id: None,
                    hidden: false,
                    vision_radius: 8.0,
                    grants_vision: false,
                    color: "#c45c4a".into(),
                    character_id: Some(character_id.clone()),
                    cast_asset: Some("ruin/bandit".into()),
                    facing: Some("west".into()),
                },
                None,
            )
            .unwrap();
        assert_eq!(table.tokens[0].footprint, [2, 2], "the body size comes from the character picture at placement");
        assert!(!table.tokens[0].cast_override);
        assert!(table.tokens[1].cast_override);
        let cast_of = |table: &Table, name: &str| {
            let seen = crate::session::permissions::snapshot(table, &gm());
            seen.tokens.into_iter().find(|token| token.name == name).unwrap().cast_asset
        };
        assert_eq!(cast_of(&table, "Ada").as_deref(), Some("ruin/brute"));
        assert_eq!(cast_of(&table, "Shade").as_deref(), Some("ruin/bandit"));
        picture(&mut table, &character_id, "ruin/scout");
        assert_eq!(cast_of(&table, "Ada").as_deref(), Some("ruin/scout"));
        assert_eq!(cast_of(&table, "Shade").as_deref(), Some("ruin/bandit"));
        assert_eq!(table.tokens[0].footprint, [2, 2], "a new picture does not resize the body");
    }
    #[test]
    fn environment_is_gm_controlled_and_lighting_survives_save_reload() {
        use crate::map::layout::{EnvironmentLighting, EnvironmentPreset, LightKind};
        use crate::net::protocol::ClientMsg;
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let mut layout = crate::forest_scene::layout();
        let dusk = EnvironmentLighting { preset: EnvironmentPreset::Dusk, brightness: 0.55 };
        layout.environment = Some(dusk.clone());
        let compiled = crate::map::compile::compile(&layout, &catalog).unwrap();
        let mut table = Table::new();
        table.publish_built(&gm(), "Lighting".into(), layout, compiled, None).unwrap();
        assert_eq!(table.scene.as_ref().unwrap().environment, dusk);
        table.players.push(Player { id: "p".into(), name: "Player".into(), role: Role::Player, color: "#7eb8a2".into(), connected: true });
        let player = Viewer { player_id: "p".into(), role: Role::Player, view_as: None, session_token: "p".into() };
        let night = EnvironmentLighting { preset: EnvironmentPreset::Night, brightness: 0.16 };
        assert!(table.command(&player, ClientMsg::SetEnvironment { environment: night.clone() }, None).is_err());
        assert_eq!(table.scene.as_ref().unwrap().environment, dusk);
        assert!(table.command(&gm(), ClientMsg::SetEnvironment { environment: EnvironmentLighting { preset: EnvironmentPreset::Night, brightness: f32::NAN } }, None).is_err());
        let before = crate::session::permissions::snapshot(&table, &player);
        assert!(matches!(table.command(&gm(), ClientMsg::SetEnvironment { environment: night.clone() }, None).unwrap(), CommandEffect::Refresh));
        let after = crate::session::permissions::snapshot(&table, &player);
        assert_eq!(after.fog.explored, before.fog.explored, "lighting never reveals fog");
        assert_eq!(after.tokens.len(), before.tokens.len());
        assert_eq!(after.scene.as_ref().unwrap().environment, night);
        assert_eq!(after.scene.as_ref().unwrap().lights.len(), 2, "players see the same illumination");
        assert_eq!(table.scene.as_ref().unwrap().layout.as_ref().unwrap().environment.as_ref(), Some(&night));
        let dir = std::env::temp_dir().join(format!("longtable-lighting-{}", Uuid::new_v4()));
        crate::storage::write(&table, &dir).unwrap();
        let mut restored = Table::new();
        crate::storage::load_file(&mut restored, &dir.join("state.json")).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        let scene = restored.scene.as_ref().unwrap();
        assert_eq!(scene.environment, night);
        assert_eq!(scene.layout.as_ref().unwrap().environment.as_ref(), Some(&night));
        assert_eq!(scene.lights[0].kind, LightKind::Torch);
        assert_eq!(scene.lights[0].intensity, 1.4);
        assert_eq!(scene.lights[1].kind, LightKind::Magic);
        assert_eq!(scene.lights[1].color, "#79c8ff");
        let layout = scene.layout.as_ref().unwrap();
        assert_eq!(layout.lights[0].kind, scene.lights[0].kind);
        assert_eq!(layout.lights[1].intensity, scene.lights[1].intensity);
    }

}
