// ------------------------------------------------------------
// Module: fog
// Purpose: A viewer's memory of the map. Each grid square is
//          split into 4x4 cells. A set bit has been seen.
//          Bits are row-major, x fastest, with the high bit
//          of each byte first. The client uses the same order.
// ------------------------------------------------------------

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::map::Vec2;

pub const CELLS_PER_SQUARE: u32 = 4;

#[derive(Clone, Debug)]
pub struct Explored {
    pub width: u32,
    pub height: u32,
    pub bits: Vec<u8>,
}

impl Explored {
    pub fn new(width: u32, height: u32) -> Self {
        let cells = width.saturating_mul(height) as usize;
        Self {
            width,
            height,
            bits: vec![0; cells.div_ceil(8)],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        *self = Self::new(width, height);
    }

    /// Combine equally sized masks without revisiting individual cells.
    pub fn merge(&mut self, other: &Self) {
        if self.width != other.width || self.height != other.height {
            return;
        }
        for (ours, theirs) in self.bits.iter_mut().zip(&other.bits) {
            *ours |= theirs;
        }
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        let Some(index) = self.index(x, y) else {
            return false;
        };
        let byte = self.bits[index / 8];
        let bit = 7 - (index % 8);
        byte & (1 << bit) != 0
    }

    pub fn set(&mut self, x: i32, y: i32, value: bool) {
        let Some(index) = self.index(x, y) else {
            return;
        };
        let bit = 7 - (index % 8);
        if value {
            self.bits[index / 8] |= 1 << bit;
        } else {
            self.bits[index / 8] &= !(1 << bit);
        }
    }

    /// Paint every fog cell whose center lies inside the polygon.
    pub fn paint_polygon(&mut self, polygon: &[Vec2]) {
        if polygon.len() < 3 {
            return;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for point in polygon {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        let scale = CELLS_PER_SQUARE as f32;
        let x0 = ((min_x * scale).floor() as i32).max(0);
        let y0 = ((min_y * scale).floor() as i32).max(0);
        let x1 = ((max_x * scale).ceil() as i32).min(self.width as i32 - 1);
        let y1 = ((max_y * scale).ceil() as i32).min(self.height as i32 - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let center = Vec2::new((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
                if super::visibility::point_in_poly(center, polygon) {
                    self.set(x, y, true);
                }
            }
        }
    }

    pub fn to_base64(&self) -> String {
        STANDARD.encode(&self.bits)
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }
}

pub fn cells_for(squares_w: f32, squares_h: f32) -> (u32, u32) {
    let scale = CELLS_PER_SQUARE as f32;
    (
        (squares_w * scale).ceil().max(1.0) as u32,
        (squares_h * scale).ceil().max(1.0) as u32,
    )
}

impl Serialize for Explored {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Explored", 3)?;
        state.serialize_field("width", &self.width)?;
        state.serialize_field("height", &self.height)?;
        state.serialize_field("bits", &self.to_base64())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Explored {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            width: u32,
            height: u32,
            bits: String,
        }
        let raw = Raw::deserialize(deserializer)?;
        let bits = STANDARD
            .decode(raw.bits.trim())
            .map_err(serde::de::Error::custom)?;
        Ok(Explored {
            width: raw.width,
            height: raw.height,
            bits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_round_trip_through_base64() {
        let mut fog = Explored::new(4, 2);
        fog.set(0, 0, true);
        fog.set(3, 1, true);
        let encoded = serde_json::to_string(&fog).unwrap();
        let decoded: Explored = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.get(0, 0));
        assert!(decoded.get(3, 1));
        assert!(!decoded.get(1, 0));
    }

    #[test]
    fn a_square_polygon_paints_its_cells() {
        let mut fog = Explored::new(16, 16);
        let polygon = vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(3.0, 1.0),
            Vec2::new(3.0, 3.0),
            Vec2::new(1.0, 3.0),
        ];
        fog.paint_polygon(&polygon);
        assert!(fog.get(6, 6));
        assert!(!fog.get(0, 0));
    }
}
