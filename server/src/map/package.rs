// ------------------------------------------------------------
// Module: package
// Purpose: Read a .ltmap archive. The logical grid comes from
//          the Universal VTT file. The pictures are the views.
// ------------------------------------------------------------

use std::io::{Cursor, Read};
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;
use zip::ZipArchive;

use super::presentation::{Capabilities, LightingDef, Prerendered, SavedDoor, SavedDoorView, SavedVariant, SavedView};
use super::uvtt::{parse_uvtt, ParsedMap};

const MAX_BYTES: u64 = 1536 * 1024 * 1024;
const MAX_FILES: usize = 400;
const MAX_SIDE: u32 = 8192;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("that package is not a valid archive")]
    Zip,
    #[error("{0}")]
    Message(String),
}

pub struct LoadedPackage {
    pub parsed: ParsedMap,
    pub presentation: Prerendered,
    pub files: Vec<(String, Vec<u8>)>,
}

pub fn read_package(bytes: &[u8]) -> Result<LoadedPackage, PackageError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| PackageError::Zip)?;
    if archive.len() > MAX_FILES {
        return Err(PackageError::Message("that package has too many files".into()));
    }
    let mut manifest_bytes = None;
    let mut logic_bytes = None;
    let mut files = Vec::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|_| PackageError::Zip)?;
        if file.is_dir() {
            continue;
        }
        let name = file
            .enclosed_name()
            .ok_or_else(|| PackageError::Message("that package has an unsafe path".into()))?
            .to_string_lossy()
            .replace('\\', "/");
        if name.is_empty() || name.ends_with('/') {
            continue;
        }
        total = total.saturating_add(file.size());
        if total > MAX_BYTES {
            return Err(PackageError::Message("that package is too large".into()));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|err| PackageError::Message(err.to_string()))?;
        match name.as_str() {
            "manifest.json" => manifest_bytes = Some(bytes),
            "logic.dd2vtt" => logic_bytes = Some(bytes),
            _ => files.push((name, bytes)),
        }
    }
    let manifest_bytes = manifest_bytes.ok_or_else(|| PackageError::Message("the package has no manifest".into()))?;
    let logic_bytes = logic_bytes.ok_or_else(|| PackageError::Message("the package has no gameplay file".into()))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(|err| PackageError::Message(err.to_string()))?;
    if manifest.format != "longtable-prerendered-scene" || manifest.version != 1 {
        return Err(PackageError::Message("this package version is not supported".into()));
    }
    let parsed = parse_uvtt(&logic_bytes).map_err(|err| PackageError::Message(err.to_string()))?;
    if (parsed.width - manifest.grid.width as f32).abs() > 0.01 || (parsed.height - manifest.grid.height as f32).abs() > 0.01 {
        return Err(PackageError::Message("the gameplay grid does not match the manifest".into()));
    }
    let presentation = validate(manifest, &files)?;
    Ok(LoadedPackage { parsed, presentation, files })
}

fn validate(manifest: Manifest, files: &[(String, Vec<u8>)]) -> Result<Prerendered, PackageError> {
    if manifest.views.is_empty() {
        return Err(PackageError::Message("the package has no views".into()));
    }
    if !manifest.views.iter().any(|view| view.id == manifest.default_view) {
        return Err(PackageError::Message("the default view is missing".into()));
    }
    if !manifest.lightings.iter().any(|lighting| lighting.id == manifest.default_lighting) {
        return Err(PackageError::Message("the default lighting is missing".into()));
    }
    let mut views = Vec::new();
    for view in &manifest.views {
        if view.projection != "orthographic" {
            return Err(PackageError::Message(format!("{} is not an orthographic view", view.id)));
        }
        if view.image_size[0] == 0 || view.image_size[1] == 0 || view.image_size[0] > MAX_SIDE || view.image_size[1] > MAX_SIDE {
            return Err(PackageError::Message(format!("{} is an unsupported image size", view.id)));
        }
        if view.camera.pull <= 0.0 || view.camera.ortho_half_width <= 0.0 || view.camera.ortho_half_height <= 0.0 {
            return Err(PackageError::Message(format!("{} has no usable camera", view.id)));
        }
        check_agreement(view)?;
        if view.variants.is_empty() {
            return Err(PackageError::Message(format!("{} has no pictures", view.id)));
        }
        for variant in &view.variants {
            require(files, &variant.color)?;
        }
        if manifest.capabilities.depth {
            let depth = view.depth.as_ref().ok_or_else(|| PackageError::Message(format!("{} is missing its depth", view.id)))?;
            if depth.encoding != "viewZ" {
                return Err(PackageError::Message(format!("{} has an unknown depth encoding", view.id)));
            }
            require(files, &depth.file)?;
        }
        views.push(SavedView {
            id: view.id.clone(),
            azimuth_deg: view.camera.azimuth_deg as f32,
            elevation_deg: view.camera.elevation_deg as f32,
            ortho_half_width: view.camera.ortho_half_width as f32,
            ortho_half_height: view.camera.ortho_half_height as f32,
            target: [view.camera.target[0] as f32, view.camera.target[1] as f32, view.camera.target[2] as f32],
            image_size: view.image_size,
            ground_to_image: [
                [view.ground_to_image[0][0] as f32, view.ground_to_image[0][1] as f32, view.ground_to_image[0][2] as f32],
                [view.ground_to_image[1][0] as f32, view.ground_to_image[1][1] as f32, view.ground_to_image[1][2] as f32],
            ],
            height_to_image: [view.height_to_image[0] as f32, view.height_to_image[1] as f32],
            pull: view.camera.pull as f32,
            near: view.camera.near as f32,
            far: view.camera.far as f32,
            pixels_per_square: view.camera.pixels_per_square as f32,
            depth_file: view.depth.as_ref().map(|depth| depth.file.clone()),
            variants: view
                .variants
                .iter()
                .map(|variant| SavedVariant { id: variant.id.clone(), file: variant.color.clone() })
                .collect(),
        });
    }
    let mut doors = Vec::new();
    for door in &manifest.doors {
        let mut layers = Vec::new();
        for layer in &door.views {
            require(files, &layer.color)?;
            if let Some(depth) = &layer.depth {
                require(files, depth)?;
            }
            layers.push(SavedDoorView {
                view: layer.view.clone(),
                lighting: layer.lighting.clone(),
                file: layer.color.clone(),
                depth_file: layer.depth.clone(),
            });
        }
        doors.push(SavedDoor { id: door.id.clone(), views: layers });
    }
    Ok(Prerendered {
        active_lighting: manifest.default_lighting,
        default_view: manifest.default_view,
        view_order: if manifest.view_order.is_empty() {
            views.iter().map(|view| view.id.clone()).collect()
        } else {
            manifest.view_order
        },
        lightings: manifest.lightings.into_iter().map(|lighting| LightingDef { id: lighting.id, label: lighting.label }).collect(),
        views,
        doors,
        capabilities: Capabilities {
            depth: manifest.capabilities.depth,
            doors: manifest.capabilities.doors,
            move_blockers: false,
            walls: manifest.capabilities.walls,
            lights: manifest.capabilities.lights,
            snow: manifest.capabilities.snow,
        },
        warnings: manifest.warnings,
    })
}

fn require(files: &[(String, Vec<u8>)], name: &str) -> Result<(), PackageError> {
    if files.iter().any(|(file, _)| file == name) {
        Ok(())
    } else {
        Err(PackageError::Message(format!("the package is missing {name}")))
    }
}

fn check_agreement(view: &ViewManifest) -> Result<(), PackageError> {
    let stored = view.ground_to_image;
    let height = view.height_to_image;
    let (x, y) = unproject(stored, project(stored, height, 2.5, 4.5, 0.0).0, project(stored, height, 2.5, 4.5, 0.0).1);
    if (x - 2.5).abs() > 0.01 || (y - 4.5).abs() > 0.01 {
        return Err(PackageError::Message(format!("{} does not line up with its grid", view.id)));
    }
    let expected = expected_ground(view);
    let again = project(expected, height_of(view), 1.0, 2.0, 0.0);
    let (back_x, back_y) = unproject(stored, again.0, again.1);
    if (back_x - 1.0).abs() > 0.01 || (back_y - 2.0).abs() > 0.01 {
        return Err(PackageError::Message(format!("{} camera does not match its picture", view.id)));
    }
    Ok(())
}

fn height_of(view: &ViewManifest) -> [f64; 2] {
    let full_w = view.camera.ortho_half_width * 2.0;
    let full_h = view.camera.ortho_half_height * 2.0;
    let (_, up, right) = basis(view.camera.azimuth_deg, view.camera.elevation_deg);
    let su = view.image_size[0] as f64 / full_w;
    let sv = view.image_size[1] as f64 / full_h;
    [right[1] * su, -up[1] * sv]
}

fn expected_ground(view: &ViewManifest) -> [[f64; 3]; 2] {
    let full_w = view.camera.ortho_half_width * 2.0;
    let full_h = view.camera.ortho_half_height * 2.0;
    let (_, up, right) = basis(view.camera.azimuth_deg, view.camera.elevation_deg);
    let target = view.camera.target;
    let su = view.image_size[0] as f64 / full_w;
    let sv = view.image_size[1] as f64 / full_h;
    let dot_r = target[0] * right[0] + target[1] * right[1] + target[2] * right[2];
    let dot_u = target[0] * up[0] + target[1] * up[1] + target[2] * up[2];
    [
        [right[0] * su, right[2] * su, -dot_r * su + view.image_size[0] as f64 * 0.5],
        [-up[0] * sv, -up[2] * sv, dot_u * sv + view.image_size[1] as f64 * 0.5],
    ]
}

fn project(ground: [[f64; 3]; 2], height: [f64; 2], x: f64, y: f64, h: f64) -> (f64, f64) {
    (
        ground[0][0] * x + ground[0][1] * y + ground[0][2] + height[0] * h,
        ground[1][0] * x + ground[1][1] * y + ground[1][2] + height[1] * h,
    )
}

fn unproject(ground: [[f64; 3]; 2], u: f64, v: f64) -> (f64, f64) {
    let a = ground[0][0];
    let b = ground[0][1];
    let tx = ground[0][2];
    let c = ground[1][0];
    let d = ground[1][1];
    let ty = ground[1][2];
    let det = a * d - b * c;
    let du = u - tx;
    let dv = v - ty;
    ((d * du - b * dv) / det, (-c * du + a * dv) / det)
}

fn basis(azimuth_deg: f64, elevation_deg: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let az = azimuth_deg.to_radians();
    let el = elevation_deg.to_radians();
    let (sin_e, cos_e) = el.sin_cos();
    let (sin_a, cos_a) = az.sin_cos();
    let forward = norm([sin_a * cos_e, -sin_e, -cos_a * cos_e]);
    let raw = [sin_a * cos_e * sin_e, cos_e * cos_e, -cos_a * cos_e * sin_e];
    let up = if len(raw) < 1e-8 { [0.0, 0.0, -1.0] } else { norm(raw) };
    let right = norm(cross(forward, up));
    let up = norm(cross(right, forward));
    (forward, up, right)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn len(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let length = len(v).max(1e-12);
    [v[0] / length, v[1] / length, v[2] / length]
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format: String,
    version: u32,
    #[serde(default)]
    view_order: Vec<String>,
    default_view: String,
    default_lighting: String,
    #[serde(default)]
    lightings: Vec<LightingManifest>,
    grid: GridManifest,
    views: Vec<ViewManifest>,
    #[serde(default)]
    doors: Vec<DoorManifest>,
    capabilities: CapabilityManifest,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridManifest {
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityManifest {
    #[serde(default)]
    depth: bool,
    #[serde(default)]
    doors: bool,
    #[serde(default)]
    walls: bool,
    #[serde(default)]
    lights: bool,
    #[serde(default)]
    snow: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LightingManifest {
    id: String,
    label: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewManifest {
    id: String,
    projection: String,
    image_size: [u32; 2],
    camera: CameraManifest,
    ground_to_image: [[f64; 3]; 2],
    height_to_image: [f64; 2],
    #[serde(default)]
    depth: Option<DepthManifest>,
    variants: Vec<VariantManifest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CameraManifest {
    azimuth_deg: f64,
    elevation_deg: f64,
    ortho_half_width: f64,
    ortho_half_height: f64,
    target: [f64; 3],
    pull: f64,
    near: f64,
    far: f64,
    pixels_per_square: f64,
}

#[derive(Deserialize)]
struct DepthManifest {
    file: String,
    encoding: String,
}

#[derive(Deserialize)]
struct VariantManifest {
    id: String,
    color: String,
}

#[derive(Deserialize)]
struct DoorManifest {
    id: String,
    views: Vec<DoorViewManifest>,
}

#[derive(Deserialize)]
struct DoorViewManifest {
    view: String,
    lighting: String,
    color: String,
    #[serde(default)]
    depth: Option<String>,
}

pub fn safe_relative(path: &str) -> Option<&Path> {
    if path.is_empty() || path.contains('\\') || path.starts_with('/') {
        return None;
    }
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
    }
    Some(Path::new(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_fixture_lines_up() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/asymmetric-test.ltmap")).unwrap();
        let loaded = read_package(&bytes).unwrap();
        assert_eq!(loaded.parsed.width, 10.0);
        assert_eq!(loaded.parsed.height, 7.0);
        assert_eq!(loaded.parsed.walls.len(), 2);
        assert_eq!(loaded.parsed.doors.len(), 1);
        assert!(!loaded.parsed.doors[0].open);
        assert_eq!(loaded.presentation.views.len(), 2);
        assert!(loaded.presentation.capabilities.depth);
        assert!(!loaded.presentation.capabilities.move_blockers);
        assert_ne!(loaded.presentation.views[0].azimuth_deg, loaded.presentation.views[1].azimuth_deg);
    }

    #[test]
    fn a_bad_path_is_rejected() {
        assert!(safe_relative("../secret").is_none());
        assert!(safe_relative("views/view-0/day.png").is_some());
    }
}
