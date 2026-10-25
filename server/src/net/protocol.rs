// ------------------------------------------------------------
// Module: protocol
// Purpose: Every message that crosses the websocket. Field names
//          are camelCase on the wire. `cargo test` writes the
//          matching TypeScript into the client.
// ------------------------------------------------------------

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::map::MapLayout;
use crate::systems::{Campaign, Character, RollOutcome, SystemDef};

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
    /// Present when the map was built in the editor. Players receive it whole,
    /// the same way they receive an imported image. Fog still covers the view.
    #[serde(default)]
    pub layout: Option<MapLayout>,
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
    pub mode: FogMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Listing {
    pub id: String,
    pub name: String,
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
    },
    Ping {
        client_time: u64,
    },
    MoveToken {
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
    },
    DeleteCharacter {
        character_id: String,
    },
    AdjustResource {
        character_id: String,
        field: String,
        delta: i32,
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
