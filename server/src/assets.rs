// ------------------------------------------------------------
// Module: assets
// Purpose: Read asset packs from disk. A pack lists terrains,
//          props, walls, and themes. Pictures are optional;
//          a missing file falls back to a flat color.
// ------------------------------------------------------------

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How the client samples a pack's pictures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Filtering {
    Linear,
    Nearest,
}

/// What a prop stops. Sight also stops movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Blocks {
    None,
    Move,
    Sight,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainDef {
    pub id: String,
    pub name: String,
    pub swatch: String,
    pub layer: u8,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecalDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub file: Option<String>,
    pub size: [u32; 2],
    #[serde(default)]
    pub rotate: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub front: Option<String>,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub post: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorDef {
    pub id: String,
    pub name: String,
    pub wall: String,
    #[serde(default)]
    pub front_closed: Option<String>,
    #[serde(default)]
    pub front_open: Option<String>,
    #[serde(default)]
    pub side_closed: Option<String>,
    #[serde(default)]
    pub side_open: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub variants: Vec<String>,
    pub footprint: [u32; 2],
    pub blocks: Blocks,
    #[serde(default)]
    pub anchor: Option<[u32; 2]>,
    #[serde(default)]
    pub mirror: bool,
    #[serde(default = "default_height")]
    pub height: f32,
}

fn default_height() -> f32 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScatterDef {
    pub asset: String,
    pub density: f32,
    pub spacing: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeDef {
    pub id: String,
    pub name: String,
    pub terrain: String,
    #[serde(default)]
    pub wall: Option<String>,
    #[serde(default)]
    pub door: Option<String>,
    #[serde(default)]
    pub scatter: Vec<ScatterDef>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayDef {
    #[serde(default)]
    pub felt: Option<String>,
    #[serde(default)]
    pub wood: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub pixels_per_cell: u32,
    pub filtering: Filtering,
    #[serde(default)]
    pub terrains: Vec<TerrainDef>,
    #[serde(default)]
    pub decals: Vec<DecalDef>,
    #[serde(default)]
    pub walls: Vec<WallDef>,
    #[serde(default)]
    pub doors: Vec<DoorDef>,
    #[serde(default)]
    pub props: Vec<PropDef>,
    #[serde(default)]
    pub themes: Vec<ThemeDef>,
    #[serde(default)]
    pub tray: TrayDef,
    /// Filled in when the pack is served, so the client can join file paths.
    #[serde(default)]
    pub base_url: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Catalog {
    pub packs: Vec<Pack>,
}

impl Catalog {
    pub fn prop(&self, asset: &str) -> Option<&PropDef> {
        let (pack_id, local) = split_asset(asset)?;
        self.packs
            .iter()
            .find(|pack| pack.id == pack_id)
            .and_then(|pack| pack.props.iter().find(|prop| prop.id == local))
    }
}

/// `core/oak` splits into the pack and the local id. A bare id is not addressed.
pub fn split_asset(asset: &str) -> Option<(&str, &str)> {
    let (pack, local) = asset.split_once('/')?;
    if pack.is_empty() || local.is_empty() || local.contains('/') {
        return None;
    }
    Some((pack, local))
}

pub fn load_catalog(assets_dir: &Path) -> Result<Catalog, String> {
    let packs_dir = assets_dir.join("packs");
    if !packs_dir.is_dir() {
        return Ok(Catalog::default());
    }
    let mut packs = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&packs_dir)
        .map_err(|err| format!("could not read {}: {err}", packs_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();
    for dir in entries {
        let manifest = dir.join("pack.json");
        if !manifest.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)
            .map_err(|err| format!("could not read {}: {err}", manifest.display()))?;
        let mut pack = parse_pack(&text)?;
        let folder = dir.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if pack.id != folder {
            return Err(format!(
                "pack {} is in a folder named {folder}",
                pack.id
            ));
        }
        pack.base_url = format!("/packs/{}", pack.id);
        packs.push(pack);
    }
    Ok(Catalog { packs })
}

pub fn parse_pack(text: &str) -> Result<Pack, String> {
    let pack: Pack = serde_json::from_str(text).map_err(|err| format!("the pack manifest is not valid JSON: {err}"))?;
    validate_pack(&pack)?;
    Ok(pack)
}

fn validate_pack(pack: &Pack) -> Result<(), String> {
    if pack.id.is_empty() || !pack.id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        return Err(format!("pack id {:?} is not a plain folder name", pack.id));
    }
    if !(8..=512).contains(&pack.pixels_per_cell) {
        return Err(format!(
            "pack {} has pixelsPerCell {}, and it must be from 8 to 512",
            pack.id, pack.pixels_per_cell
        ));
    }
    let mut ids = HashSet::new();
    for terrain in &pack.terrains {
        remember(&mut ids, &pack.id, "terrain", &terrain.id)?;
        check_swatch(&pack.id, &terrain.swatch)?;
        if terrain.layer > 9 {
            return Err(format!("terrain {}/{} has a layer above 9", pack.id, terrain.id));
        }
    }
    for decal in &pack.decals {
        remember(&mut ids, &pack.id, "decal", &decal.id)?;
        if decal.size[0] == 0 || decal.size[1] == 0 || decal.size[0] > 8 || decal.size[1] > 8 {
            return Err(format!("decal {}/{} has a size outside 1 to 8", pack.id, decal.id));
        }
    }
    for wall in &pack.walls {
        remember(&mut ids, &pack.id, "wall", &wall.id)?;
    }
    for door in &pack.doors {
        remember(&mut ids, &pack.id, "door", &door.id)?;
        if !pack.walls.iter().any(|wall| wall.id == door.wall) {
            return Err(format!("door {}/{} names an unknown wall {}", pack.id, door.id, door.wall));
        }
    }
    for prop in &pack.props {
        remember(&mut ids, &pack.id, "prop", &prop.id)?;
        if prop.footprint[0] == 0 || prop.footprint[1] == 0 || prop.footprint[0] > 12 || prop.footprint[1] > 12 {
            return Err(format!("prop {}/{} has a footprint outside 1 to 12", pack.id, prop.id));
        }
    }
    for theme in &pack.themes {
        remember(&mut ids, &pack.id, "theme", &theme.id)?;
        if !pack.terrains.iter().any(|terrain| terrain.id == theme.terrain) {
            return Err(format!(
                "theme {}/{} names an unknown terrain {}",
                pack.id, theme.id, theme.terrain
            ));
        }
        if let Some(wall) = &theme.wall {
            if !pack.walls.iter().any(|item| &item.id == wall) {
                return Err(format!("theme {}/{} names an unknown wall {wall}", pack.id, theme.id));
            }
        }
        for scatter in &theme.scatter {
            if !known_scatter(pack, &scatter.asset) {
                return Err(format!(
                    "theme {}/{} scatters an unknown asset {}",
                    pack.id, theme.id, scatter.asset
                ));
            }
            if scatter.density < 0.0 || scatter.spacing < 0.0 {
                return Err(format!("theme {}/{} has a negative scatter", pack.id, theme.id));
            }
        }
    }
    Ok(())
}

fn known_scatter(pack: &Pack, asset: &str) -> bool {
    let Some((pack_id, local)) = split_asset(asset) else {
        return false;
    };
    if pack_id != pack.id {
        return false;
    }
    pack.props.iter().any(|prop| prop.id == local) || pack.decals.iter().any(|decal| decal.id == local)
}

fn remember(ids: &mut HashSet<String>, pack: &str, kind: &str, id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains('/') {
        return Err(format!("{kind} in pack {pack} has an empty or slashed id"));
    }
    if !ids.insert(format!("{kind}:{id}")) {
        return Err(format!("pack {pack} repeats {kind} id {id}"));
    }
    Ok(())
}

fn check_swatch(pack: &str, swatch: &str) -> Result<(), String> {
    let hex = swatch.strip_prefix('#').unwrap_or("");
    if hex.len() == 6 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("pack {pack} has a swatch {swatch} that is not #rrggbb"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_pack_loads() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = load_catalog(&path).unwrap();
        assert!(catalog.packs.iter().any(|pack| pack.id == "core" && pack.base_url == "/packs/core"));
        assert_eq!(catalog.prop("core/table").map(|prop| prop.footprint), Some([2, 1]));
    }

    #[test]
    fn an_invalid_manifest_names_the_problem() {
        let bad = r#"{
            "id":"core","name":"Core","pixelsPerCell":128,"filtering":"linear",
            "themes":[{"id":"forest","name":"Forest","terrain":"grass","scatter":[]}]
        }"#;
        let err = parse_pack(bad).unwrap_err();
        assert!(err.contains("unknown terrain"), "{err}");
    }
}
