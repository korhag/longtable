// ------------------------------------------------------------
// Module: ruin_scene
// Purpose: The diagnostic ruin pack and the 12 by 12 scene
//          that proves tiles, objects, stairs, and sight together.
//          The pictures are colored blocks, not production art.
// ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::Path;

    use crate::assets::load_catalog;
    use crate::map::compile::compile;
    use crate::map::layout::{encode_cells, EdgeAxis, EdgePiece, MapLayout, PieceKind, PlacedDecal, PlacedProp, StairLink};
    use crate::map::navigation::check_route;
    use crate::map::Vec2;
    use crate::net::protocol::{ClientMsg, Role};
    use crate::session::permissions::snapshot;
    use crate::session::state::{Table, Viewer};
    use crate::systems::{Character, CharacterKind};

    fn assets_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")
    }

    fn zip_pack(dir: &Path) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let mut files = Vec::new();
        walk(dir, dir, &mut files);
        for (name, bytes) in files {
            writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn walk(root: &Path, dir: &Path, files: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                files.push((name, std::fs::read(path).unwrap()));
            }
        }
    }

    fn scene() -> MapLayout {
        let mut cells = vec![0_u8; 144];
        for y in 0..6 {
            for x in 6..12 {
                cells[y * 12 + x] = 1;
            }
        }
        for y in 6..12 {
            for x in 0..6 {
                cells[y * 12 + x] = 2;
            }
        }
        for y in 6..12 {
            for x in 6..12 {
                cells[y * 12 + x] = 3;
            }
        }
        let mut levels = vec![0_u8; 144];
        levels[2 * 12 + 8] = 1;
        levels[2 * 12 + 9] = 2;
        MapLayout {
            width: 12,
            height: 12,
            terrains: vec!["ruin/grass".into(), "ruin/dirt".into(), "ruin/cobble".into(), "ruin/floor".into()],
            cells: encode_cells(&cells),
            pieces: vec![
                piece("wh1", PieceKind::Wall, 1, 5, EdgeAxis::H),
                piece("door", PieceKind::Door, 2, 5, EdgeAxis::H),
                piece("wh2", PieceKind::Wall, 3, 5, EdgeAxis::H),
                piece("wv1", PieceKind::Wall, 4, 5, EdgeAxis::V),
                piece("wv2", PieceKind::Wall, 4, 6, EdgeAxis::V),
                piece("wv3", PieceKind::Wall, 4, 7, EdgeAxis::V),
            ],
            props: vec![
                prop("oak", "ruin/oak", 3.0, 8.0),
                prop("barrel", "ruin/barrel", 5.0, 3.0),
                prop("table", "ruin/table", 6.0, 8.0),
                prop("hedge", "ruin/hedge", 6.0, 2.0),
            ],
            decals: vec![PlacedDecal { asset: "ruin/leaves".into(), x: 1.5, y: 1.5, scale: 1.0, angle: 0.0 }],
            lights: Vec::new(),
            levels: Some(encode_cells(&levels)),
            seed: Some(7),
            environment: None,
            variants: None,
            stairs: Some(vec![
                StairLink { id: "step".into(), asset: "ruin/stair".into(), x: 7, y: 2, x2: 8, y2: 2 },
                StairLink { id: "rise".into(), asset: "ruin/stair".into(), x: 8, y: 2, x2: 9, y2: 2 },
            ]),
        }
    }

    fn piece(id: &str, kind: PieceKind, x: u32, y: u32, axis: EdgeAxis) -> EdgePiece {
        EdgePiece { id: id.into(), kind, asset: "ruin/wall".into(), x, y, open: false, axis }
    }

    fn prop(id: &str, asset: &str, x: f32, y: f32) -> PlacedProp {
        PlacedProp { id: id.into(), asset: asset.into(), x, y, mirror: false }
    }

    #[test]
    fn the_ruin_pack_imports_and_the_forest_scene_holds() {
        let assets = assets_dir();
        let catalog = load_catalog(&assets).unwrap();
        assert!(catalog.warnings.iter().all(|warning| !warning.contains("ruin")), "{:?}", catalog.warnings);
        assert!(catalog.tile("ruin/grass").is_some());
        assert!(catalog.tile("ruin/leaves").is_some());
        assert!(catalog.object("ruin/oak").is_some());
        assert!(catalog.cast("ruin/bandit").is_some());
        assert!(catalog.cast("ruin/scout").is_some());

        let temp = std::env::temp_dir().join(format!("longtable-ruin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("packs")).unwrap();
        let imported = crate::asset_import::import_bytes(&zip_pack(&assets.join("packs/ruin")), &temp).unwrap();
        assert_eq!(imported.id, "ruin");
        let again = load_catalog(&temp).unwrap();
        assert!(again.warnings.is_empty(), "{:?}", again.warnings);
        let _ = std::fs::remove_dir_all(&temp);

        let layout = scene();
        layout.validate().unwrap();
        check_route(&layout, Vec2::new(7.5, 2.5), Vec2::new(8.5, 2.5)).unwrap();
        check_route(&layout, Vec2::new(8.5, 2.5), Vec2::new(9.5, 2.5)).unwrap();
        check_route(&layout, Vec2::new(8.5, 2.5), Vec2::new(8.5, 3.5)).expect("the square beside the rise is reached by walking down the stair and around");

        let compiled = compile(&layout, &catalog).unwrap();
        assert!(compiled.walls.len() >= 5, "walls run on both axes");
        assert_eq!(compiled.doors.len(), 1);
        assert!(compiled.move_blockers.iter().any(|seg| seg.a.x < 4.2 && seg.a.y > 8.0), "the oak trunk stops walking");
        assert!(compiled.sight.iter().any(|block| (block.top - 0.5).abs() < 0.01), "the hedge is a low sight span");
        assert!(compiled.sight.iter().any(|block| (block.top - 3.0).abs() < 0.01), "the oak keeps its height");
        let barrel = |point: crate::map::Vec2| (point.x - 5.5).hypot(point.y - 3.5) < 1.0;
        assert!(compiled.sight.iter().all(|block| !barrel(block.a) && !barrel(block.b)), "the barrel does not block sight");

        let mut table = Table::new();
        table.characters.push(Character {
            id: "ada".into(),
            name: "Ada".into(),
            kind: CharacterKind::Pc,
            owner_id: None,
            system_id: None,
            numbers: Default::default(),
            texts: Default::default(),
            cast_asset_id: Some("ruin/scout".into()),
        });
        let gm = Viewer { player_id: "gm".into(), role: Role::Gm, view_as: None, session_token: "gm".into() };
        table.publish_built(&gm, "Ruin".into(), layout, compiled, None).unwrap();
        let mut place = |name: &str, x: f32, y: f32, hidden: bool, character: Option<String>, cast: Option<String>| {
            table.command(&gm, ClientMsg::CreateToken {
                name: name.into(),
                x,
                y,
                owner_id: None,
                hidden,
                vision_radius: 12.0,
                grants_vision: name == "Ada",
                color: "#d4a656".into(),
                character_id: character,
                cast_asset: cast,
                facing: Some("east".into()),
            }, None).unwrap();
        };
        place("Ada", 9.5, 2.5, false, Some("ada".into()), None);
        place("Bram", 2.5, 2.5, false, None, Some("ruin/bandit".into()));
        place("Shade", 3.5, 6.5, false, None, Some("ruin/bandit".into()));
        place("Lurker", 10.5, 10.5, true, None, Some("ruin/bandit".into()));
        // North-east of the oak root (4, 9), along the fixed camera's look
        // direction, and outside the trunk. The crown covers this spot on
        // screen. The wall at y = 5 does not cross Ada's line to it.
        place("Wren", 4.52, 8.48, false, None, Some("ruin/bandit".into()));
        assert_eq!(table.tokens.iter().filter(|token| token.cast_asset.as_deref() == Some("ruin/bandit") || token.character_id.as_deref() == Some("ada")).count(), 5);

        let player = Viewer { player_id: "ada-seat".into(), role: Role::Player, view_as: None, session_token: "seat".into() };
        table.players.push(crate::session::state::Player {
            id: "ada-seat".into(),
            name: "Ada".into(),
            role: Role::Player,
            color: "#7eb8a2".into(),
            connected: true,
        });
        table.tokens[0].owner_id = Some("ada-seat".into());
        table.reveal_from_tokens();
        let seen = snapshot(&table, &player);
        assert!(seen.tokens.iter().any(|token| token.name == "Ada" && token.cast_asset.as_deref() == Some("ruin/scout")));
        assert!(seen.tokens.iter().any(|token| token.name == "Bram" && token.cast_asset.as_deref() == Some("ruin/bandit")), "the low hedge does not hide Bram");
        assert!(seen.tokens.iter().any(|token| token.name == "Wren" && token.cast_asset.as_deref() == Some("ruin/bandit")), "the oak trunk does not hide Wren");
        assert!(seen.tokens.iter().all(|token| token.name != "Shade"), "the wall hides Shade");
        assert!(seen.tokens.iter().all(|token| token.name != "Lurker"), "a hidden figure is absent");
        let names = seen.tokens.iter().map(|token| token.name.as_str()).collect::<Vec<_>>();
        assert!(!names.contains(&"Lurker") && !names.contains(&"Shade"), "{names:?}");
        table.players.push(crate::session::state::Player {
            id: "gm".into(),
            name: "Host".into(),
            role: Role::Gm,
            color: "#f4f1ea".into(),
            connected: true,
        });
        let eyes = Viewer { player_id: "gm".into(), role: Role::Gm, view_as: Some("ada-seat".into()), session_token: "gm".into() };
        let through = snapshot(&table, &eyes);
        assert!(through.tokens.iter().any(|token| token.name == "Wren"), "looking through Ada still shows Wren");
        assert!(through.tokens.iter().all(|token| token.name != "Shade" && token.name != "Lurker"), "looking through Ada hides Shade and Lurker");

        let save_dir = std::env::temp_dir().join(format!("longtable-ruin-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&save_dir);
        crate::storage::write(&table, &save_dir).unwrap();
        let mut restored = Table::new();
        assert!(crate::storage::load_file(&mut restored, &save_dir.join("state.json")).unwrap());
        let _ = std::fs::remove_dir_all(&save_dir);
        let layout = {
            let scene = restored.scene.as_ref().unwrap();
            assert_eq!(scene.name, "Ruin");
            assert!(scene.move_blockers.len() >= 8, "a reload keeps the oak and the barrel");
            assert!(scene.sight.iter().any(|block| (block.top - 0.5).abs() < 0.01), "a reload keeps the hedge");
            assert_eq!(scene.layout.as_ref().unwrap().stairs.as_ref().unwrap().len(), 2);
            scene.layout.clone().unwrap()
        };
        let empty = crate::assets::Catalog::default();
        let err = match compile(&layout, &empty) {
            Err(message) => message,
            Ok(_) => panic!("a missing pack must not compile the blockers away"),
        };
        assert!(err.contains("not an object"), "{err}");
        assert!(restored.scene.as_ref().unwrap().move_blockers.len() >= 8, "a missing pack does not erase the saved blockers");
        let again = snapshot(&restored, &player);
        assert!(again.tokens.iter().any(|token| token.name == "Bram"));
        assert!(again.tokens.iter().any(|token| token.name == "Wren"), "a reload still shows the figure behind the oak");
        assert!(again.tokens.iter().all(|token| token.name != "Shade" && token.name != "Lurker"));
        assert_eq!(restored.tokens.len(), 5);
    }
}
