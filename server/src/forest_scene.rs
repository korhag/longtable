// A complete, editable woodland encounter. Loading it creates a new scene;
// existing scenes and their tokens remain on the campaign shelf.
use crate::assets::Catalog;
use crate::map::compile::compile;
use crate::map::drafts::Draft;
use crate::map::layout::{encode_cells, EdgeAxis, EdgePiece, MapLayout, PieceKind, PlacedLight, PlacedProp, StairLink};
use crate::net::protocol::{ClientMsg, Role};
use crate::session::state::{now_ms, SceneShelf, Table, Viewer};
use crate::systems::{Character, CharacterKind};
use uuid::Uuid;

pub const NAME: &str = "The Old Forest Road";
const CABIN_WEST: u32 = 3;
const CABIN_EAST: u32 = 10;
const CABIN_NORTH: u32 = 5;
const CABIN_SOUTH: u32 = 12;
const CABIN_DOOR_X: u32 = 6;

pub fn layout() -> MapLayout {
    let (width, height) = (26_u32, 22_u32);
    let mut cells = vec![0_u8; (width * height) as usize];
    let mut levels = cells.clone();
    for y in 0..height {
        let center = 12.0 + (y as f32 * 0.27).sin() * 2.0;
        for x in 0..width {
            if (x as f32 - center).abs() <= 1.7 { cells[(y * width + x) as usize] = 1; }
            if x >= 18 && y <= 10 { levels[(y * width + x) as usize] = 1; }
            if x >= 22 && y <= 6 { levels[(y * width + x) as usize] = 2; }
        }
    }
    // Worn approaches make the cabin entrance and both stair landings readable.
    for y in 12..14 { for x in 6..15 { cells[(y * width + x) as usize] = 1; } }
    for x in 13..=19 { cells[(8 * width + x) as usize] = 1; }
    for y in 4..=8 { cells[(y * width + 20) as usize] = 1; }
    for x in 20..=23 { cells[(4 * width + x) as usize] = 1; }
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
        id: format!("forest-tree-{i}"), asset: format!("forest/{asset}"), x: *x, y: *y, mirror: false,
    }).collect();
    props.push(PlacedProp { id: "forest-boulder".into(), asset: "forest/boulder".into(), x: 17., y: 14., mirror: false });
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
        props.push(PlacedProp { id: format!("forest-cabin-{id}"), asset: format!("forest/{asset}"), x, y, mirror: false });
    }
    MapLayout {
        width, height, terrains: vec!["forest/forest-floor".into(), "forest/road".into(), "forest/cabin-floor".into()],
        cells: encode_cells(&cells), pieces, props, decals: Vec::new(),
        lights: vec![
            PlacedLight { x: 6.5, y: 8.5, range: 5., color: "#ffad42".into(), kind: crate::map::layout::LightKind::Torch, intensity: 1.4 },
            PlacedLight { x: 19.5, y: 7.5, range: 5., color: "#79c8ff".into(), kind: crate::map::layout::LightKind::Magic, intensity: 1.3 },
        ],
        environment: Some(Default::default()),
        levels: Some(encode_cells(&levels)), seed: Some(7013), variants: None,
        stairs: Some(vec![
            StairLink { id: "forest-lower-stairs".into(), asset: "forest/stairs".into(), x: 17, y: 8, x2: 18, y2: 8 },
            StairLink { id: "forest-upper-stairs".into(), asset: "forest/stairs".into(), x: 21, y: 4, x2: 22, y2: 4 },
        ]),
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
        prepared.characters.push(Character {
            id: character_id.clone(), name: name.into(), kind: if player { CharacterKind::Pc } else { CharacterKind::Adversary },
            owner_id: None, system_id: None, numbers: Default::default(), texts: Default::default(), cast_asset_id: Some(cast.clone()),
        });
        prepared.command(viewer, ClientMsg::CreateToken {
            name: name.into(), x, y, owner_id: None, hidden: false, vision_radius: 10., grants_vision: player,
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
    table.reveal_from_tokens();
    Ok(Draft { id: Uuid::new_v4().to_string(), name: NAME.into(), updated_at: now_ms(), scene_id: Some(scene_id), environment_base: Some(layout.environment.clone().unwrap_or_default()), layout })
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
        assert_eq!(layout.level_at(10., 10.), 0.);
        assert_eq!(layout.level_at(20., 8.), 1.);
        assert_eq!(layout.level_at(24., 4.), 2.);
        check_route(&layout, Vec2::new(17.5, 8.5), Vec2::new(18.5, 8.5)).unwrap();
        check_route(&layout, Vec2::new(21.5, 4.5), Vec2::new(22.5, 4.5)).unwrap();
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
        assert!(!occupied.contains(&(6, 8)), "captain must not stand in furniture");
        for y in 8..CABIN_SOUTH { assert!(!occupied.contains(&(CABIN_DOOR_X, y)), "the door aisle stays clear"); }
        let captain = Vec2::new(6.5, 8.5);
        let doorstep = Vec2::new(6.5, 13.5);
        let mut closed = compiled.walls.clone();
        closed.extend(compiled.doors.iter().map(|door| Segment { a: door.a, b: door.b }));
        assert!(find_route(&layout, doorstep, captain, &closed, &compiled.move_blockers, [1, 1]).is_err(), "closed cabin must not admit a figure");
        let entrance = find_route(&layout, doorstep, captain, &compiled.walls, &compiled.move_blockers, [1, 1]).expect("opening the door gives the captain a clear route outside");
        assert!(entrance.contains(&(CABIN_DOOR_X as i32, CABIN_SOUTH as i32)));
        for destination in [Vec2::new(5.5, 7.5), Vec2::new(8.5, 8.5)] {
            find_route(&layout, captain, destination, &compiled.walls, &compiled.move_blockers, [1, 1]).expect("bedside and work area remain reachable");
        }
        let terraces = find_route(&layout, Vec2::new(11.5, 17.5), Vec2::new(23.5, 4.5), &closed, &compiled.move_blockers, [1, 1]).expect("the party can reach both terraces around scenery");
        for landing in [(17, 8), (18, 8), (21, 4), (22, 4)] {
            assert!(terraces.contains(&landing), "the route must use each stair landing");
        }
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
        assert_eq!(table.scene.as_ref().unwrap().doors.len(), 1);
        let door_id = table.scene.as_ref().unwrap().doors[0].id.clone();
        assert!(!table.scene.as_ref().unwrap().doors[0].open);
        table.command(&gm, ClientMsg::SetDoor { door_id, open: true }, None).unwrap();
        assert!(table.scene.as_ref().unwrap().doors[0].open);
        let second = install(&mut table, &gm, &catalog).unwrap();
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
