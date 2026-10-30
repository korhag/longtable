// ------------------------------------------------------------
// Module: presentation
// Purpose: Pictures filed with an old angled map. The table
//          keeps them. The play view does not draw them.
// ------------------------------------------------------------

/// Shown when a saved scene still has angled pictures on file.
pub const FILED_VIEW: &str = "This scene still has an angled picture. The table keeps that picture filed away and draws the fixed view.";

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prerendered {
    pub active_lighting: String,
    pub default_view: String,
    pub view_order: Vec<String>,
    pub lightings: Vec<LightingDef>,
    pub views: Vec<SavedView>,
    pub doors: Vec<SavedDoor>,
    pub capabilities: Capabilities,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightingDef {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub depth: bool,
    pub doors: bool,
    pub move_blockers: bool,
    pub walls: bool,
    pub lights: bool,
    pub snow: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedView {
    pub id: String,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub ortho_half_width: f32,
    pub ortho_half_height: f32,
    pub target: [f32; 3],
    pub image_size: [u32; 2],
    pub ground_to_image: [[f32; 3]; 2],
    pub height_to_image: [f32; 2],
    pub pull: f32,
    pub near: f32,
    pub far: f32,
    pub pixels_per_square: f32,
    pub depth_file: Option<String>,
    pub variants: Vec<SavedVariant>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedVariant {
    pub id: String,
    pub file: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDoor {
    pub id: String,
    pub views: Vec<SavedDoorView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDoorView {
    pub view: String,
    pub lighting: String,
    pub file: String,
    pub depth_file: Option<String>,
}
