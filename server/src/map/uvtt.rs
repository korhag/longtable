// ------------------------------------------------------------
// Module: uvtt
// Purpose: Read a Universal VTT file (`.dd2vtt`), the export
//          Dungeon Alchemist writes. Grid size, wall polylines,
//          door portals, lights, and the map image all come
//          from that one JSON document.
// ------------------------------------------------------------

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use thiserror::Error;

use super::scene::{Door, Light, Segment, Vec2};

#[derive(Debug, Error)]
pub enum UvttError {
    #[error("that file is not Universal VTT JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the map size is missing or not usable")]
    Size,
    #[error("the map image is not valid base64")]
    Image,
}

/// Geometry plus the optional decoded map image.
pub struct ParsedMap {
    pub width: f32,
    pub height: f32,
    pub walls: Vec<Segment>,
    pub doors: Vec<Door>,
    pub lights: Vec<Light>,
    pub image: Option<Vec<u8>>,
}

pub fn parse_uvtt(bytes: &[u8]) -> Result<ParsedMap, UvttError> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}');
    let file: UvttFile = serde_json::from_str(text)?;
    let width = file.resolution.map_size.x;
    let height = file.resolution.map_size.y;
    if !(1.0..=400.0).contains(&width) || !(1.0..=400.0).contains(&height) {
        return Err(UvttError::Size);
    }
    let origin_x = file.resolution.map_origin.as_ref().map(|p| p.x).unwrap_or(0.0);
    let origin_y = file.resolution.map_origin.as_ref().map(|p| p.y).unwrap_or(0.0);
    let shift = |point: &UvttPoint| Vec2::new(point.x - origin_x, point.y - origin_y);

    let mut walls = Vec::new();
    for line in &file.line_of_sight {
        for pair in line.windows(2) {
            let a = shift(&pair[0]);
            let b = shift(&pair[1]);
            if (b - a).length() > 0.01 {
                walls.push(Segment { a, b });
            }
        }
    }

    let mut doors = Vec::new();
    for (index, portal) in file.portals.iter().enumerate() {
        let Some(bounds) = portal.bounds.as_ref() else {
            continue;
        };
        if bounds.len() < 2 {
            continue;
        }
        let a = shift(&bounds[0]);
        let b = shift(&bounds[1]);
        if (b - a).length() <= 0.01 {
            continue;
        }
        doors.push(Door {
            id: format!("door-{index}"),
            a,
            b,
            open: !portal.closed.unwrap_or(false),
        });
    }

    let lights = file
        .lights
        .iter()
        .filter(|light| light.range > 0.0)
        .map(|light| Light {
            x: light.position.x - origin_x,
            y: light.position.y - origin_y,
            range: light.range,
            color: css_color(light.color.as_deref()),
            kind: Default::default(),
            intensity: 1.0,
        })
        .collect();

    let image = match file.image.as_deref() {
        Some(value) if !value.trim().is_empty() => Some(decode_image(value).map_err(|_| UvttError::Image)?),
        _ => None,
    };

    Ok(ParsedMap { width, height, walls, doors, lights, image })
}

fn decode_image(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    let compact: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    let payload = if let Some(rest) = compact.strip_prefix("data:") {
        rest.split_once(',').map(|(_, data)| data).unwrap_or(rest)
    } else {
        compact.as_str()
    };
    STANDARD.decode(payload)
}

fn css_color(value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim) else {
        return "#e2a14a".into();
    };
    let hex = value.trim_start_matches('#');
    let body = if hex.len() == 8 {
        &hex[2..]
    } else if hex.len() == 6 {
        hex
    } else {
        return "#e2a14a".into();
    };
    if body.chars().all(|ch| ch.is_ascii_hexdigit()) {
        format!("#{body}")
    } else {
        "#e2a14a".into()
    }
}

#[derive(Deserialize)]
struct UvttFile {
    resolution: UvttResolution,
    #[serde(default)]
    line_of_sight: Vec<Vec<UvttPoint>>,
    #[serde(default)]
    portals: Vec<UvttPortal>,
    #[serde(default)]
    lights: Vec<UvttLight>,
    image: Option<String>,
}

#[derive(Deserialize)]
struct UvttResolution {
    map_size: UvttPoint,
    #[serde(default)]
    map_origin: Option<UvttPoint>,
}

#[derive(Deserialize)]
struct UvttPoint {
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
struct UvttPortal {
    bounds: Option<Vec<UvttPoint>>,
    closed: Option<bool>,
}

#[derive(Deserialize)]
struct UvttLight {
    position: UvttPoint,
    range: f32,
    color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    #[test]
    fn reads_grid_walls_doors_and_image() {
        let json = format!(
            r#"{{
                "resolution": {{
                    "map_origin": {{ "x": 1, "y": 2 }},
                    "map_size": {{ "x": 20, "y": 15 }},
                    "pixels_per_grid": 70
                }},
                "line_of_sight": [[
                    {{ "x": 1, "y": 2 }},
                    {{ "x": 5, "y": 2 }},
                    {{ "x": 5, "y": 6 }}
                ]],
                "portals": [{{
                    "bounds": [{{ "x": 5, "y": 2 }}, {{ "x": 5, "y": 4 }}],
                    "closed": true
                }}],
                "lights": [{{
                    "position": {{ "x": 3, "y": 4 }},
                    "range": 6,
                    "color": "ffcc8844"
                }}],
                "image": "{PNG}"
            }}"#
        );
        let parsed = parse_uvtt(json.as_bytes()).unwrap();
        assert_eq!(parsed.width, 20.0);
        assert_eq!(parsed.height, 15.0);
        assert_eq!(parsed.walls.len(), 2);
        assert_eq!(parsed.walls[0].a.x, 0.0);
        assert_eq!(parsed.walls[0].a.y, 0.0);
        assert_eq!(parsed.doors.len(), 1);
        assert!(!parsed.doors[0].open);
        assert_eq!(parsed.lights[0].x, 2.0);
        assert_eq!(parsed.lights[0].color, "#cc8844");
        assert!(parsed.image.unwrap().starts_with(b"\x89PNG"));
    }
}
