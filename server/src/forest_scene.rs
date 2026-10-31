// A complete, editable woodland encounter. Loading it creates a new scene;
// existing scenes and their tokens remain on the campaign shelf.
use crate::assets::Catalog;
use crate::map::compile::compile;
use crate::map::drafts::Draft;
use crate::map::layout::{encode_cells, EdgeAxis, EdgePiece, MapLayout, PieceKind, PlacedLight, PlacedProp, StairLink};
use crate::net::protocol::{ClientMsg, Role};
use crate::session::state::{now_ms, SceneShelf, Table, Viewer};
use crate::systems::{Character, CharacterKind};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const NAME: &str = "The Old Forest Road";
const FOREST_MARGIN: u32 = 5;
const CABIN_WEST: u32 = 3 + FOREST_MARGIN;
const CABIN_EAST: u32 = 10 + FOREST_MARGIN;
const CABIN_NORTH: u32 = 5 + FOREST_MARGIN;
const CABIN_SOUTH: u32 = 12 + FOREST_MARGIN;
const CABIN_DOOR_X: u32 = 6 + FOREST_MARGIN;
const TREE_COUNT: usize = 72;

fn forest_point(value: f32) -> f32 { value + FOREST_MARGIN as f32 }

pub fn layout() -> MapLayout {
    // An extra five cells on every side doubles the original 26 × 22 area,
    // without doubling the distances between the cabin, cast and stairways.
    let (width, height) = (36_u32, 32_u32);
    let mut cells = vec![0_u8; (width * height) as usize];
    let mut levels = cells.clone();
    for y in 0..height {
        let center = forest_point(12.) + ((y as f32 - FOREST_MARGIN as f32) * 0.27).sin() * 2.0;
        for x in 0..width {
            if (x as f32 - center).abs() <= 1.7 { cells[(y * width + x) as usize] = 1; }
            if x >= 18 + FOREST_MARGIN && y <= 10 + FOREST_MARGIN { levels[(y * width + x) as usize] = 1; }
            if x >= 22 + FOREST_MARGIN && y <= 6 + FOREST_MARGIN { levels[(y * width + x) as usize] = 2; }
        }
    }
    // Worn approaches make the cabin entrance and both stair landings readable.
    for y in 12 + FOREST_MARGIN..14 + FOREST_MARGIN { for x in 6 + FOREST_MARGIN..15 + FOREST_MARGIN { cells[(y * width + x) as usize] = 1; } }
    for x in 13 + FOREST_MARGIN..=19 + FOREST_MARGIN { cells[((8 + FOREST_MARGIN) * width + x) as usize] = 1; }
    for y in 4 + FOREST_MARGIN..=8 + FOREST_MARGIN { cells[(y * width + 20 + FOREST_MARGIN) as usize] = 1; }
    for x in 20 + FOREST_MARGIN..=23 + FOREST_MARGIN { cells[((4 + FOREST_MARGIN) * width + x) as usize] = 1; }
    for y in CABIN_NORTH..CABIN_SOUTH {
        for x in CABIN_WEST..CABIN_EAST { cells[(y * width + x) as usize] = 2; }
    }
    let mut pieces = Vec::new();
    for x in CABIN_WEST..CABIN_EAST {
        pieces.push(piece(x, CABIN_NORTH, EdgeAxis::H, false));
        pieces.push(piece(x, CABIN_SOUTH, EdgeAxis::H, x == CABIN_DOOR_X));
    }
    for y in CABIN_NORTH..CABIN_SOUTH {
        pieces.push(piece(CABIN_WEST, y, EdgeAxis::V, false));
        pieces.push(piece(CABIN_EAST, y, EdgeAxis::V, false));
    }
    let trees = [
        ("oak", 1., 1.), ("pine", 5., 1.), ("birch", 9., 2.), ("dead-tree", 15., 2.),
        ("pine", 1., 6.), ("oak", 0., 16.), ("birch", 4., 15.), ("dead-tree", 1., 18.),
        ("oak", 6., 19.), ("pine", 17., 18.), ("birch", 21., 18.), ("dead-tree", 24., 18.),
        ("oak", 21., 12.), ("pine", 24., 8.), ("birch", 19., 2.), ("dead-tree", 23., 2.),
        ("oak", 15., 6.), ("pine", 2., 15.),
    ];
    let mut props: Vec<_> = trees.iter().enumerate().map(|(i, (asset, x, y))| PlacedProp {
        id: format!("forest-tree-{i}"), asset: format!("forest/{asset}"), x: forest_point(*x), y: forest_point(*y), mirror: false,
    }).collect();
    for (id, x, y) in [("forest-boulder", 22., 19.), ("forest-boulder-east", 30., 18.), ("forest-boulder-south", 28., 27.), ("forest-boulder-west", 2., 8.)] {
        props.push(PlacedProp { id: id.into(), asset: "forest/boulder".into(), x, y, mirror: false });
    }
    plant_groves(width, height, &cells, &levels, &mut props);
    // Each furnishing is a reusable library object, with its own blocking footprint.
    // The central aisle stays open from the door to the captain and work desk.
    for (id, asset, x, y) in [
        ("bed", "cabin-bed", 3., 6.),
        ("desk", "cabin-desk", 7., 6.),
        ("rear-shelf", "cabin-shelf", 7., 5.),
        ("front-shelf", "cabin-shelf", 7., 10.),
        ("barrel-a", "cabin-barrel", 3., 10.),
        ("barrel-b", "cabin-barrel", 4., 10.),
    ] {
        props.push(PlacedProp { id: format!("forest-cabin-{id}"), asset: format!("forest/{asset}"), x: forest_point(x), y: forest_point(y), mirror: false });
    }
    MapLayout {
        width, height, terrains: vec!["forest/forest-floor".into(), "forest/road".into(), "forest/cabin-floor".into()],
        cells: encode_cells(&cells), pieces, props, decals: Vec::new(),
        lights: vec![
            PlacedLight { x: forest_point(6.5), y: forest_point(8.5), range: 5., color: "#ffad42".into(), kind: crate::map::layout::LightKind::Torch, intensity: 1.4 },
            PlacedLight { x: forest_point(19.5), y: forest_point(7.5), range: 5., color: "#79c8ff".into(), kind: crate::map::layout::LightKind::Magic, intensity: 1.3 },
        ],
        environment: Some(Default::default()),
        levels: Some(encode_cells(&levels)), seed: Some(7013), variants: None,
        stairs: Some(vec![
            StairLink { id: "forest-lower-stairs".into(), asset: "forest/stairs".into(), x: 17 + FOREST_MARGIN, y: 8 + FOREST_MARGIN, x2: 18 + FOREST_MARGIN, y2: 8 + FOREST_MARGIN },
            StairLink { id: "forest-upper-stairs".into(), asset: "forest/stairs".into(), x: 21 + FOREST_MARGIN, y: 4 + FOREST_MARGIN, x2: 22 + FOREST_MARGIN, y2: 4 + FOREST_MARGIN },
        ]),
    }
}

/// Plant connected groves rather than evenly spaced rows. The same seed always
/// gives the same editable encounter; nothing scatters while the game is running.
fn plant_groves(width: u32, height: u32, cells: &[u8], levels: &[u8], props: &mut Vec<PlacedProp>) {
    let mut reserved = vec![false; (width * height) as usize];
    let mut reserve = |x: i32, y: i32| {
        if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 { reserved[(y as u32 * width + x as u32) as usize] = true; }
    };
    // Keep cobbles, cabin walls/doorway, and party staging space free of trunks.
    for y in 0..height { for x in 0..width { if cells[(y * width + x) as usize] != 0 { reserve(x as i32, y as i32); } } }
    for y in CABIN_NORTH - 1..=CABIN_SOUTH + 1 { for x in CABIN_WEST - 1..=CABIN_EAST + 1 { reserve(x as i32, y as i32); } }
    for (x, y) in [(11, 17), (13, 17), (11, 19), (13, 19), (15, 9), (20, 8), (9, 13)] {
        for dy in -1..=1 { for dx in -1..=1 { reserve(x + FOREST_MARGIN as i32 + dx, y + FOREST_MARGIN as i32 + dy); } }
    }
    // Winding forest-floor lanes offer flanking choices without turning the
    // whole forest into paving. Their side clearings break long sight lines.
    let paths: &[&[(i32, i32)]] = &[
        &[(6, 29), (6, 20), (10, 20), (10, 18), (15, 18)],
        &[(6, 20), (6, 6), (12, 6), (12, 3), (18, 3)],
        &[(18, 21), (28, 21), (28, 24), (32, 24), (32, 29)],
        &[(24, 13), (25, 13), (25, 9), (30, 9), (30, 5), (33, 5)],
    ];
    for path in paths { for segment in path.windows(2) {
        let (mut x, mut y) = segment[0];
        let (end_x, end_y) = segment[1];
        loop {
            for dy in -1..=1 { for dx in -1..=1 { reserve(x + dx, y + dy); } }
            if (x, y) == (end_x, end_y) { break; }
            x += (end_x - x).signum(); y += (end_y - y).signum();
        }
    } }
    // Some former edge trees now sit on a new woodland trail. Move that canopy
    // into a grove instead of silently blocking the expanded walking lanes.
    props.retain(|prop| !prop.id.starts_with("forest-tree-") || (0..2).all(|dy| (0..2).all(|dx| {
        !reserved[((prop.y as u32 + dy) * width + prop.x as u32 + dx) as usize]
    })));
    let mut occupied = vec![false; reserved.len()];
    for prop in props.iter() { for y in prop.y as u32..prop.y as u32 + 2 { for x in prop.x as u32..prop.x as u32 + 2 { occupied[(y * width + x) as usize] = true; } } }
    let groves: [(i32, i32); 9] = [(3, 4), (10, 3), (29, 3), (33, 13), (27, 27), (9, 28), (21, 29), (3, 16), (20, 7)];
    let hash = |x: u32, y: u32| {
        let n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263)).wrapping_add(7013);
        (n ^ (n >> 13)).wrapping_mul(1274126177)
    };
    let mut candidates: Vec<_> = (0..height - 1).flat_map(|y| (0..width - 1).map(move |x| (x, y))).collect();
    candidates.sort_by_key(|&(x, y)| {
        let distance = groves.iter().map(|&(cx, cy)| (x as i32 + 1 - cx).pow(2) + (y as i32 + 1 - cy).pow(2)).min().unwrap();
        (distance * 3 + (hash(x, y) % 23) as i32, hash(y, x))
    });
    let mut trees = props.iter().filter(|prop| prop.id.starts_with("forest-tree-")).count();
    for (x, y) in candidates {
        if trees >= TREE_COUNT { break; }
        let footprint = [y * width + x, y * width + x + 1, (y + 1) * width + x, (y + 1) * width + x + 1];
        if footprint.iter().any(|&cell| reserved[cell as usize] || occupied[cell as usize] || levels[cell as usize] != levels[footprint[0] as usize]) { continue; }
        let asset = ["oak", "pine", "birch", "oak", "pine", "dead-tree"][(hash(x, y) % 6) as usize];
        props.push(PlacedProp { id: format!("forest-tree-grove-{x}-{y}"), asset: format!("forest/{asset}"), x: x as f32, y: y as f32, mirror: false });
        for cell in footprint { occupied[cell as usize] = true; }
        trees += 1;
    }
}

fn piece(x: u32, y: u32, axis: EdgeAxis, door: bool) -> EdgePiece {
    EdgePiece {
        id: format!("forest-cabin-{}-{x}-{y}", if axis == EdgeAxis::H { "h" } else { "v" }),
        kind: if door { PieceKind::Door } else { PieceKind::Wall },
        asset: if door { "forest/cabin-door" } else { "forest/cabin-wall" }.into(),
        x, y, axis, open: false,
    }
}

/// Prepare the encounter off-table first so invalid/missing assets do not replace a live scene.
pub fn install(table: &mut Table, viewer: &Viewer, catalog: &Catalog) -> Result<Draft, String> {
    if viewer.role != Role::Gm { return Err("only the GM can load a sample scene".into()); }
    let layout = layout();
    layout.validate()?;
    for asset in &layout.terrains {
        if catalog.tile(asset).is_none() { return Err(format!("Install the Forest pack first: missing {asset}")); }
    }
    for asset in layout.pieces.iter().map(|p| &p.asset).chain(layout.props.iter().map(|p| &p.asset)).chain(layout.stairs.iter().flatten().map(|s| &s.asset)) {
        if catalog.object(asset).is_none() { return Err(format!("Install the Forest pack first: missing {asset}")); }
    }
    let mut prepared = Table::new();
    prepared.note_cast_footprints(catalog);
    prepared.publish_built(viewer, NAME.into(), layout.clone(), compile(&layout, catalog)?, None)?;
    for (asset, name, x, y, player) in [
        ("human", "Human · Rowan", 11.5, 17.5, true),
        ("elf", "Elf · Sylva", 13.5, 17.5, true),
        ("dwarf", "Dwarf · Brenn", 11.5, 19.5, true),
        ("orc", "Orc · Korr", 13.5, 19.5, true),
        ("bandit-scout", "Bandit scout", 15.5, 9.5, false),
        ("bandit-archer", "Bandit archer", 20.5, 8.5, false),
        ("bandit-brute", "Bandit brute", 9.5, 13.5, false),
        ("bandit-captain", "Bandit captain", 6.5, 8.5, false),
    ] {
        let cast = format!("forest/{asset}");
        if catalog.cast(&cast).is_none() { return Err(format!("Install the Forest pack first: missing {cast}")); }
        let character_id = Uuid::new_v4().to_string();
        let (numbers, texts) = showroom_stats(asset, player);
        prepared.characters.push(Character {
            id: character_id.clone(), name: name.into(), kind: if player { CharacterKind::Pc } else { CharacterKind::Adversary },
            owner_id: None, system_id: Some("daggerheart".into()), numbers, texts, cast_asset_id: Some(cast.clone()),
        });
        prepared.command(viewer, ClientMsg::CreateToken {
            name: name.into(), x: forest_point(x), y: forest_point(y), owner_id: None, hidden: false, vision_radius: 10., grants_vision: player,
            color: if player { "#c9b07a" } else { "#b75b45" }.into(), character_id: Some(character_id),
            cast_asset: Some(cast), facing: Some(if player { "north" } else { "south" }.into()),
        }, None)?;
    }
    // A lobby can contain figures without a map. Preserve those too.
    let previous_explored = table.explored.clone();
    let loose = if table.scene.is_none() && !table.tokens.is_empty() { Some(std::mem::take(&mut table.tokens)) } else { None };
    let scene_id = match table.publish_built(viewer, NAME.into(), layout.clone(), compile(&layout, catalog)?, None) {
        Ok(id) => id,
        Err(err) => { if let Some(tokens) = loose { table.tokens = tokens; } return Err(err); }
    };
    if let Some(tokens) = loose {
        table.scenes.push(SceneShelf { id: Uuid::new_v4().to_string(), name: "Before the forest sample".into(), scene: None, tokens, explored: previous_explored });
    }
    table.characters.extend(prepared.characters);
    table.tokens = prepared.tokens;
    // A new campaign can try the encounter immediately; an existing rules choice is preserved.
    if table.campaign.system_id.is_none() { table.campaign.system_id = Some("daggerheart".into()); }
    table.reveal_from_tokens();
    Ok(Draft { id: Uuid::new_v4().to_string(), name: NAME.into(), updated_at: now_ms(), scene_id: Some(scene_id), environment_base: Some(layout.environment.clone().unwrap_or_default()), layout })
}

/// Editable demonstration builds, not published SRD adversary stat blocks.
fn showroom_stats(asset: &str, player: bool) -> (BTreeMap<String, i32>, BTreeMap<String, String>) {
    let mut numbers: BTreeMap<String, i32> = [
        ("hp", 0), ("hpmax", 6), ("stress", 0), ("stressmax", 6),
        ("minor", 1), ("major", 7), ("severe", 13), ("tier", 1),
    ].into_iter().map(|(key, value)| (key.into(), value)).collect();
    let mut texts = BTreeMap::from([("notes".into(), "Editable showroom character for trying movement, facing, attacks and threshold damage.".into())]);
    if player {
        numbers.extend([
            ("agility", 1), ("strength", 1), ("finesse", 0), ("instinct", 0),
            ("presence", 0), ("knowledge", 0), ("evasion", 10), ("hope", 2),
            ("armor", 0), ("armorscore", 0), ("proficiency", 1), ("level", 1),
        ].into_iter().map(|(key, value)| (key.into(), value)));
        let (ancestry, weapon, damage, trait_name, trait_key, range) = match asset {
            "elf" => ("Elf", "Longbow", "1d8+2 phy", "Agility", "agility", "Far"),
            "dwarf" => ("Dwarf", "Warhammer", "1d10+2 phy", "Strength", "strength", "Melee"),
            "orc" => ("Orc", "Battleaxe", "1d10+2 phy", "Strength", "strength", "Melee"),
            _ => ("Human", "Longsword", "1d8+2 phy", "Agility", "agility", "Melee"),
        };
        numbers.insert(trait_key.into(), 2);
        texts.extend([
            ("ancestry".into(), ancestry.into()),
            ("primary".into(), format!("{weapon} · {trait_name} · {range}")),
            ("attackrange".into(), range.into()), ("damage".into(), damage.into()),
        ]);
    } else {
        let (major, severe, hpmax, difficulty, attack, kind, weapon, range, damage) = match asset {
            "bandit-archer" => (6, 11, 3, 12, 1, "Ranged", "Shortbow", "Far", "1d8+2 phy"),
            "bandit-brute" => (9, 17, 6, 13, 2, "Bruiser", "Heavy club", "Melee", "1d12+2 phy"),
            "bandit-captain" => (10, 18, 6, 14, 2, "Leader", "Sabre", "Melee", "1d10+2 phy"),
            _ => (7, 12, 3, 11, 1, "Skulk", "Daggers", "Melee", "1d8+1 phy"),
        };
        numbers.extend([
            ("major", major), ("severe", severe), ("hpmax", hpmax),
            ("difficulty", difficulty), ("attack", attack), ("stressmax", 3),
        ].into_iter().map(|(key, value)| (key.into(), value)));
        texts.extend([
            ("advtype".into(), kind.into()), ("attackname".into(), weapon.into()),
            ("attackrange".into(), range.into()), ("atkdamage".into(), damage.into()),
        ]);
    }
    (numbers, texts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::navigation::{check_route, find_route};
    use crate::map::{Segment, Vec2};

    #[test]
    fn every_showroom_cast_has_four_distinct_transparent_views() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let mut all_pictures = std::collections::HashSet::new();
        for id in ["human", "elf", "dwarf", "orc", "bandit-scout", "bandit-archer", "bandit-brute", "bandit-captain"] {
            let actor = catalog.cast(&format!("forest/{id}")).expect("showroom cast is installed");
            let mut paths = std::collections::HashSet::new();
            for (direction, frame) in [("north", &actor.views.north), ("east", &actor.views.east), ("south", &actor.views.south), ("west", &actor.views.west)] {
                assert!(paths.insert(&frame.file), "{id}/{direction} must not alias another direction's file");
                let path = assets.join("packs/forest").join(&frame.file);
                let picture = image::open(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display())).to_rgba8();
                assert_eq!([picture.width(), picture.height()], frame.size_px, "{id}/{direction} dimensions");
                assert!(frame.pivot_px[0] < picture.width() && frame.pivot_px[1] < picture.height(), "{id}/{direction} pivot must be inside the frame");
                assert!(picture.pixels().any(|p| p[3] == 0), "{id}/{direction} needs transparent space around the silhouette");
                assert!(picture.pixels().any(|p| p[3] >= 128), "{id}/{direction} must contain a visible figure");
                // Compare decoded pixels, so copying/re-encoding a front pose into four files cannot pass.
                assert!(all_pictures.insert((picture.width(), picture.height(), picture.into_raw())), "{id}/{direction} must contain distinct artwork");
            }
        }
        assert_eq!(all_pictures.len(), 32);
    }

    #[test]
    fn showroom_animation_atlases_have_all_drawn_frames_without_cell_clipping() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let mut sheets = std::collections::HashSet::new();
        let mut frame_count = 0;
        for id in ["human", "elf", "dwarf", "orc", "bandit-scout", "bandit-archer", "bandit-brute", "bandit-captain"] {
            let cast = catalog.cast(&format!("forest/{id}")).unwrap();
            let animation = cast.animations.as_ref().expect("each showroom cast has drawn animation sheets");
            for (view, still) in [
                (&animation.views.north, &cast.views.north), (&animation.views.east, &cast.views.east),
                (&animation.views.south, &cast.views.south), (&animation.views.west, &cast.views.west),
            ] {
                let path = &view.file;
                assert!(sheets.insert(path.clone()), "directional sheets must be distinct");
                let picture = image::open(assets.join("packs/forest").join(path)).unwrap().to_rgba8();
                assert_eq!([picture.width(), picture.height()], [1024, 1536]);
                let opaque_height = |image: &image::RgbaImage| {
                    let rows: Vec<_> = image.enumerate_pixels().filter(|(_, _, pixel)| pixel[3] >= 128).map(|(_, y, _)| y).collect();
                    (rows.iter().max().unwrap() - rows.iter().min().unwrap() + 1) as f32
                };
                let reference = image::open(assets.join("packs/forest").join(&still.file)).unwrap().to_rgba8();
                let idle_height: f32 = (0..4).map(|column| opaque_height(&image::imageops::crop_imm(&picture, column * 256, 0, 256, 256).to_image())).sum::<f32>() / 4.;
                let source_height = opaque_height(&reference) / cast.pixels_per_world_unit as f32;
                let drawn_height = idle_height / view.pixels_per_world_unit.unwrap_or(animation.pixels_per_world_unit as f32);
                assert!((source_height - drawn_height).abs() <= source_height * 0.1,
                    "{path}: drawn body must keep its original world scale ({source_height} vs {drawn_height})");
                for row in 0..6 {
                    let mut poses = std::collections::HashSet::new();
                    for column in 0..4 {
                        let cell = image::imageops::crop_imm(&picture, column * 256, row * 256, 256, 256).to_image();
                        assert!(cell.pixels().any(|pixel| pixel[3] >= 128), "{path} frame {row}/{column} is empty");
                        assert!(cell.enumerate_pixels().all(|(x, y, pixel)| {
                            pixel[3] < 128 || (x > 1 && x < 254 && y > 1 && y < 254)
                        }), "{path} frame {row}/{column} crosses its cell boundary");
                        assert!(poses.insert(cell.into_raw()), "{path} action {row} repeats the same picture");
                        frame_count += 1;
                    }
                }
            }
        }
        assert_eq!(sheets.len(), 32);
        assert_eq!(frame_count, 768);
    }

    #[test]
    fn showroom_figures_turn_in_place_and_keep_their_facing_after_reload() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let mut table = Table::new();
        let gm = Viewer { player_id: "gm".into(), role: Role::Gm, view_as: None, session_token: "gm".into() };
        install(&mut table, &gm, &catalog).unwrap();
        let initial: Vec<_> = table.tokens.iter().map(|token| (token.id.clone(), token.x, token.y, token.cast_asset.clone())).collect();
        let directions = ["north", "east", "south", "west"];
        for (index, (id, x, y, cast)) in initial.iter().enumerate() {
            // Every adversary and adventurer exercises every view, ending on different facings.
            for step in 0..4 {
                let facing = directions[(step + index) % 4];
                table.command(&gm, ClientMsg::UpdateToken {
                    token_id: id.clone(), name: None, owner_id: None, clear_owner: false,
                    hidden: None, vision_radius: None, grants_vision: None, color: None,
                    character_id: None, clear_character: false, set_cast_asset: false,
                    cast_asset: None, facing: Some(facing.into()),
                }, None).unwrap();
                let token = table.tokens.iter().find(|token| &token.id == id).unwrap();
                assert_eq!(token.facing, facing);
                assert_eq!((token.x, token.y), (*x, *y), "turning must not move a showroom figure");
                assert_eq!(&token.cast_asset, cast, "turning must keep the chosen character art");
            }
        }
        let dir = std::env::temp_dir().join(format!("longtable-forest-facing-{}", Uuid::new_v4()));
        crate::storage::write(&table, &dir).unwrap();
        let mut restored = Table::new();
        crate::storage::load_file(&mut restored, &dir.join("state.json")).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        for token in &table.tokens {
            let loaded = restored.tokens.iter().find(|loaded| loaded.id == token.id).unwrap();
            assert_eq!(loaded.facing, token.facing);
            assert_eq!(loaded.cast_asset, token.cast_asset);
            assert_eq!((loaded.x, loaded.y), (token.x, token.y));
        }
    }

    #[test]
    fn forest_has_three_connected_tiers_and_an_enclosed_cabin() {
        let layout = layout();
        layout.validate().unwrap();
        assert_eq!(layout.level_at(forest_point(10.), forest_point(10.)), 0.);
        assert_eq!(layout.level_at(forest_point(20.), forest_point(8.)), 1.);
        assert_eq!(layout.level_at(forest_point(24.), forest_point(4.)), 2.);
        check_route(&layout, Vec2::new(forest_point(17.5), forest_point(8.5)), Vec2::new(forest_point(18.5), forest_point(8.5))).unwrap();
        check_route(&layout, Vec2::new(forest_point(21.5), forest_point(4.5)), Vec2::new(forest_point(22.5), forest_point(4.5))).unwrap();
        assert_eq!(layout.pieces.len(), 28);
        assert_eq!(layout.pieces.iter().filter(|p| p.kind == PieceKind::Door).count(), 1);
    }

    #[test]
    fn furnished_cabin_has_a_clear_aisle_and_an_operable_entrance() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let layout = layout();
        let compiled = compile(&layout, &catalog).unwrap();
        let furnishings: Vec<_> = layout.props.iter().filter(|prop| prop.id.starts_with("forest-cabin-")).collect();
        assert_eq!(furnishings.len(), 6);
        for (asset, count) in [("cabin-bed", 1), ("cabin-desk", 1), ("cabin-shelf", 2), ("cabin-barrel", 2)] {
            assert_eq!(furnishings.iter().filter(|prop| prop.asset == format!("forest/{asset}")).count(), count);
        }
        let mut occupied = std::collections::HashSet::new();
        for prop in furnishings {
            let object = catalog.object(&prop.asset).expect("furniture is available in the library");
            assert!(object.blocks_movement, "{} must be an obstacle", prop.asset);
            for y in prop.y as u32..prop.y as u32 + object.footprint_cells[1] {
                for x in prop.x as u32..prop.x as u32 + object.footprint_cells[0] {
                    assert!((CABIN_WEST..CABIN_EAST).contains(&x) && (CABIN_NORTH..CABIN_SOUTH).contains(&y));
                    assert!(occupied.insert((x, y)), "furnishings overlap at {x},{y}");
                }
            }
        }
        assert!(!occupied.contains(&(6 + FOREST_MARGIN, 8 + FOREST_MARGIN)), "captain must not stand in furniture");
        for y in 8 + FOREST_MARGIN..CABIN_SOUTH { assert!(!occupied.contains(&(CABIN_DOOR_X, y)), "the door aisle stays clear"); }
        let captain = Vec2::new(forest_point(6.5), forest_point(8.5));
        let doorstep = Vec2::new(forest_point(6.5), forest_point(13.5));
        let mut closed = compiled.walls.clone();
        closed.extend(compiled.doors.iter().map(|door| Segment { a: door.a, b: door.b }));
        assert!(find_route(&layout, doorstep, captain, &closed, &compiled.move_blockers, [1, 1]).is_err(), "closed cabin must not admit a figure");
        let entrance = find_route(&layout, doorstep, captain, &compiled.walls, &compiled.move_blockers, [1, 1]).expect("opening the door gives the captain a clear route outside");
        assert!(entrance.contains(&(CABIN_DOOR_X as i32, CABIN_SOUTH as i32)));
        for destination in [Vec2::new(forest_point(5.5), forest_point(7.5)), Vec2::new(forest_point(8.5), forest_point(8.5))] {
            find_route(&layout, captain, destination, &compiled.walls, &compiled.move_blockers, [1, 1]).expect("bedside and work area remain reachable");
        }
        let terraces = find_route(&layout, Vec2::new(forest_point(11.5), forest_point(17.5)), Vec2::new(forest_point(23.5), forest_point(4.5)), &closed, &compiled.move_blockers, [1, 1]).expect("the party can reach both terraces around scenery");
        for landing in [(17, 8), (18, 8), (21, 4), (22, 4)] {
            assert!(terraces.contains(&(landing.0 + FOREST_MARGIN as i32, landing.1 + FOREST_MARGIN as i32)), "the route must use each stair landing");
        }
    }

    #[test]
    fn expanded_woodland_has_dense_groves_and_accessible_clearings() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let layout = layout();
        let compiled = compile(&layout, &catalog).expect("all woodland objects must fit on a single tier");
        assert_eq!((layout.width, layout.height), (36, 32));
        assert!((26 * 22 * 2..26 * 22 * 2 + 20).contains(&(layout.width * layout.height)));
        let trees: Vec<_> = layout.props.iter().filter(|prop| prop.id.starts_with("forest-tree-")).collect();
        assert_eq!(trees.len(), TREE_COUNT);
        assert!(trees.len() as f32 / (layout.width * layout.height) as f32 > 18. / (26. * 22.), "tree density increases with the area");
        assert_eq!(layout.props.iter().filter(|prop| prop.asset == "forest/boulder").count(), 4);
        let mut occupied = std::collections::HashSet::new();
        for prop in &layout.props {
            let object = catalog.object(&prop.asset).unwrap();
            for y in prop.y as u32..prop.y as u32 + object.footprint_cells[1] {
                for x in prop.x as u32..prop.x as u32 + object.footprint_cells[0] {
                    assert!(occupied.insert((x, y)), "{} overlaps another prop at {x},{y}", prop.id);
                }
            }
        }
        let party = Vec2::new(forest_point(11.5), forest_point(17.5));
        let mut closed = compiled.walls.clone();
        closed.extend(compiled.doors.iter().map(|door| Segment { a: door.a, b: door.b }));
        for destination in [
            Vec2::new(6.5, 29.5), Vec2::new(6.5, 6.5), Vec2::new(18.5, 3.5),
            Vec2::new(32.5, 24.5), Vec2::new(32.5, 29.5), Vec2::new(32.5, 5.5),
            Vec2::new(forest_point(6.5), forest_point(13.5)),
            Vec2::new(forest_point(15.5), forest_point(9.5)),
            Vec2::new(forest_point(20.5), forest_point(8.5)),
            Vec2::new(forest_point(9.5), forest_point(13.5)),
        ] {
            find_route(&layout, party, destination, &closed, &compiled.move_blockers, [1, 1])
                .unwrap_or_else(|error| panic!("clearing {},{} must remain reachable: {error}", destination.x, destination.y));
        }
        assert!(compiled.sight.len() >= TREE_COUNT * 4, "the denser forest must create actual sight blockers");
        let tree = trees.iter().find(|tree| tree.x >= 2. && tree.x + 4. < layout.width as f32 && tree.y > 16.).unwrap();
        let (x, y) = (tree.x + 1., tree.y + 1.);
        assert!(crate::vision::sees_actor(x - 2., y, 10., x + 2., y, &layout, &[]));
        assert!(!crate::vision::sees_actor(x - 2., y, 10., x + 2., y, &layout, &compiled.sight), "a grove must hide a figure behind its trunks");
        assert_eq!(serde_json::to_value(&layout.props).unwrap(), serde_json::to_value(super::layout().props).unwrap(), "groves should be stable every time the sample loads");
    }

    #[test]
    fn missing_assets_and_player_requests_preserve_the_live_scene() {
        let mut table = Table::new();
        let gm = Viewer { player_id: "gm".into(), role: Role::Gm, view_as: None, session_token: "gm".into() };
        assert!(install(&mut table, &gm, &Catalog::default()).unwrap_err().contains("Forest pack"));
        assert!(table.scene.is_none());
        let player = Viewer { role: Role::Player, ..gm };
        assert!(install(&mut table, &player, &Catalog::default()).unwrap_err().contains("only the GM"));
    }

    #[test]
    fn forest_sample_loads_all_cast_and_keeps_the_previous_encounter() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let catalog = crate::assets::load_catalog(&assets).unwrap();
        let pack = catalog.packs.iter().find(|pack| pack.id == "forest").unwrap();
        let theme = pack.themes.iter().find(|theme| theme.id == "woodland").unwrap();
        assert!(catalog.tile(&format!("{}/{}", pack.id, theme.terrain)).is_some());
        for asset in [&theme.wall, &theme.door].into_iter().flatten() {
            assert!(catalog.object(&format!("{}/{}", pack.id, asset)).is_some());
        }
        let mut table = Table::new();
        let gm = Viewer { player_id: "gm".into(), role: Role::Gm, view_as: None, session_token: "gm".into() };
        let first = install(&mut table, &gm, &catalog).unwrap();
        assert_eq!(table.tokens.len(), 8);
        assert_eq!(table.characters.iter().filter(|c| c.kind == CharacterKind::Pc).count(), 4);
        assert_eq!(table.campaign.system_id.as_deref(), Some("daggerheart"));
        for character in &table.characters {
            assert_eq!(character.numbers["hp"], 0, "HP records marked slots, not remaining health");
            assert!(character.numbers["hpmax"] >= 3);
            assert!(character.numbers["major"] < character.numbers["severe"]);
            let key = if character.kind == CharacterKind::Pc { "damage" } else { "atkdamage" };
            assert!(!character.texts[key].is_empty());
        }
        assert_eq!(table.scene.as_ref().unwrap().doors.len(), 1);
        let door_id = table.scene.as_ref().unwrap().doors[0].id.clone();
        assert!(!table.scene.as_ref().unwrap().doors[0].open);
        table.command(&gm, ClientMsg::SetDoor { door_id, open: true }, None).unwrap();
        assert!(table.scene.as_ref().unwrap().doors[0].open);
        table.campaign.system_id = Some("pathfinder2e".into());
        let second = install(&mut table, &gm, &catalog).unwrap();
        assert_eq!(table.campaign.system_id.as_deref(), Some("pathfinder2e"), "loading a sample must preserve a chosen campaign system");
        assert_ne!(first.scene_id, second.scene_id);
        assert_eq!(table.scenes.len(), 1);
        assert_eq!(table.scenes[0].tokens.len(), 8);
        assert_eq!(table.tokens.len(), 8);
        // The sample remains playable after a real save/reload, including the shelved room's open door.
        let dir = std::env::temp_dir().join(format!("longtable-forest-save-{}", Uuid::new_v4()));
        crate::storage::write(&table, &dir).unwrap();
        let mut restored = Table::new();
        crate::storage::load_file(&mut restored, &dir.join("state.json")).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(restored.tokens.len(), 8);
        assert_eq!(restored.characters.len(), 16);
        assert_eq!(restored.scenes.len(), 1);
        assert!(restored.scenes[0].scene.as_ref().unwrap().doors[0].open);
        let saved_layout = restored.scene.as_ref().unwrap().layout.as_ref().unwrap();
        assert_eq!(saved_layout.stairs.as_ref().unwrap().len(), 2);
        compile(saved_layout, &catalog).unwrap();
    }
}
