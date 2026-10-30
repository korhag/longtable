// ------------------------------------------------------------
// Module: scene
// Purpose: Grid-space geometry shared by movement, vision, and
//          the client. The origin is the top-left of the map
//          image, x grows right, and y grows down.
// ------------------------------------------------------------

use serde::{Deserialize, Serialize};

use crate::map::layout::MapLayout;
use crate::map::presentation::Prerendered;

/// A point on the battle map, in grid squares.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

/// A straight wall or door edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub a: Vec2,
    pub b: Vec2,
}

/// A sight blocker with a vertical span. `base` and `top` are world heights.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SightBlock {
    pub a: Vec2,
    pub b: Vec2,
    pub base: f32,
    pub top: f32,
}

/// A door. Closed doors block movement and sight; open doors do not.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Door {
    pub id: String,
    pub a: Vec2,
    pub b: Vec2,
    pub open: bool,
}

/// A light marker imported with the map. Vision itself comes from tokens.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Light {
    pub x: f32,
    pub y: f32,
    pub range: f32,
    pub color: String,
}

pub const DIMETRIC_PROFILE: &str = "dimetric-30-v1";

fn schema_one() -> u32 {
    1
}

/// One imported or blank battle map.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    /// File name inside the maps directory. Empty when the map is a blank grid.
    pub image_file: Option<String>,
    pub walls: Vec<Segment>,
    pub doors: Vec<Door>,
    pub lights: Vec<Light>,
    /// The editor's layout, when this map was built rather than imported.
    #[serde(default)]
    pub layout: Option<MapLayout>,
    /// Props that stop walking but not sight, such as a table.
    #[serde(default)]
    pub move_blockers: Vec<Segment>,
    /// Finite sight spans. Absent on older saves, which keep infinite walls.
    #[serde(default)]
    pub sight: Vec<SightBlock>,
    /// Angled pictures of this same grid. Absent on older saves.
    #[serde(default)]
    pub presentation: Option<Prerendered>,
    /// 1 is the save shape this process writes. Older files omit it.
    #[serde(default = "schema_one")]
    pub schema_version: u32,
    /// Empty until a scene is marked as the fixed dimetric view.
    #[serde(default)]
    pub visual_profile: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_scene_without_a_layout_still_loads() {
        let old = r#"{"id":"m","name":"Old","width":10.0,"height":8.0,"image_file":null,"walls":[],"doors":[],"lights":[]}"#;
        let scene: Scene = serde_json::from_str(old).unwrap();
        assert!(scene.layout.is_none());
        assert!(scene.move_blockers.is_empty());
        assert_eq!(scene.schema_version, 1);
        assert!(scene.visual_profile.is_empty());
    }
}
