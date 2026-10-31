// ------------------------------------------------------------
// Module: compile
// Purpose: Turn an editor layout into the walls, doors, and
//          movement blockers the live table already understands.
//          Walking uses movement blockers. Sight uses finite
//          spans. An object never becomes a wall.
// ------------------------------------------------------------

use crate::assets::{Blocks, Catalog};
use crate::map::layout::{EdgeAxis, MapLayout, PieceKind};
use crate::map::{Door, Light, Segment, SightBlock, Vec2};

pub struct Compiled {
    pub walls: Vec<Segment>,
    pub doors: Vec<Door>,
    pub move_blockers: Vec<Segment>,
    pub lights: Vec<Light>,
    pub sight: Vec<SightBlock>,
}

pub fn compile(layout: &MapLayout, catalog: &Catalog) -> Result<Compiled, String> {
    let mut walls = Vec::new();
    let mut doors = Vec::new();
    let mut move_blockers = Vec::new();
    let mut lights = Vec::new();
    let mut sight = Vec::new();

    for piece in &layout.pieces {
        let (a, b) = edge(piece.x, piece.y, piece.axis);
        match piece.kind {
            PieceKind::Wall => walls.push(Segment { a, b }),
            PieceKind::Door => doors.push(Door {
                id: piece.id.clone(),
                a,
                b,
                open: piece.open,
            }),
        }
    }

    let mut unfit = Vec::new();
    for prop in &layout.props {
        if let Some(object) = catalog.object(&prop.asset) {
            if object.kind == "stairs" || object.placement == "connection" {
                continue;
            }
            if prop.mirror {
                return Err(format!("{} cannot be mirrored. Upright art needs its own picture.", object.name));
            }
            if let Some(problem) = support_problem(layout, prop.x, prop.y, object.footprint_cells) {
                unfit.push(format!("{} at {} {}", object.name, place(prop.x, prop.y), problem));
                continue;
            }
            place_object(&mut move_blockers, &mut sight, layout, prop.x, prop.y, object);
            continue;
        }
        let Some(def) = catalog.prop(&prop.asset) else {
            return Err(format!("{} is not an object in the catalog, so its blocker was not skipped", prop.asset));
        };
        if prop.mirror && !def.mirror {
            return Err(format!("{} cannot be mirrored. Upright art needs its own picture.", def.name));
        }
        if let Some(problem) = support_problem(layout, prop.x, prop.y, def.footprint) {
            unfit.push(format!("{} at {} {}", def.name, place(prop.x, prop.y), problem));
            continue;
        }
        let footprint = rectangle(prop.x, prop.y, def.footprint[0] as f32, def.footprint[1] as f32);
        match def.blocks {
            Blocks::Sight => walls.extend(footprint),
            Blocks::Move => move_blockers.extend(footprint),
            Blocks::None => {}
        }
    }
    if !unfit.is_empty() {
        let move_them = if unfit.len() == 1 { "Move it" } else { "Move each one" };
        return Err(format!(
            "{}. {move_them} so the whole object sits on one level, then put the map on the table again.",
            unfit.join("; ")
        ));
    }

    for light in &layout.lights {
        lights.push(Light {
            x: light.x,
            y: light.y,
            range: light.range,
            color: light.color.clone(),
            kind: light.kind,
            intensity: light.intensity,
        });
    }

    Ok(Compiled { walls, doors, move_blockers, lights, sight })
}

/// Movement and sight are separate. A vision span never becomes a wall.
fn place_object(
    move_blockers: &mut Vec<Segment>,
    sight: &mut Vec<SightBlock>,
    layout: &MapLayout,
    x: f32,
    y: f32,
    object: &crate::assets::ObjectDef,
) {
    let origin = object.origin();
    let root_x = x + origin[0];
    let root_y = y + origin[1];
    let movement = polygon_or_footprint(x, y, object.footprint_cells, &object.collision_polygon, root_x, root_y);
    if object.blocks_movement {
        move_blockers.extend(edges(&movement));
    }
    if !object.blocks_vision {
        return;
    }
    let volume = object.vision_volumes.iter().find(|volume| !volume.polygon.is_empty());
    let vision = volume.map(|volume| shift(&volume.polygon, root_x, root_y)).unwrap_or_else(|| movement.clone());
    let base = layout.level_at(root_x, root_y) + volume.map(|volume| volume.base_height).unwrap_or(0.0);
    let top = base + volume.map(|volume| volume.height).unwrap_or(DEFAULT_SIGHT_HEIGHT);
    for edge in edges(&vision) {
        sight.push(SightBlock { a: edge.a, b: edge.b, base, top });
    }
}

const DEFAULT_SIGHT_HEIGHT: f32 = 8.0;

fn place(x: f32, y: f32) -> String {
    format!("{}, {}", trim(x), trim(y))
}

fn trim(value: f32) -> String {
    if value.fract().abs() < 0.001 { format!("{}", value as i32) } else { format!("{value}") }
}

/// The footprint is the support. A stair is allowed to join two heights.
/// Anything else must sit inside the map on one tier.
fn support_problem(layout: &MapLayout, x: f32, y: f32, footprint: [u32; 2]) -> Option<&'static str> {
    let width = footprint[0].max(1);
    let depth = footprint[1].max(1);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let mut level: Option<f32> = None;
    for dy in 0..depth {
        for dx in 0..width {
            let cx = x0 + dx as i32;
            let cy = y0 + dy as i32;
            if cx < 0 || cy < 0 || cx >= layout.width as i32 || cy >= layout.height as i32 {
                return Some("does not fit on the map");
            }
            let here = layout.level_at(cx as f32 + 0.5, cy as f32 + 0.5);
            if let Some(previous) = level {
                if (previous - here).abs() > 0.01 {
                    return Some("stands on two heights");
                }
            } else {
                level = Some(here);
            }
        }
    }
    None
}

fn polygon_or_footprint(x: f32, y: f32, footprint: [u32; 2], polygon: &[[f32; 2]], root_x: f32, root_y: f32) -> Vec<[f32; 2]> {
    if polygon.is_empty() {
        let east = x + footprint[0] as f32;
        let south = y + footprint[1] as f32;
        vec![[x, y], [east, y], [east, south], [x, south]]
    } else {
        shift(polygon, root_x, root_y)
    }
}

fn shift(polygon: &[[f32; 2]], root_x: f32, root_y: f32) -> Vec<[f32; 2]> {
    polygon.iter().map(|point| [root_x + point[0], root_y + point[1]]).collect()
}

fn edges(polygon: &[[f32; 2]]) -> Vec<Segment> {
    let mut segments = Vec::new();
    if polygon.len() < 2 {
        return segments;
    }
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        segments.push(Segment { a: Vec2::new(a[0], a[1]), b: Vec2::new(b[0], b[1]) });
    }
    segments
}

fn edge(x: u32, y: u32, axis: EdgeAxis) -> (Vec2, Vec2) {
    let x = x as f32;
    let y = y as f32;
    match axis {
        EdgeAxis::H => (Vec2::new(x, y), Vec2::new(x + 1.0, y)),
        EdgeAxis::V => (Vec2::new(x, y), Vec2::new(x, y + 1.0)),
    }
}

fn rectangle(x: f32, y: f32, width: f32, depth: f32) -> Vec<Segment> {
    let east = x + width;
    let south = y + depth;
    vec![
        Segment { a: Vec2::new(x, y), b: Vec2::new(east, y) },
        Segment { a: Vec2::new(east, y), b: Vec2::new(east, south) },
        Segment { a: Vec2::new(east, south), b: Vec2::new(x, south) },
        Segment { a: Vec2::new(x, south), b: Vec2::new(x, y) },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::parse_pack;
    use crate::map::layout::{EdgePiece, MapLayout, PlacedProp};

    fn catalog() -> Catalog {
        let text = r#"{
            "id":"core","name":"Core","pixelsPerCell":128,"filtering":"linear",
            "props":[
                {"id":"table","name":"Table","footprint":[2,1],"blocks":"move","height":0.75},
                {"id":"pillar","name":"Pillar","footprint":[1,1],"blocks":"sight","height":2}
            ]
        }"#;
        Catalog { packs: vec![parse_pack(text).unwrap()], ..Catalog::default() }
    }

    fn layout() -> MapLayout {
        MapLayout {
            width: 10,
            height: 10,
            terrains: vec!["core/stone".into()],
            cells: crate::map::layout::encode_cells(&vec![0_u8; 100]),
            pieces: vec![
                EdgePiece {
                    id: "w".into(),
                    kind: PieceKind::Wall,
                    asset: "core/masonry".into(),
                    x: 2,
                    y: 4,
                    open: false,
                    axis: EdgeAxis::H,
                },
                EdgePiece {
                    id: "d".into(),
                    kind: PieceKind::Door,
                    asset: "core/dungeon-door".into(),
                    x: 3,
                    y: 1,
                    open: true,
                    axis: EdgeAxis::V,
                },
            ],
            props: vec![PlacedProp { id: "p".into(), asset: "core/table".into(), x: 4.0, y: 5.0, mirror: false }],
            decals: Vec::new(),
            lights: vec![crate::map::layout::PlacedLight {
                x: 2.5,
                y: 3.5,
                range: 6.0,
                color: "#e8a04a".into(),
                kind: Default::default(), intensity: 1.0,
            }],
            levels: None,
            seed: None,
            environment: None,
            variants: None,
            stairs: None,
        }
    }

    #[test]
    fn walls_doors_and_a_table_compile_apart() {
        let compiled = compile(&layout(), &catalog()).unwrap();
        assert_eq!(compiled.walls.len(), 1);
        assert!((compiled.walls[0].a.x - 2.0).abs() < 0.01 && (compiled.walls[0].b.x - 3.0).abs() < 0.01);
        assert_eq!(compiled.doors.len(), 1);
        assert!(compiled.doors[0].open);
        assert!((compiled.doors[0].a.x - 3.0).abs() < 0.01);
        assert!((compiled.doors[0].b.y - 2.0).abs() < 0.01);
        assert!(compiled.walls.iter().all(|wall| (wall.a.y - 5.0).abs() > 0.1 || (wall.a.x - 4.0).abs() > 0.1));
        assert_eq!(compiled.move_blockers.len(), 4);
        assert_eq!(compiled.lights.len(), 1);
        assert_eq!(compiled.lights[0].color, "#e8a04a");
        assert!((compiled.lights[0].range - 6.0).abs() < 0.01);
        let north = compiled.move_blockers.iter().find(|seg| (seg.a.y - 5.0).abs() < 0.01 && (seg.b.y - 5.0).abs() < 0.01);
        let north = north.expect("the table's near edge");
        assert!((north.b.x - north.a.x).abs() - 2.0 < 0.01);
    }

    #[test]
    fn a_barrel_blocks_walking_without_blocking_sight() {
        let text = r#"{
            "id":"demo","name":"Demo","pixelsPerCell":128,"filtering":"linear",
            "objects":[{
                "id":"barrel","name":"Barrel","kind":"static","placement":"cell",
                "footprintCells":[1,1],
                "blocksMovement":true,"blocksVision":false,
                "collisionPolygon":[[-0.3,-0.3],[0.3,-0.3],[0.3,0.3],[-0.3,0.3]]
            },{
                "id":"oak","name":"Oak","kind":"static","placement":"cell",
                "footprintCells":[2,2],"originCells":[1,1],
                "blocksMovement":true,"blocksVision":true,
                "collisionPolygon":[[-0.2,-0.2],[0.2,-0.2],[0.2,0.2],[-0.2,0.2]],
                "visionVolumes":[{"polygon":[[-0.2,-0.2],[0.2,-0.2],[0.2,0.2],[-0.2,0.2]],"baseHeight":0,"height":3}]
            }]
        }"#;
        let catalog = Catalog { packs: vec![parse_pack(text).unwrap()], ..Catalog::default() };
        let mut map = layout();
        map.props = vec![
            PlacedProp { id: "b".into(), asset: "demo/barrel".into(), x: 2.0, y: 3.0, mirror: false },
            PlacedProp { id: "t".into(), asset: "demo/oak".into(), x: 6.0, y: 6.0, mirror: false },
        ];
        map.pieces.clear();
        let compiled = compile(&map, &catalog).unwrap();
        assert!(compiled.walls.is_empty(), "an object must not become a wall");
        assert_eq!(compiled.move_blockers.len(), 8, "the barrel and the oak each stop walking");
        let oak = compiled
            .move_blockers
            .iter()
            .filter(|seg| seg.a.x > 5.0)
            .map(|seg| (seg.b.x - seg.a.x).abs().max((seg.b.y - seg.a.y).abs()))
            .fold(0.0_f32, f32::max);
        assert!(oak < 0.5, "the oak trunk is smaller than its 2 by 2 footprint: {oak}");
        assert!(compiled.sight.iter().any(|block| (block.top - 3.0).abs() < 0.01), "the oak keeps its authored height");
        assert!(compiled.sight.iter().all(|block| block.a.x > 5.0), "the barrel does not block sight");
    }

    #[test]
    fn a_sight_blocker_does_not_stop_walking() {
        let text = r#"{
            "id":"demo","name":"Demo","pixelsPerCell":128,"filtering":"linear",
            "objects":[{
                "id":"curtain","name":"Curtain","kind":"static","placement":"cell",
                "footprintCells":[1,1],
                "blocksMovement":false,"blocksVision":true,
                "visionVolumes":[{"polygon":[[-0.4,-0.1],[0.4,-0.1],[0.4,0.1],[-0.4,0.1]],"baseHeight":0,"height":2}]
            }]
        }"#;
        let catalog = Catalog { packs: vec![parse_pack(text).unwrap()], ..Catalog::default() };
        let mut map = layout();
        map.pieces.clear();
        map.props = vec![PlacedProp { id: "c".into(), asset: "demo/curtain".into(), x: 2.0, y: 2.0, mirror: false }];
        let compiled = compile(&map, &catalog).unwrap();
        assert!(compiled.walls.is_empty());
        assert!(compiled.move_blockers.is_empty(), "sight alone must not stop walking");
        assert!(compiled.sight.iter().any(|block| (block.top - 2.0).abs() < 0.01));
    }

    #[test]
    fn a_table_on_two_heights_is_refused() {
        let text = r#"{
            "id":"demo","name":"Demo","pixelsPerCell":128,"filtering":"linear",
            "objects":[{
                "id":"table","name":"Table","kind":"static","placement":"cell",
                "footprintCells":[2,1],"blocksMovement":true,"blocksVision":false
            }]
        }"#;
        let catalog = Catalog { packs: vec![parse_pack(text).unwrap()], ..Catalog::default() };
        let mut map = layout();
        map.pieces.clear();
        let mut heights = vec![0_u8; 100];
        heights[5 * 10 + 5] = 1;
        map.levels = Some(crate::map::layout::encode_cells(&heights));
        map.props = vec![PlacedProp { id: "t".into(), asset: "demo/table".into(), x: 4.0, y: 5.0, mirror: false }];
        let err = match compile(&map, &catalog) {
            Err(err) => err,
            Ok(_) => panic!("a table across a step compiled"),
        };
        assert!(err.contains("Table at 4, 5 stands on two heights"), "{err}");
        map.props[0].x = 2.0;
        compile(&map, &catalog).expect("the same table fits on one height");
        map.props[0].x = 9.0;
        let edge = match compile(&map, &catalog) {
            Err(err) => err,
            Ok(_) => panic!("a table hanging off the map compiled"),
        };
        assert!(edge.contains("does not fit on the map"), "{edge}");
        map.props[0].x = 2.0;
        map.props[0].mirror = true;
        let flipped = match compile(&map, &catalog) {
            Err(err) => err,
            Ok(_) => panic!("a mirrored upright table compiled"),
        };
        assert!(flipped.contains("cannot be mirrored"), "{flipped}");
    }

    #[test]
    fn an_unknown_object_is_rejected() {
        let mut map = layout();
        map.props[0].asset = "core/missing".into();
        let err = match compile(&map, &catalog()) {
            Err(err) => err,
            Ok(_) => panic!("an unknown object compiled"),
        };
        assert!(err.contains("not an object"), "{err}");
    }
}
