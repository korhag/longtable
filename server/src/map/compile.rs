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

pub fn compile(layout: &MapLayout, catalog: &Catalog) -> Compiled {
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
        let Some(def) = catalog.prop(&prop.asset) else {
            continue;
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

    Compiled { walls, doors, move_blockers, lights }
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
        Catalog { packs: vec![parse_pack(text).unwrap()] }
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
        }
    }

    #[test]
    fn walls_doors_and_a_table_compile_apart() {
        let compiled = compile(&layout(), &catalog());
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
}
