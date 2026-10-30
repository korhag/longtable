// ------------------------------------------------------------
// Module: navigation
// Purpose: Walk a token from square to square. A cliff of one
//          tier needs a stair. A higher cliff is refused.
// ------------------------------------------------------------

use std::collections::{HashMap, HashSet, VecDeque};

use crate::map::layout::MapLayout;
use crate::map::{Segment, Vec2};

pub fn check_route(layout: &MapLayout, from: Vec2, to: Vec2) -> Result<(), String> {
    find_route(layout, from, to, &[], &[], [1, 1]).map(|_| ())
}

/// The squares a figure walks. A clear straight walk is kept. A wall on that
/// line can send the walk around it. A cliff still needs a stair. Every step
/// has to leave the whole body on one level.
pub fn find_route(
    layout: &MapLayout,
    from: Vec2,
    to: Vec2,
    walls: &[Segment],
    blockers: &[Segment],
    footprint: [u32; 2],
) -> Result<Vec<(i32, i32)>, String> {
    let heights = match &layout.levels {
        Some(text) => Some(crate::map::layout::decode_cells(text)?),
        None => None,
    };
    let direct = crossed_cells(from, to);
    if let Some(error) = route_error(layout, &heights, &direct, walls, blockers, footprint) {
        let start = direct[0];
        let goal = *direct.last().unwrap_or(&start);
        if let Some(path) = search(layout, &heights, start, goal, walls, blockers, footprint) {
            return Ok(path);
        }
        return Err(error);
    }
    Ok(direct)
}

fn height_at(layout: &MapLayout, heights: &Option<Vec<u8>>, x: i32, y: i32) -> u8 {
    let Some(heights) = heights else { return 0 };
    if x < 0 || y < 0 || x >= layout.width as i32 || y >= layout.height as i32 {
        return 0;
    }
    heights[(y as u32 * layout.width + x as u32) as usize]
}

fn route_error(
    layout: &MapLayout,
    heights: &Option<Vec<u8>>,
    cells: &[(i32, i32)],
    walls: &[Segment],
    blockers: &[Segment],
    footprint: [u32; 2],
) -> Option<String> {
    for pair in cells.windows(2) {
        if let Some(error) = step_error(layout, heights, pair[0], pair[1], walls, blockers, footprint) {
            return Some(error);
        }
    }
    None
}

fn step_error(
    layout: &MapLayout,
    heights: &Option<Vec<u8>>,
    from: (i32, i32),
    to: (i32, i32),
    walls: &[Segment],
    blockers: &[Segment],
    footprint: [u32; 2],
) -> Option<String> {
    let there = match stance_level(layout, heights, to, footprint) {
        Ok(level) => level,
        Err(message) => return Some(message),
    };
    let here = stance_level(layout, heights, from, footprint).unwrap_or_else(|_| height_at(layout, heights, from.0, from.1));
    let diff = here.abs_diff(there);
    if diff > 1 {
        return Some("that drop is more than one step".into());
    }
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let old = occupied(anchor_point(from, footprint), footprint);
    for (cx, cy) in occupied(anchor_point(to, footprint), footprint) {
        if old.contains(&(cx, cy)) {
            continue;
        }
        let prev = (cx - dx, cy - dy);
        if diff == 1 && !stair_between(layout, prev.0, prev.1, cx, cy) {
            return Some("there are no stairs on that edge".into());
        }
        let start = Vec2::new(prev.0 as f32 + 0.5, prev.1 as f32 + 0.5);
        let end = Vec2::new(cx as f32 + 0.5, cy as f32 + 0.5);
        if segments_hit(start, end, walls) {
            return Some("a wall is in the way".into());
        }
        if segments_hit(start, end, blockers) {
            return Some("something is in the way".into());
        }
    }
    None
}

fn stance_level(layout: &MapLayout, heights: &Option<Vec<u8>>, cell: (i32, i32), footprint: [u32; 2]) -> Result<u8, String> {
    let mut level = None;
    for (x, y) in occupied(anchor_point(cell, footprint), footprint) {
        if !inside(layout, x, y) {
            return Err("that square is off the map".into());
        }
        let here = height_at(layout, heights, x, y);
        match level {
            None => level = Some(here),
            Some(previous) if previous != here => return Err("that body does not fit on one level".into()),
            _ => {}
        }
    }
    Ok(level.unwrap_or(0))
}

fn anchor_point(cell: (i32, i32), footprint: [u32; 2]) -> (f32, f32) {
    let x = if footprint[0].max(1) % 2 == 1 { cell.0 as f32 + 0.5 } else { cell.0 as f32 };
    let y = if footprint[1].max(1) % 2 == 1 { cell.1 as f32 + 0.5 } else { cell.1 as f32 };
    (x, y)
}

fn occupied(center: (f32, f32), footprint: [u32; 2]) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for y in axis_cells(center.1, footprint[1]) {
        for x in axis_cells(center.0, footprint[0]) {
            cells.push((x, y));
        }
    }
    cells
}

fn axis_cells(center: f32, span: u32) -> Vec<i32> {
    let span = span.max(1) as i32;
    let start = (center - span as f32 / 2.0).floor() as i32;
    (0..span).map(|step| start + step).collect()
}

fn search(
    layout: &MapLayout,
    heights: &Option<Vec<u8>>,
    start: (i32, i32),
    goal: (i32, i32),
    walls: &[Segment],
    blockers: &[Segment],
    footprint: [u32; 2],
) -> Option<Vec<(i32, i32)>> {
    if start == goal {
        return Some(vec![start]);
    }
    let limit = (layout.width as usize).saturating_mul(layout.height as usize).max(1);
    let mut queue = VecDeque::from([start]);
    let mut previous = HashMap::new();
    let mut seen = HashSet::from([start]);
    let mut expanded = 0usize;
    while let Some(cell) = queue.pop_front() {
        expanded += 1;
        if expanded > limit {
            return None;
        }
        for next in [ (cell.0 + 1, cell.1), (cell.0, cell.1 + 1), (cell.0 - 1, cell.1), (cell.0, cell.1 - 1) ] {
            if seen.contains(&next) {
                continue;
            }
            if step_error(layout, heights, cell, next, walls, blockers, footprint).is_some() {
                continue;
            }
            seen.insert(next);
            previous.insert(next, cell);
            if next == goal {
                let mut path = vec![goal];
                let mut cursor = goal;
                while cursor != start {
                    cursor = previous[&cursor];
                    path.push(cursor);
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(next);
        }
    }
    None
}

fn inside(layout: &MapLayout, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < layout.width as i32 && y < layout.height as i32
}

fn segments_hit(from: Vec2, to: Vec2, walls: &[Segment]) -> bool {
    walls.iter().any(|wall| segments_cross(from, to, wall.a, wall.b))
}

fn segments_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let ab_c = (b - a).cross(c - a);
    let ab_d = (b - a).cross(d - a);
    let cd_a = (d - c).cross(a - c);
    let cd_b = (d - c).cross(b - c);
    let opposite = |left: f32, right: f32| (left > 1e-5 && right < -1e-5) || (left < -1e-5 && right > 1e-5);
    opposite(ab_c, ab_d) && opposite(cd_a, cd_b)
}

fn stair_between(layout: &MapLayout, ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    let Some(stairs) = &layout.stairs else { return false };
    stairs.iter().any(|stair| {
        let p = (stair.x as i32, stair.y as i32);
        let q = (stair.x2 as i32, stair.y2 as i32);
        (p == (ax, ay) && q == (bx, by)) || (p == (bx, by) && q == (ax, ay))
    })
}

/// Orthogonal steps from the start cell to the end cell.
/// A diagonal move is two steps, so it cannot cut a corner.
fn crossed_cells(from: Vec2, to: Vec2) -> Vec<(i32, i32)> {
    let mut x = from.x.floor() as i32;
    let mut y = from.y.floor() as i32;
    let end_x = to.x.floor() as i32;
    let end_y = to.y.floor() as i32;
    let mut cells = vec![(x, y)];
    while x != end_x || y != end_y {
        if x != end_x {
            x += (end_x - x).signum();
            cells.push((x, y));
        }
        if y != end_y {
            y += (end_y - y).signum();
            cells.push((x, y));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::layout::{encode_cells, StairLink};

    fn raised() -> MapLayout {
        let mut levels = vec![0_u8; 36];
        levels[1] = 1;
        levels[2] = 2;
        levels[6] = 1;
        let mut layout = MapLayout::blank(6, 6, "core/grass");
        layout.levels = Some(encode_cells(&levels));
        layout.stairs = Some(vec![
            StairLink { id: "up".into(), asset: "steps/stair".into(), x: 0, y: 0, x2: 1, y2: 0 },
            StairLink { id: "top".into(), asset: "steps/stair".into(), x: 1, y: 0, x2: 2, y2: 0 },
        ]);
        layout
    }

    #[test]
    fn stairs_climb_and_a_cliff_does_not() {
        let layout = raised();
        layout.validate().unwrap();
        check_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(1.5, 0.5)).unwrap();
        check_route(&layout, Vec2::new(1.5, 0.5), Vec2::new(2.5, 0.5)).unwrap();
        let cliff = check_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(0.5, 1.5)).unwrap_err();
        assert!(cliff.contains("no stairs"), "{cliff}");
        let drop = check_route(&layout, Vec2::new(2.5, 0.5), Vec2::new(3.5, 0.5)).unwrap_err();
        assert!(drop.contains("more than one step"), "{drop}");
    }

    #[test]
    fn a_stair_on_flat_ground_is_refused() {
        let mut layout = MapLayout::blank(6, 6, "core/grass");
        layout.stairs = Some(vec![
            StairLink { id: "flat".into(), asset: "steps/stair".into(), x: 0, y: 0, x2: 1, y2: 0 },
            StairLink { id: "far".into(), asset: "steps/stair".into(), x: 3, y: 2, x2: 3, y2: 3 },
        ]);
        let err = layout.validate().unwrap_err();
        assert!(err.contains("one step apart"), "{err}");
        assert!(err.contains("(0, 0) to (1, 0)"), "{err}");
        assert!(err.contains("(3, 2) to (3, 3)"), "{err}");
    }

    #[test]
    fn a_walk_goes_around_a_wall() {
        let layout = MapLayout::blank(4, 4, "core/grass");
        let wall = Segment { a: Vec2::new(1.0, 0.0), b: Vec2::new(1.0, 1.0) };
        let path = find_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(1.5, 0.5), &[wall], &[], [1, 1]).unwrap();
        assert_eq!(path, vec![(0, 0), (0, 1), (1, 1), (1, 0)]);
        let blocked = Segment { a: Vec2::new(1.0, 0.0), b: Vec2::new(1.0, 4.0) };
        let err = find_route(&layout, Vec2::new(0.5, 0.5), Vec2::new(1.5, 0.5), &[blocked], &[], [1, 1]).unwrap_err();
        assert!(err.contains("wall"), "{err}");
    }
}
