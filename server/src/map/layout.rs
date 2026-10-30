// ------------------------------------------------------------
// Module: layout
// Purpose: The built map, as the editor saves it. Cells are one
//          byte each, base64, row-major, x fastest. Pieces sit
//          on the edges between cells.
// ------------------------------------------------------------

use std::collections::HashSet;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MIN_SIDE: u32 = 5;
pub const MAX_SIDE: u32 = 200;
const MAX_PIECES: usize = 4_000;
const MAX_PROPS: usize = 2_000;
const MAX_DECALS: usize = 8_000;
const MAX_LIGHTS: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum EdgeAxis {
    H,
    V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum PieceKind {
    Wall,
    Door,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EdgePiece {
    pub id: String,
    pub kind: PieceKind,
    pub asset: String,
    pub x: u32,
    pub y: u32,
    #[serde(default)]
    pub open: bool,
    pub axis: EdgeAxis,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlacedProp {
    pub id: String,
    pub asset: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub mirror: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlacedDecal {
    pub asset: String,
    pub x: f32,
    pub y: f32,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default)]
    pub angle: f32,
}

fn one() -> f32 {
    1.0
}

/// A light the GM placed on a built map. The marker is the source;
/// sight still comes from tokens.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlacedLight {
    pub x: f32,
    pub y: f32,
    pub range: f32,
    pub color: String,
}

/// A stair between two neighboring squares. The squares must differ by one tier.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct StairLink {
    pub id: String,
    pub asset: String,
    pub x: u32,
    pub y: u32,
    pub x2: u32,
    pub y2: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct MapLayout {
    pub width: u32,
    pub height: u32,
    pub terrains: Vec<String>,
    pub cells: String,
    #[serde(default)]
    pub pieces: Vec<EdgePiece>,
    #[serde(default)]
    pub props: Vec<PlacedProp>,
    #[serde(default)]
    pub decals: Vec<PlacedDecal>,
    #[serde(default)]
    pub lights: Vec<PlacedLight>,
    /// One height byte per cell, base64. Absent or empty means every cell is tier 0.
    #[serde(default)]
    #[ts(optional)]
    pub levels: Option<String>,
    /// Chooses tile variants. Absent means seed 1, so an old map does not reshuffle.
    #[serde(default)]
    #[ts(optional)]
    pub seed: Option<u32>,
    /// One byte per cell, base64. 0 lets the seed choose. 1 is the first picture.
    #[serde(default)]
    #[ts(optional)]
    pub variants: Option<String>,
    /// Stairs between neighboring squares. Absent means the map has none.
    #[serde(default)]
    #[ts(optional)]
    pub stairs: Option<Vec<StairLink>>,
}

impl MapLayout {
    pub fn blank(width: u32, height: u32, terrain: &str) -> Self {
        let count = (width as usize) * (height as usize);
        Self {
            width,
            height,
            terrains: vec![terrain.to_string()],
            cells: STANDARD.encode(vec![0_u8; count]),
            pieces: Vec::new(),
            props: Vec::new(),
            decals: Vec::new(),
            lights: Vec::new(),
            levels: None,
            seed: Some(1),
            variants: None,
            stairs: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_SIDE..=MAX_SIDE).contains(&self.width) || !(MIN_SIDE..=MAX_SIDE).contains(&self.height) {
            return Err(format!("a map is between {MIN_SIDE} and {MAX_SIDE} squares on a side"));
        }
        if self.terrains.is_empty() || self.terrains.len() > 64 {
            return Err("a map needs between 1 and 64 terrains".into());
        }
        let bytes = decode_cells(&self.cells)?;
        let expected = (self.width as usize) * (self.height as usize);
        if bytes.len() != expected {
            return Err(format!("the cell grid has {} bytes and the map wants {expected}", bytes.len()));
        }
        if bytes.iter().any(|index| *index as usize >= self.terrains.len()) {
            return Err("a cell names a terrain that is not on this map".into());
        }
        if let Some(variants) = &self.variants {
            let pictures = decode_cells(variants)?;
            if pictures.len() != expected {
                return Err(format!("the picture grid has {} bytes and the map wants {expected}", pictures.len()));
            }
        }
        if self.pieces.len() > MAX_PIECES || self.props.len() > MAX_PROPS || self.decals.len() > MAX_DECALS || self.lights.len() > MAX_LIGHTS {
            return Err("this map has more pieces than the table will store".into());
        }
        for piece in &self.pieces {
            let (max_x, max_y) = match piece.axis {
                EdgeAxis::H => (self.width.saturating_sub(1), self.height),
                EdgeAxis::V => (self.width, self.height.saturating_sub(1)),
            };
            if piece.x > max_x || piece.y > max_y {
                return Err(format!("a piece at {},{} is off the map", piece.x, piece.y));
            }
        }
        for prop in &self.props {
            if !on_map(prop.x, prop.y, self.width, self.height) {
                return Err(format!("a prop at {},{} is off the map", prop.x, prop.y));
            }
        }
        for decal in &self.decals {
            if !on_map(decal.x, decal.y, self.width, self.height) {
                return Err(format!("a decal at {},{} is off the map", decal.x, decal.y));
            }
        }
        for light in &self.lights {
            if !on_map(light.x, light.y, self.width, self.height) {
                return Err(format!("a light at {},{} is off the map", light.x, light.y));
            }
            if !light.range.is_finite() || !(0.5..=40.0).contains(&light.range) {
                return Err("a light reaches between half a square and 40".into());
            }
            if light.color.is_empty() || light.color.chars().count() > 32 {
                return Err("a light needs a color".into());
            }
        }
        let heights = if let Some(levels) = &self.levels {
            let bytes = decode_cells(levels)?;
            let expected = (self.width as usize) * (self.height as usize);
            if bytes.len() != expected {
                return Err(format!("the height grid has {} bytes and the map wants {expected}", bytes.len()));
            }
            if bytes.iter().any(|level| *level > 2) {
                return Err("a square's height is 0, 1, or 2".into());
            }
            Some(bytes)
        } else {
            None
        };
        let stairs = self.stairs.as_deref().unwrap_or(&[]);
        if stairs.len() > MAX_PROPS {
            return Err("this map has more stairs than the table will store".into());
        }
        let bad: Vec<String> = stairs
            .iter()
            .filter(|stair| !self.stair_ok(stair, heights.as_deref()))
            .map(|stair| format!("({}, {}) to ({}, {})", stair.x, stair.y, stair.x2, stair.y2))
            .collect();
        if !bad.is_empty() {
            return Err(format!("These stairs do not join neighboring squares one step apart: {}.", bad.join("; ")));
        }
        let mut ids = HashSet::new();
        for id in self.pieces.iter().map(|piece| piece.id.as_str())
            .chain(self.props.iter().map(|prop| prop.id.as_str()))
            .chain(stairs.iter().map(|stair| stair.id.as_str()))
        {
            if id.is_empty() {
                return Err("a thing on this map has no id".into());
            }
            if !ids.insert(id) {
                return Err(format!("two things on this map use the id {id}"));
            }
        }
        Ok(())
    }

    fn stair_ok(&self, stair: &StairLink, heights: Option<&[u8]>) -> bool {
        if stair.x >= self.width || stair.y >= self.height || stair.x2 >= self.width || stair.y2 >= self.height {
            return false;
        }
        let dx = stair.x.abs_diff(stair.x2);
        let dy = stair.y.abs_diff(stair.y2);
        if dx + dy != 1 {
            return false;
        }
        let here = self.height_at(stair.x, stair.y, heights);
        let there = self.height_at(stair.x2, stair.y2, heights);
        here.abs_diff(there) == 1
    }

    /// The walkable height at a map point. Missing or off-map ground is 0.
    pub fn level_at(&self, x: f32, y: f32) -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        if ix < 0 || iy < 0 || ix >= self.width as i32 || iy >= self.height as i32 {
            return 0.0;
        }
        let Some(text) = &self.levels else { return 0.0 };
        let Ok(bytes) = decode_cells(text) else { return 0.0 };
        let index = (iy as u32 * self.width + ix as u32) as usize;
        bytes.get(index).copied().unwrap_or(0).min(2) as f32
    }

    pub fn has_raised_ground(&self) -> bool {
        let Some(text) = &self.levels else { return false };
        decode_cells(text).ok().is_some_and(|bytes| bytes.iter().any(|level| *level > 0))
    }

    fn height_at(&self, x: u32, y: u32, heights: Option<&[u8]>) -> u8 {
        let Some(heights) = heights else { return 0 };
        heights[(y * self.width + x) as usize]
    }
}

fn on_map(x: f32, y: f32, width: u32, height: u32) -> bool {
    x.is_finite() && y.is_finite() && x >= -0.01 && y >= -0.01 && x <= width as f32 + 0.01 && y <= height as f32 + 0.01
}

pub fn decode_cells(text: &str) -> Result<Vec<u8>, String> {
    STANDARD.decode(text.trim().as_bytes()).map_err(|_| "the cell grid is not base64".to_string())
}

pub fn encode_cells(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_layout_without_lights_still_loads() {
        let mut value = serde_json::to_value(MapLayout::blank(5, 5, "core/grass")).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("lights");
        object.remove("seed");
        let layout: MapLayout = serde_json::from_value(value).unwrap();
        assert!(layout.lights.is_empty());
        assert!(layout.seed.is_none());
        assert!(layout.variants.is_none());
        layout.validate().unwrap();
    }

    #[test]
    fn a_picture_grid_must_cover_the_map() {
        let mut layout = MapLayout::blank(5, 5, "core/grass");
        layout.variants = Some(encode_cells(&[0, 1]));
        let err = layout.validate().unwrap_err();
        assert!(err.contains("picture grid"), "{err}");
        layout.variants = Some(encode_cells(&vec![0_u8; 25]));
        layout.validate().unwrap();
    }

    #[test]
    fn two_props_cannot_share_an_id() {
        let mut layout = MapLayout::blank(5, 5, "core/grass");
        layout.props.push(PlacedProp { id: "same".into(), asset: "core/table".into(), x: 1.0, y: 1.0, mirror: false });
        layout.props.push(PlacedProp { id: "same".into(), asset: "core/table".into(), x: 3.0, y: 1.0, mirror: false });
        let err = layout.validate().unwrap_err();
        assert!(err.contains("two things on this map use the id same"), "{err}");
    }
}
