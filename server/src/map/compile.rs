// ------------------------------------------------------------
// Module: compile
// Purpose: Turn an editor layout into the walls, doors, and
//          movement blockers the live table already understands.
//          A prop that blocks sight becomes walls. A prop that
//          only blocks walking becomes move blockers, so a
//          table stops a token but not its vision.
// ------------------------------------------------------------

use crate::assets::{Blocks, Catalog};
use crate::map::layout::{EdgeAxis, MapLayout, PieceKind};
use crate::map::{Door, Light, Segment, Vec2};

pub struct Compiled {
    pub walls: Vec<Segment>,
    pub doors: Vec<Door>,
    pub move_blockers: Vec<Segment>,
    pub lights: Vec<Light>,
}

pub fn compile(layout: &MapLayout, catalog: &Catalog) -> Result<Compiled, String> {
    let mut walls = Vec::new();
    let mut doors = Vec::new();
    let mut move_blockers = Vec::new();
    let mut lights = Vec::new();

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

    for prop in &layout.props {
        if let Some(object) = catalog.object(&prop.asset) {
            if object.kind == "stairs" || object.placement == "connection" {
                return Err(format!("{} uses stairs, which are not available yet", prop.asset));
            }
            place_object(&mut walls, &mut move_blockers, prop.x, prop.y, object);
            continue;
        }
        let Some(def) = catalog.prop(&prop.asset) else {
            return Err(format!("{} is not an object in the catalog, so its blocker was not skipped", prop.asset));
        };
        let footprint = rectangle(prop.x, prop.y, def.footprint[0] as f32, def.footprint[1] as f32);
        match def.blocks {
            Blocks::Sight => walls.extend(footprint),
            Blocks::Move => move_blockers.extend(footprint),
            Blocks::None => {}
        }
    }

    for light in &layout.lights {
        lights.push(Light {
            x: light.x,
            y: light.y,
            range: light.range,
            color: light.color.clone(),
        });
    }

    Ok(Compiled { walls, doors, move_blockers, lights })
}

/// A vision blocker becomes an infinite wall on this flat release, so it also
/// stops walking. A movement-only object does not block sight. Finite height
/// is a later goal.
fn place_object(walls: &mut Vec<Segment>, move_blockers: &mut Vec<Segment>, x: f32, y: f32, object: &crate::assets::ObjectDef) {
    let origin = object.origin();
    let root_x = x + origin[0];
    let root_y = y + origin[1];
    let movement = polygon_or_footprint(x, y, object.footprint_cells, &object.collision_polygon, root_x, root_y);
    let vision = if let Some(volume) = object.vision_volumes.iter().find(|volume| !volume.polygon.is_empty()) {
        shift(&volume.polygon, root_x, root_y)
    } else {
        movement.clone()
    };
    if object.blocks_vision {
        walls.extend(edges(&vision));
    }
    if object.blocks_movement && !object.blocks_vision {
        move_blockers.extend(edges(&movement));
    } else if object.blocks_movement && object.blocks_vision && object.collision_polygon != vision_poly(object) {
        move_blockers.extend(edges(&movement));
    }
}

fn vision_poly(object: &crate::assets::ObjectDef) -> Vec<[f32; 2]> {
    object
        .vision_volumes
        .iter()
        .find(|volume| !volume.polygon.is_empty())
        .map(|volume| volume.polygon.clone())
        .unwrap_or_else(|| object.collision_polygon.clone())
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
            }],
            levels: None,
            seed: None,
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
