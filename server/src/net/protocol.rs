// ------------------------------------------------------------
// Module: protocol
// Purpose: Every message that crosses the websocket. Field names
//          are camelCase on the wire. `cargo test` writes the
//          matching TypeScript into the client.
// ------------------------------------------------------------

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::map::MapLayout;
use crate::map::layout::{EnvironmentLighting, LightKind, light_intensity};
use crate::systems::{Campaign, Character, CharacterKind, RollOutcome, SystemDef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum Role {
    Gm,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum RollVisibility {
    Public,
    ToGm,
    GmSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum FogBrushMode {
    Reveal,
    Hide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum FogMode {
    Gm,
    Player,
}

/// Disc tokens are the first view. A miniature can be added later
/// as another variant without changing how the table stores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum TokenKind {
    Disc,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FogCell {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct MeshFace {
    pub indices: Vec<u16>,
    pub value: u8,
    pub normal: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DieMesh {
    pub sides: u8,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<MeshFace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Keyframe {
    pub p: [f32; 3],
    pub q: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlayedDie {
    pub sides: u8,
    pub value: u8,
    pub sign: i8,
    #[serde(default)]
    pub tag: Option<String>,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RollEvent {
    pub id: String,
    pub roller_id: String,
    pub roller_name: String,
    pub formula: String,
    pub label: Option<String>,
    pub visibility: RollVisibility,
    pub modifier: i32,
    pub total: i32,
    pub play_at: u64,
    pub keyframe_dt: f32,
    pub dice: Vec<PlayedDie>,
    #[serde(default)]
    pub outcome: Option<RollOutcome>,
    /// Who this roll was against, when it was an attack or the damage that followed.
    #[serde(default)]
    pub contest: Option<Contest>,
}

/// An attack, or the damage roll that followed a hit.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Contest {
    pub attacker_id: String,
    pub attacker_name: String,
    pub target_id: String,
    pub target_name: String,
    pub defense_name: String,
    pub defense: i32,
    /// `hit` or `miss` on an attack. Empty on a damage roll.
    pub verdict: Option<String>,
    pub blow: Option<Blow>,
}

/// What a damage total does to the target before armor.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Blow {
    pub step: String,
    pub marks: i32,
}

/// The armor question waiting on the defender, or the GM when nobody owns them.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct StrikeOffer {
    pub target_id: String,
    pub target_name: String,
    pub step: String,
    pub marks: i32,
    pub with_armor: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ConcealedRoll {
    pub id: String,
    pub notice: String,
    pub play_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlayerInfo {
    pub id: String,
    pub name: String,
    pub role: Role,
    pub color: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SegmentView {
    pub a: [f32; 2],
    pub b: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DoorView {
    pub id: String,
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LightView {
    pub x: f32,
    pub y: f32,
    pub range: f32,
    pub color: String,
    #[serde(default)]
    #[ts(as = "Option<LightKind>", optional)]
    pub kind: LightKind,
    #[serde(default = "light_intensity")]
    #[ts(as = "Option<f32>", optional)]
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PresentationLighting {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PresentationCapabilities {
    pub depth: bool,
    pub doors: bool,
    pub move_blockers: bool,
    pub walls: bool,
    pub lights: bool,
    pub snow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PresentationVariant {
    pub id: String,
    pub color_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PresentationView {
    pub id: String,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub ortho_half_width: f32,
    pub ortho_half_height: f32,
    pub target: [f32; 3],
    pub image_size: [u32; 2],
    pub pull: f32,
    pub near: f32,
    pub far: f32,
    pub depth_url: Option<String>,
    pub variants: Vec<PresentationVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PresentationDoorLayer {
    pub view: String,
    pub lighting: String,
    pub color_url: String,
    pub depth_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PresentationDoor {
    pub id: String,
    pub layers: Vec<PresentationDoorLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Presentation {
    pub active_lighting: String,
    pub default_view: String,
    pub view_order: Vec<String>,
    pub lightings: Vec<PresentationLighting>,
    pub views: Vec<PresentationView>,
    pub doors: Vec<PresentationDoor>,
    pub capabilities: PresentationCapabilities,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SceneView {
    pub id: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub image_url: Option<String>,
    /// Walls are sent to the GM only. Players see the drawing, not the vectors.
    pub walls: Vec<SegmentView>,
    pub lights: Vec<LightView>,
    #[serde(default)]
    #[ts(as = "Option<EnvironmentLighting>", optional)]
    pub environment: EnvironmentLighting,
    /// Present when the map was built in the editor. Players receive it whole,
    /// the same way they receive an imported image. Fog still covers the view.
    #[serde(default)]
    pub layout: Option<MapLayout>,
    /// Angled pictures of this grid. Missing on an ordinary battle map.
    #[serde(default)]
    pub presentation: Option<Presentation>,
    #[serde(default = "schema_one")]
    pub schema_version: u32,
    #[serde(default)]
    pub visual_profile: String,
}

fn schema_one() -> u32 {
    1
}

fn facing_south() -> String {
    "south".into()
}


pub(crate) fn flex_revision<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    struct Visit;
    impl<'de> serde::de::Visitor<'de> for Visit {
        type Value = String;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a revision string or a small number")
        }
        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
            Ok(value.to_string())
        }
        fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<String, E> {
            Ok(value.to_string())
        }
        fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<String, E> {
            Ok(value.to_string())
        }
    }
    deserializer.deserialize_any(Visit)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct TokenView {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub kind: TokenKind,
    pub image_url: Option<String>,
    pub color: String,
    pub owner_id: Option<String>,
    pub hidden: bool,
    pub vision_radius: f32,
    pub grants_vision: bool,
    #[serde(default)]
    pub character_id: Option<String>,
    /// Placement override. When set, this wins over the character's cast.
    #[serde(default)]
    #[ts(optional)]
    pub cast_asset: Option<String>,
    /// World facing: north, east, south, or west. The camera does not turn.
    #[serde(default = "facing_south")]
    pub facing: String,
    /// Gameplay size in cells. Absent means one square.
    #[serde(default)]
    #[ts(optional)]
    pub footprint: Option<[u32; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DieResult {
    pub sides: u8,
    pub value: u8,
    pub sign: i8,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RollLogEntry {
    pub id: String,
    pub roller_name: String,
    pub formula: Option<String>,
    pub label: Option<String>,
    pub visibility: RollVisibility,
    pub modifier: i32,
    pub total: Option<i32>,
    pub dice: Vec<DieResult>,
    pub concealed: bool,
    pub notice: Option<String>,
    pub created_at: u64,
    #[serde(default)]
    pub outcome: Option<RollOutcome>,
    #[serde(default)]
    pub contest: Option<Contest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FogView {
    pub cells_per_square: u32,
    pub width: u32,
    pub height: u32,
    pub explored: String,
    pub visible_polygons: Vec<Vec<[f32; 2]>>,
    /// Cells the viewer can see right now, in the same bit order as `explored`.
    /// Absent means the client still uses `visiblePolygons`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub visible: Option<String>,
    pub mode: FogMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Listing {
    pub id: String,
    pub name: String,
}

/// A token's public face. Words only: exact marks stay on the sheet.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Figure {
    pub token_id: String,
    pub name: String,
    pub kind: CharacterKind,
    pub health: String,
    pub stress: String,
    pub hope: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Snapshot {
    pub you: PlayerInfo,
    pub players: Vec<PlayerInfo>,
    pub scene: Option<SceneView>,
    pub tokens: Vec<TokenView>,
    pub doors: Vec<DoorView>,
    pub fog: FogView,
    pub rolls: Vec<RollLogEntry>,
    pub view_as: Option<String>,
    #[serde(default)]
    pub campaign: Campaign,
    #[serde(default)]
    pub characters: Vec<Character>,
    /// Other campaigns and this campaign's scenes. Empty for a player.
    #[serde(default)]
    pub campaigns: Vec<Listing>,
    #[serde(default)]
    pub scenes: Vec<Listing>,
    /// Daggerheart Fear. Zero on any other system, and on a save from before it existed.
    #[serde(default)]
    pub fear: u8,
    #[serde(default)]
    pub figures: Vec<Figure>,
    #[serde(default)]
    pub can_undo: bool,
    /// The armor question, when this viewer is allowed to answer it.
    #[serde(default)]
    pub strike: Option<StrikeOffer>,
    /// What has been said at this table, including public rolls.
    #[serde(default)]
    pub talk: Vec<TalkLine>,
    /// Changes when a pack is imported, so every seat can reload the library.
    #[serde(default, deserialize_with = "flex_revision")]
    pub catalog_revision: String,
    /// Why the table will not write, when a save could not be opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub notice: Option<String>,
    /// What an older save kept. Only the GM receives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub review: Option<String>,
}

/// One line in the talk panel. The speaker is the character, when one is chosen.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct TalkLine {
    pub id: String,
    pub speaker: String,
    pub body: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum ClientMsg {
    Join {
        name: String,
        passphrase: String,
        #[serde(default)]
        as_gm: bool,
        #[serde(default)]
        gm_key: Option<String>,
        #[serde(default)]
        resume_id: Option<String>,
        #[serde(default)]
        account_token: Option<String>,
        #[serde(default)]
        table_id: Option<String>,
    },
    /// The character this seat is speaking and rolling as.
    Voice {
        #[serde(default)]
        character_id: Option<String>,
    },
    Say {
        text: String,
    },
    Ping {
        client_time: u64,
    },
    MoveToken {
        token_id: String,
        x: f32,
        y: f32,
    },
    /// Deliberate GM placement; ordinary movement always follows walkable ground.
    TeleportToken {
        token_id: String,
        x: f32,
        y: f32,
    },
    Roll {
        formula: String,
        visibility: RollVisibility,
        #[serde(default)]
        label: Option<String>,
    },
    SetDoor {
        door_id: String,
        open: bool,
    },
    SetEnvironment {
        environment: EnvironmentLighting,
    },
    SetLighting {
        lighting: String,
    },
    FogBrush {
        mode: FogBrushMode,
        cells: Vec<FogCell>,
    },
    CreateToken {
        name: String,
        x: f32,
        y: f32,
        #[serde(default)]
        owner_id: Option<String>,
        #[serde(default)]
        hidden: bool,
        vision_radius: f32,
        #[serde(default = "default_true")]
        grants_vision: bool,
        color: String,
        #[serde(default)]
        character_id: Option<String>,
        #[serde(default)]
        cast_asset: Option<String>,
        #[serde(default)]
        facing: Option<String>,
    },
    UpdateToken {
        token_id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        owner_id: Option<String>,
        #[serde(default)]
        clear_owner: bool,
        #[serde(default)]
        hidden: Option<bool>,
        #[serde(default)]
        vision_radius: Option<f32>,
        #[serde(default)]
        grants_vision: Option<bool>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        character_id: Option<String>,
        #[serde(default)]
        clear_character: bool,
        #[serde(default)]
        set_cast_asset: bool,
        #[serde(default)]
        cast_asset: Option<String>,
        #[serde(default)]
        facing: Option<String>,
    },
    DeleteToken {
        token_id: String,
    },
    RemovePlayer {
        player_id: String,
    },
    ViewAs {
        player_id: Option<String>,
    },
    BlankMap {
        width: u32,
        height: u32,
    },
    SetCampaignSystem {
        system_id: Option<String>,
    },
    SaveSystem {
        system: SystemDef,
    },
    CreateCharacter {
        name: String,
        kind: crate::systems::CharacterKind,
    },
    UpdateCharacter {
        character_id: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        kind: Option<crate::systems::CharacterKind>,
        #[serde(default)]
        owner_id: Option<String>,
        #[serde(default)]
        clear_owner: bool,
        #[serde(default)]
        numbers: std::collections::BTreeMap<String, i32>,
        #[serde(default)]
        texts: std::collections::BTreeMap<String, String>,
        #[serde(default)]
        set_cast_asset: bool,
        #[serde(default)]
        cast_asset_id: Option<String>,
    },
    DeleteCharacter {
        character_id: String,
    },
    AdjustResource {
        character_id: String,
        field: String,
        delta: i32,
    },
    AdjustFear {
        delta: i32,
    },
    ApplyDamage {
        character_id: String,
        field_id: String,
        amount: i32,
    },
    RollAction {
        character_id: String,
        action_id: String,
        visibility: RollVisibility,
        #[serde(default)]
        plus: i32,
        #[serde(default)]
        minus: i32,
    },
    Attack {
        attacker_id: String,
        target_id: String,
        #[serde(default)]
        plus: i32,
        #[serde(default)]
        minus: i32,
    },
    Strike {
        attacker_id: String,
        target_id: String,
        #[serde(default)]
        plus: i32,
        #[serde(default)]
        minus: i32,
    },
    AnswerStrike {
        use_armor: bool,
    },
    Undo,
    CreateCampaign {
        name: String,
        system_id: Option<String>,
    },
    SwitchCampaign {
        campaign_id: String,
    },
    RenameCampaign {
        campaign_id: String,
        name: String,
    },
    DeleteCampaign {
        campaign_id: String,
    },
    CreateScene {
        name: String,
        width: u32,
        height: u32,
    },
    SwitchScene {
        scene_id: String,
    },
    RenameScene {
        scene_id: String,
        name: String,
    },
    DeleteScene {
        scene_id: String,
    },
    DuplicateScene {
        scene_id: String,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum ServerMsg {
    Welcome {
        player_id: String,
        role: Role,
        session_token: String,
        server_time: u64,
        dice: Vec<DieMesh>,
    },
    Pong {
        client_time: u64,
        server_time: u64,
    },
    Snapshot(Snapshot),
    Roll(RollEvent),
    Concealed(ConcealedRoll),
    Error {
        message: String,
    },
    /// The GM removed this seat. The browser should leave and not reconnect.
    Kicked {
        message: String,
    },
    Systems {
        systems: Vec<SystemDef>,
    },
}

pub fn dice_meshes() -> Vec<DieMesh> {
    use crate::dice::{templates, DieKind};
    [DieKind::D4, DieKind::D6, DieKind::D8, DieKind::D10, DieKind::D12, DieKind::D20]
        .into_iter()
        .map(|kind| {
            let template = templates()[kind.sides() as usize].as_ref().expect("die mesh");
            DieMesh {
                sides: kind.sides(),
                vertices: template.vertices.clone(),
                faces: template
                    .faces
                    .iter()
                    .map(|face| MeshFace {
                        indices: face.indices.clone(),
                        value: face.value,
                        normal: face.normal,
                    })
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_and_snapshot_use_a_type_tag() {
        let join: ClientMsg = serde_json::from_str(
            r#"{"type":"join","name":"Ada","passphrase":"table","asGm":true,"gmKey":"ABC","resumeId":null}"#,
        )
        .unwrap();
        assert!(matches!(
            join,
            ClientMsg::Join { as_gm: true, gm_key: Some(key), .. } if key == "ABC"
        ));

        let message = ServerMsg::Error { message: "no".into() };
        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(json["type"], "error");
    }

    #[test]
    fn export_typescript_bindings() {
        let cfg = ts_rs::Config::from_env().with_large_int("number");
        ServerMsg::export_all(&cfg).unwrap();
        ClientMsg::export_all(&cfg).unwrap();
    }
}
