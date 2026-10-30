// ------------------------------------------------------------
// Module: height
// Purpose: Sight that knows the ground height. An eye sits one
//          step above its square. A figure is sampled a little
//          lower. Terrain is solid below its surface, and a
//          blocker only counts inside its own vertical span.
// ------------------------------------------------------------

use std::collections::HashMap;

use crate::map::layout::MapLayout;
use crate::map::{SightBlock, Vec2};
use crate::vision::fog::{Explored, CELLS_PER_SQUARE};

/// Eye height above the square the observer stands on.
pub const EYE_HEIGHT: f32 = 1.0;
/// Sample height above the square a figure stands on.
pub const TARGET_HEIGHT: f32 = 0.8;
/// A wall or closed door with no authored volume.
pub const TALL_WALL: f32 = 8.0;

struct Ground {
    width: u32,
    height: u32,
    bytes: Option<Vec<u8>>,
}

impl Ground {
    fn from_layout(layout: &MapLayout) -> Self {
        let bytes = layout.levels.as_deref().and_then(|text| crate::map::layout::decode_cells(text).ok());
        Self { width: layout.width, height: layout.height, bytes }
    }

    fn at(&self, x: f32, y: f32) -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        if ix < 0 || iy < 0 || ix >= self.width as i32 || iy >= self.height as i32 {
            return 0.0;
        }
        let Some(bytes) = &self.bytes else { return 0.0 };
        let index = (iy as u32 * self.width + ix as u32) as usize;
        bytes.get(index).copied().unwrap_or(0).min(2) as f32
    }
}

/// One look across the map. The ground is decoded once, and a blocker is
/// tested only when the sight line crosses a square that blocker touches.
struct Field<'a> {
    ground: Ground,
    spans: &'a [SightBlock],
    buckets: HashMap<(i32, i32), Vec<usize>>,
    stamp: Vec<u32>,
    tick: u32,
}

impl<'a> Field<'a> {
    fn new(layout: &MapLayout, spans: &'a [SightBlock]) -> Self {
        let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (index, span) in spans.iter().enumerate() {
            for cell in span_cells(span) {
                buckets.entry(cell).or_default().push(index);
            }
        }
        Self { ground: Ground::from_layout(layout), spans, buckets, stamp: vec![0; spans.len()], tick: 0 }
    }

    fn sees_surface(&mut self, eye_x: f32, eye_y: f32, radius: f32, target_x: f32, target_y: f32) -> bool {
        self.sees(eye_x, eye_y, radius, target_x, target_y, 0.05)
    }

    fn sees_actor(&mut self, eye_x: f32, eye_y: f32, radius: f32, target_x: f32, target_y: f32) -> bool {
        self.sees(eye_x, eye_y, radius, target_x, target_y, TARGET_HEIGHT)
    }

    fn sees(&mut self, eye_x: f32, eye_y: f32, radius: f32, target_x: f32, target_y: f32, above: f32) -> bool {
        let dx = target_x - eye_x;
        let dy = target_y - eye_y;
        if dx * dx + dy * dy > radius * radius {
            return false;
        }
        let eye = self.ground.at(eye_x, eye_y) + EYE_HEIGHT;
        let sample = self.ground.at(target_x, target_y) + above;
        self.ray_clear(eye_x, eye_y, eye, target_x, target_y, sample)
    }

    fn ray_clear(&mut self, from_x: f32, from_y: f32, from_h: f32, to_x: f32, to_y: f32, to_h: f32) -> bool {
        if (to_x - from_x).hypot(to_y - from_y) < 0.05 {
            return true;
        }
        let start = (from_x.floor() as i32, from_y.floor() as i32);
        let end = (to_x.floor() as i32, to_y.floor() as i32);
        let cells = crossed_cells(from_x, from_y, to_x, to_y);
        for (cell_x, cell_y) in &cells {
            if (*cell_x, *cell_y) == start || (*cell_x, *cell_y) == end {
                continue;
            }
            let Some((t0, t1)) = clip_cell(from_x, from_y, to_x, to_y, *cell_x, *cell_y) else {
                continue;
            };
            let low = (from_h + (to_h - from_h) * t0).min(from_h + (to_h - from_h) * t1);
            let surface = self.ground.at(*cell_x as f32 + 0.5, *cell_y as f32 + 0.5);
            if low < surface - 0.02 {
                return false;
            }
        }
        self.tick = self.tick.wrapping_add(1);
        if self.tick == 0 {
            self.stamp.fill(0);
            self.tick = 1;
        }
        let mut hits = Vec::new();
        for (cell_x, cell_y) in cells {
            if let Some(list) = self.buckets.get(&(cell_x, cell_y)) {
                hits.extend(list.iter().copied());
            }
        }
        let from = Vec2::new(from_x, from_y);
        let to = Vec2::new(to_x, to_y);
        let tick = self.tick;
        for index in hits {
            if self.stamp[index] == tick {
                continue;
            }
            self.stamp[index] = tick;
            let span = &self.spans[index];
            let Some(t) = cross_t(from, to, span.a, span.b) else {
                continue;
            };
            let height = from_h + (to_h - from_h) * t;
            if height > span.base + 0.02 && height < span.top - 0.02 {
                return false;
            }
        }
        true
    }
}

fn span_cells(span: &SightBlock) -> Vec<(i32, i32)> {
    let min_x = (span.a.x.min(span.b.x) - 0.01).floor() as i32;
    let max_x = (span.a.x.max(span.b.x) + 0.01).floor() as i32;
    let min_y = (span.a.y.min(span.b.y) - 0.01).floor() as i32;
    let max_y = (span.a.y.max(span.b.y) + 0.01).floor() as i32;
    let mut cells = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            cells.push((x, y));
        }
    }
    cells
}

pub fn sees_surface(eye_x: f32, eye_y: f32, radius: f32, target_x: f32, target_y: f32, layout: &MapLayout, spans: &[SightBlock]) -> bool {
    Field::new(layout, spans).sees_surface(eye_x, eye_y, radius, target_x, target_y)
}

pub fn sees_actor(eye_x: f32, eye_y: f32, radius: f32, target_x: f32, target_y: f32, layout: &MapLayout, spans: &[SightBlock]) -> bool {
    Field::new(layout, spans).sees_actor(eye_x, eye_y, radius, target_x, target_y)
}

/// Paint explored memory and, when provided, the cells visible right now.
pub fn reveal_height(explored: &mut Explored, mut visible: Option<&mut Explored>, eye_x: f32, eye_y: f32, radius: f32, layout: &MapLayout, spans: &[SightBlock]) {
    if radius <= 0.0 {
        return;
    }
    let mut field = Field::new(layout, spans);
    let scale = CELLS_PER_SQUARE as f32;
    let reach = (radius * scale).ceil() as i32 + 1;
    let origin_x = (eye_x * scale).floor() as i32;
    let origin_y = (eye_y * scale).floor() as i32;
    for y in (origin_y - reach)..=(origin_y + reach) {
        for x in (origin_x - reach)..=(origin_x + reach) {
            let px = (x as f32 + 0.5) / scale;
            let py = (y as f32 + 0.5) / scale;
            if !field.sees_surface(eye_x, eye_y, radius, px, py) {
                continue;
            }
            explored.set(x, y, true);
            if let Some(mask) = visible.as_mut() {
                mask.set(x, y, true);
            }
        }
    }
}

fn clip_cell(x0: f32, y0: f32, x1: f32, y1: f32, cell_x: i32, cell_y: i32) -> Option<(f32, f32)> {
    let mut t0 = 0.0;
    let mut t1 = 1.0;
    if !slab(x0, x1 - x0, cell_x as f32, cell_x as f32 + 1.0, &mut t0, &mut t1) {
        return None;
    }
    if !slab(y0, y1 - y0, cell_y as f32, cell_y as f32 + 1.0, &mut t0, &mut t1) {
        return None;
    }
    Some((t0, t1))
}

fn slab(origin: f32, delta: f32, min: f32, max: f32, t0: &mut f32, t1: &mut f32) -> bool {
    if delta.abs() < 1e-8 {
        return origin >= min - 1e-4 && origin <= max + 1e-4;
    }
    let mut near = (min - origin) / delta;
    let mut far = (max - origin) / delta;
    if near > far {
        std::mem::swap(&mut near, &mut far);
    }
    *t0 = t0.max(near);
    *t1 = t1.min(far);
    *t0 <= *t1
}

fn cross_t(from: Vec2, to: Vec2, a: Vec2, b: Vec2) -> Option<f32> {
    let dir = to - from;
    let edge = b - a;
    let denom = dir.cross(edge);
    if denom.abs() < 1e-8 {
        return None;
    }
    let offset = a - from;
    let t = offset.cross(edge) / denom;
    let along = offset.cross(dir) / denom;
    if (0.02..0.98).contains(&t) && (0.0..=1.0).contains(&along) {
        Some(t)
    } else {
        None
    }
}

fn crossed_cells(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(i32, i32)> {
    let mut x = x0.floor() as i32;
    let mut y = y0.floor() as i32;
    let end_x = x1.floor() as i32;
    let end_y = y1.floor() as i32;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let step_x = if dx > 0.0 { 1 } else { -1 };
    let step_y = if dy > 0.0 { 1 } else { -1 };
    let mut t_max_x = next_boundary(x0, dx, x, step_x);
    let mut t_max_y = next_boundary(y0, dy, y, step_y);
    let t_dx = if dx.abs() < 1e-8 { f32::INFINITY } else { (1.0 / dx).abs() };
    let t_dy = if dy.abs() < 1e-8 { f32::INFINITY } else { (1.0 / dy).abs() };
    let mut cells = vec![(x, y)];
    let mut guard = 0;
    while (x != end_x || y != end_y) && guard < 512 {
        if t_max_x < t_max_y - 1e-6 {
            x += step_x;
            t_max_x += t_dx;
        } else if t_max_y < t_max_x - 1e-6 {
            y += step_y;
            t_max_y += t_dy;
        } else {
            cells.push((x + step_x, y));
            cells.push((x, y + step_y));
            x += step_x;
            y += step_y;
            t_max_x += t_dx;
            t_max_y += t_dy;
        }
        cells.push((x, y));
        guard += 1;
    }
    cells
}

fn next_boundary(origin: f32, delta: f32, cell: i32, step: i32) -> f32 {
    if delta.abs() < 1e-8 {
        return f32::INFINITY;
    }
    let boundary = if step > 0 { cell as f32 + 1.0 } else { cell as f32 };
    ((boundary - origin) / delta).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::layout::encode_cells;
    use std::time::Instant;

    fn ground(levels: &[u8]) -> MapLayout {
        let mut layout = MapLayout::blank(8, 8, "core/grass");
        layout.levels = Some(encode_cells(levels));
        layout
    }

    fn wall(x: f32, top: f32) -> SightBlock {
        SightBlock { a: Vec2::new(x, 0.0), b: Vec2::new(x, 8.0), base: 0.0, top }
    }

    #[test]
    fn a_high_eye_clears_a_low_wall_and_not_a_tall_one() {
        let mut levels = vec![0_u8; 64];
        levels[1] = 2;
        let layout = ground(&levels);
        let low = [wall(4.0, 0.5)];
        assert!(sees_actor(1.5, 1.5, 20.0, 6.5, 1.5, &layout, &low));
        let tall = [wall(4.0, 4.0)];
        assert!(!sees_actor(1.5, 1.5, 20.0, 6.5, 1.5, &layout, &tall));
    }

    #[test]
    fn a_figure_can_be_seen_when_the_floor_sample_is_not() {
        let layout = MapLayout::blank(8, 8, "core/grass");
        let hedge = [wall(4.0, 0.6)];
        assert!(!sees_surface(1.5, 1.5, 20.0, 6.5, 1.5, &layout, &hedge));
        assert!(sees_actor(1.5, 1.5, 20.0, 6.5, 1.5, &layout, &hedge));
    }

    #[test]
    fn a_ridge_hides_the_far_side() {
        let mut levels = vec![0_u8; 64];
        for y in 0..8 {
            levels[y * 8 + 3] = 2;
        }
        let layout = ground(&levels);
        assert!(!sees_actor(1.5, 1.5, 20.0, 5.5, 1.5, &layout, &[]));
    }

    #[test]
    fn height_sight_on_a_small_map_stays_bounded() {
        let layout = MapLayout::blank(12, 12, "core/grass");
        let spans = vec![wall(6.0, 8.0)];
        let started = Instant::now();
        for _ in 0..200 {
            assert!(!sees_actor(1.5, 6.5, 20.0, 10.5, 6.5, &layout, &spans));
        }
        let elapsed = started.elapsed();
        assert!(elapsed.as_millis() < 500, "200 sight checks took {elapsed:?}");
    }
}
