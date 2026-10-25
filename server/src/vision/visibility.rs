// ------------------------------------------------------------
// Module: visibility
// Purpose: 2D wall tests. Sight is a visibility polygon cast
//          from a token. Movement is a segment that must not
//          cross a wall or a closed door.
// ------------------------------------------------------------

use crate::map::{Segment, Vec2};

const ANGLE_NUDGE: f32 = 0.0001;

/// True when the open segment from `from` to `to` properly crosses a wall.
/// Touching an endpoint does not count, so a token can slide past a corner.
pub fn path_blocked(from: Vec2, to: Vec2, walls: &[Segment]) -> bool {
    walls.iter().any(|wall| segments_cross(from, to, wall.a, wall.b))
}

/// The polygon a token can see, clipped to `radius` grid squares.
/// With no walls this is a circle, which is the open-table case.
pub fn visibility_polygon(origin: Vec2, radius: f32, walls: &[Segment]) -> Vec<Vec2> {
    if radius <= 0.0 {
        return Vec::new();
    }

    // Sample the whole circle, then nudge extra rays at every corner so a
    // wall edge is not missed between two samples.
    let mut angles = Vec::with_capacity(72 + walls.len() * 6);
    for step in 0..64 {
        angles.push(step as f32 / 64.0 * std::f32::consts::TAU);
    }
    for wall in walls {
        for point in [wall.a, wall.b] {
            let angle = (point.y - origin.y).atan2(point.x - origin.x);
            angles.push(angle - ANGLE_NUDGE);
            angles.push(angle);
            angles.push(angle + ANGLE_NUDGE);
        }
    }

    angles.sort_by(|a, b| a.total_cmp(b));
    let mut polygon = Vec::new();
    let mut previous = f32::NAN;
    for angle in angles {
        if (angle - previous).abs() < 1e-6 {
            continue;
        }
        previous = angle;
        let dir = Vec2::new(angle.cos(), angle.sin());
        polygon.push(ray_end(origin, dir, radius, walls));
    }
    polygon
}

pub fn point_in_poly(point: Vec2, polygon: &[Vec2]) -> bool {
    let count = polygon.len();
    if count < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = count - 1;
    for index in 0..count {
        let a = polygon[index];
        let b = polygon[previous];
        let crosses = (a.y > point.y) != (b.y > point.y);
        if crosses {
            let x_at = (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
            if point.x < x_at {
                inside = !inside;
            }
        }
        previous = index;
    }
    inside
}

fn ray_end(origin: Vec2, dir: Vec2, radius: f32, walls: &[Segment]) -> Vec2 {
    let mut best = radius;
    for wall in walls {
        if let Some(distance) = ray_segment(origin, dir, wall.a, wall.b) {
            if distance > 1e-4 && distance < best {
                best = distance;
            }
        }
    }
    origin + dir * best
}

/// Distance along a unit direction until the ray hits the segment.
fn ray_segment(origin: Vec2, dir: Vec2, a: Vec2, b: Vec2) -> Option<f32> {
    let edge = b - a;
    let denom = dir.cross(edge);
    if denom.abs() < 1e-8 {
        return None;
    }
    let offset = a - origin;
    let distance = offset.cross(edge) / denom;
    let along = offset.cross(dir) / denom;
    if distance > 0.0 && (0.0..=1.0).contains(&along) {
        Some(distance)
    } else {
        None
    }
}

fn segments_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let ab_c = (b - a).cross(c - a);
    let ab_d = (b - a).cross(d - a);
    let cd_a = (d - c).cross(a - c);
    let cd_b = (d - c).cross(b - c);
    let opposite = |left: f32, right: f32| (left > 1e-5 && right < -1e-5) || (left < -1e-5 && right > 1e-5);
    opposite(ab_c, ab_d) && opposite(cd_a, cd_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> Vec<Segment> {
        vec![
            Segment { a: Vec2::new(0.0, 0.0), b: Vec2::new(10.0, 0.0) },
            Segment { a: Vec2::new(10.0, 0.0), b: Vec2::new(10.0, 10.0) },
            Segment { a: Vec2::new(10.0, 10.0), b: Vec2::new(0.0, 10.0) },
            Segment { a: Vec2::new(0.0, 10.0), b: Vec2::new(0.0, 0.0) },
        ]
    }

    #[test]
    fn center_of_a_room_sees_inside_and_not_outside() {
        let walls = room();
        let polygon = visibility_polygon(Vec2::new(5.0, 5.0), 30.0, &walls);
        assert!(point_in_poly(Vec2::new(5.0, 5.0), &polygon));
        assert!(point_in_poly(Vec2::new(1.0, 1.0), &polygon));
        assert!(!point_in_poly(Vec2::new(12.0, 5.0), &polygon));
    }

    #[test]
    fn a_wall_hides_the_far_side() {
        let walls = vec![Segment { a: Vec2::new(5.0, 0.0), b: Vec2::new(5.0, 10.0) }];
        let polygon = visibility_polygon(Vec2::new(2.0, 5.0), 20.0, &walls);
        assert!(point_in_poly(Vec2::new(3.0, 5.0), &polygon));
        assert!(!point_in_poly(Vec2::new(7.0, 5.0), &polygon));
    }

    #[test]
    fn movement_stops_at_a_wall_but_can_run_beside_it() {
        let walls = vec![Segment { a: Vec2::new(0.0, 5.0), b: Vec2::new(10.0, 5.0) }];
        assert!(path_blocked(Vec2::new(5.0, 4.0), Vec2::new(5.0, 6.0), &walls));
        assert!(!path_blocked(Vec2::new(1.0, 4.0), Vec2::new(8.0, 4.0), &walls));
    }

    #[test]
    fn open_ground_is_a_disc() {
        let polygon = visibility_polygon(Vec2::new(0.0, 0.0), 4.0, &[]);
        assert!(polygon.len() >= 16);
        assert!(point_in_poly(Vec2::new(1.0, 1.0), &polygon));
        assert!(!point_in_poly(Vec2::new(6.0, 0.0), &polygon));
    }
}
