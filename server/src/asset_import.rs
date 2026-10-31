// ------------------------------------------------------------
// Module: asset_import
// Purpose: Check a schema-2 pack and install it beside the
//          others. A bad archive installs nothing.
// ------------------------------------------------------------

use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::assets::Pack;
use crate::map::DIMETRIC_PROFILE;

pub const MAX_COMPRESSED: usize = 100 * 1024 * 1024;
pub const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
pub const MAX_FILES: usize = 2_000;
pub const MAX_SIDE: u32 = 2_048;
/// Decoded RGBA bytes allowed for one pack. Checked before a full decode.
pub const MAX_DECODED: u64 = 256 * 1024 * 1024;

static INSTALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn content_hash(manifest: &str, files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(manifest.as_bytes());
    for (name, bytes) in files {
        hasher.update(name.as_bytes());
        hasher.update(bytes);
    }
    format!("{:x}", hasher.finalize())
}

pub fn validate_v2_shape(pack: &Pack) -> Result<(), String> {
    if pack.visual_profile != DIMETRIC_PROFILE {
        return Err(format!(
            "pack {} uses profile {:?}, and this table only accepts {DIMETRIC_PROFILE}",
            pack.id, pack.visual_profile
        ));
    }
    if pack.id.is_empty() || !pack.id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        return Err(format!("pack id {:?} is not a plain folder name", pack.id));
    }
    let mut ids = HashSet::new();
    for tile in &pack.tiles {
        remember(&mut ids, &pack.id, &tile.id)?;
        if !matches!(tile.kind.as_str(), "ground" | "overlay" | "side") {
            return Err(format!("tile {}/{} has an unknown kind {}", pack.id, tile.id, tile.kind));
        }
        if tile.kind == "ground" && (tile.source_size_px != [128, 128] || tile.coverage_cells != [1, 1]) {
            return Err(format!("tile {}/{} must be a 128 by 128 picture of one cell", pack.id, tile.id));
        }
        if !matches!(tile.transition_policy.as_str(), "" | "none" | "rigid" | "organic") {
            return Err(format!("tile {}/{} has an unknown transition {}", pack.id, tile.id, tile.transition_policy));
        }
        if tile.coverage_cells[0] == 0 || tile.coverage_cells[1] == 0 || tile.coverage_cells[0] > 8 || tile.coverage_cells[1] > 8 {
            return Err(format!("tile {}/{} coverage must be between 1 and 8 cells", pack.id, tile.id));
        }
        if tile.files.is_empty() {
            return Err(format!("tile {}/{} has no file", pack.id, tile.id));
        }
        for file in &tile.files {
            check_path(&pack.id, file)?;
        }
    }
    for object in &pack.objects {
        remember(&mut ids, &pack.id, &object.id)?;
        let stair = object.kind == "stairs" || object.placement == "connection";
        if stair && (object.kind != "stairs" || object.placement != "connection") {
            return Err(format!("object {}/{} stairs use a connection between two squares", pack.id, object.id));
        }
        if !stair && !matches!(object.kind.as_str(), "static" | "wall" | "door") {
            return Err(format!("object {}/{} has an unknown kind {}", pack.id, object.id, object.kind));
        }
        if !stair && !matches!(object.placement.as_str(), "cell" | "edge") {
            return Err(format!("object {}/{} has an unknown placement {}", pack.id, object.id, object.placement));
        }
        if object.footprint_cells[0] == 0 || object.footprint_cells[1] == 0 || object.footprint_cells[0] > 8 || object.footprint_cells[1] > 8 {
            return Err(format!("object {}/{} footprint must be between 1 and 8 cells", pack.id, object.id));
        }
        if !(1..=4_096).contains(&object.pixels_per_world_unit) {
            return Err(format!("object {}/{} scale must be between 1 and 4096 pixels per world unit", pack.id, object.id));
        }
        if let Some(origin) = object.origin_cells {
            finite_pair(origin, &format!("object {}/{} origin", pack.id, object.id))?;
        }
        if let Some(structure) = &object.structure {
            let expected = match structure.profile.as_str() {
                "log-wall" => "wall",
                "plank-door" => "door",
                _ => return Err(format!("object {}/{} has an unknown structure profile", pack.id, object.id)),
            };
            if object.kind != expected || object.placement != "edge" {
                return Err(format!("object {}/{} structure must be an edge {expected}", pack.id, object.id));
            }
            if !structure.height.is_finite() || !(1.0..=4.0).contains(&structure.height)
                || !structure.thickness.is_finite() || !(0.1..=0.5).contains(&structure.thickness) {
                return Err(format!("object {}/{} structure needs height 1–4 and thickness 0.1–0.5", pack.id, object.id));
            }
            check_path(&pack.id, &structure.texture)?;
        }
        finite_polygon(&object.collision_polygon, &format!("object {}/{} collision", pack.id, object.id))?;
        for (index, volume) in object.vision_volumes.iter().enumerate() {
            finite_polygon(&volume.polygon, &format!("object {}/{} vision {index}", pack.id, object.id))?;
            if !volume.base_height.is_finite() || !volume.height.is_finite() || volume.height <= 0.0 || volume.height > 32.0 {
                return Err(format!("object {}/{} vision {index} needs a finite height up to 32", pack.id, object.id));
            }
        }
        if object.render_layers.is_empty() {
            return Err(format!("object {}/{} has no picture", pack.id, object.id));
        }
        for layer in &object.render_layers {
            check_path(&pack.id, &layer.file)?;
            if !matches!(layer.kind.as_str(), "upright" | "ground") {
                return Err(format!("{} has an unknown layer kind {}", layer.file, layer.kind));
            }
            if !matches!(layer.axis.as_str(), "" | "h" | "v") || !matches!(layer.state.as_str(), "" | "open" | "closed") {
                return Err(format!("{} has an unknown axis or door state", layer.file));
            }
            check_pivot(&layer.file, layer.pivot_px, layer.size_px)?;
            finite_triple(layer.root_offset_world, &layer.file)?;
        }
    }
    for cast in &pack.cast {
        remember(&mut ids, &pack.id, &cast.id)?;
        if !(1..=4_096).contains(&cast.pixels_per_world_unit) {
            return Err(format!("cast {}/{} scale must be between 1 and 4096 pixels per world unit", pack.id, cast.id));
        }
        if cast.suggested_footprint_cells[0] == 0
            || cast.suggested_footprint_cells[1] == 0
            || cast.suggested_footprint_cells[0] > 8
            || cast.suggested_footprint_cells[1] > 8
        {
            return Err(format!("cast {}/{} footprint must be from 1 to 8 cells on a side", pack.id, cast.id));
        }
        for frame in [&cast.views.north, &cast.views.east, &cast.views.south, &cast.views.west] {
            check_path(&pack.id, &frame.file)?;
            check_pivot(&frame.file, frame.pivot_px, frame.size_px)?;
        }
        if let Some(portrait) = &cast.portrait {
            check_path(&pack.id, portrait)?;
        }
    }
    Ok(())
}

pub fn validate_v2_files(pack: &Pack, files: &BTreeMap<String, Vec<u8>>, decode: bool) -> Result<Vec<String>, String> {
    validate_v2_shape(pack)?;
    let mut budget = MAX_DECODED;
    let mut warnings = Vec::new();
    for tile in &pack.tiles {
        for file in &tile.files {
            let image = read_image(files, file, &mut budget, decode)?;
            if image.width != tile.source_size_px[0] || image.height != tile.source_size_px[1] {
                return Err(format!("{file} is {} by {}, not {} by {}", image.width, image.height, tile.source_size_px[0], tile.source_size_px[1]));
            }
            if tile.kind == "overlay" && decode && !image.has_alpha {
                warnings.push(format!("{file} is an overlay with no transparent pixels"));
            }
        }
    }
    for object in &pack.objects {
        if let Some(structure) = &object.structure {
            let image = read_image(files, &structure.texture, &mut budget, decode)?;
            if decode && image.has_alpha {
                return Err(format!("{} is a structure surface and must be opaque", structure.texture));
            }
        }
        for layer in &object.render_layers {
            let image = read_image(files, &layer.file, &mut budget, decode)?;
            if image.width != layer.size_px[0] || image.height != layer.size_px[1] {
                return Err(format!("{} is {} by {}, not the size the pack declares", layer.file, image.width, image.height));
            }
            if decode && layer.kind == "upright" && !image.has_alpha {
                warnings.push(format!("{} has no transparent pixels. Cutout art needs a transparent background", layer.file));
            }
        }
    }
    for cast in &pack.cast {
        for frame in [&cast.views.north, &cast.views.east, &cast.views.south, &cast.views.west] {
            let image = read_image(files, &frame.file, &mut budget, decode)?;
            if image.width != frame.size_px[0] || image.height != frame.size_px[1] {
                return Err(format!("{} is {} by {}, not the size the pack declares", frame.file, image.width, image.height));
            }
            if decode && !image.has_alpha {
                warnings.push(format!("{} has no transparent pixels. Cast art needs a transparent background", frame.file));
            }
        }
        if let Some(portrait) = &cast.portrait {
            let image = read_image(files, portrait, &mut budget, decode)?;
            if image.width == 0 || image.height == 0 {
                return Err(format!("{portrait} is empty"));
            }
        }
    }
    Ok(warnings)
}

pub fn read_tree(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    walk(dir, dir, &mut files)?;
    Ok(files)
}

fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).map_err(|err| err.to_string())?;
        if meta.file_type().is_symlink() {
            return Err(format!("{} is a link", path.display()));
        }
        if path.is_dir() {
            walk(root, &path, files)?;
            continue;
        }
        let name = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if name == "pack.json" {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
        files.insert(name, bytes);
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackReport {
    pub id: String,
    pub name: String,
    pub tiles: usize,
    pub objects: usize,
    pub cast: usize,
    pub warnings: Vec<String>,
}

fn prepare(bytes: &[u8]) -> Result<(Pack, String, BTreeMap<String, Vec<u8>>, Vec<String>), String> {
    if bytes.len() > MAX_COMPRESSED {
        return Err("that pack is larger than 100 MiB compressed".into());
    }
    let (manifest, files) = read_zip(bytes)?;
    let mut pack = crate::assets::parse_pack(&manifest)?;
    if pack.schema_version != 2 {
        return Err("an imported pack must use schema 2".into());
    }
    let warnings = validate_v2_files(&pack, &files, true)?;
    pack.content_hash = content_hash(&manifest, &files);
    Ok((pack, manifest, files, warnings))
}

/// Read a zip and report problems. Nothing is written.
pub fn check_bytes(bytes: &[u8]) -> Result<PackReport, String> {
    let (pack, _, _, warnings) = prepare(bytes)?;
    Ok(PackReport { id: pack.id, name: pack.name, tiles: pack.tiles.len(), objects: pack.objects.len(), cast: pack.cast.len(), warnings })
}

pub fn import_bytes(bytes: &[u8], assets_dir: &Path) -> Result<Pack, String> {
    let (mut pack, manifest, files, _) = prepare(bytes)?;
    install(&pack, &manifest, &files, assets_dir)?;
    pack.base_url = format!("/packs/{}", pack.id);
    Ok(pack)
}

fn read_zip(bytes: &[u8]) -> Result<(String, BTreeMap<String, Vec<u8>>), String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| "that pack is not a valid archive".to_string())?;
    if archive.len() > MAX_FILES {
        return Err(format!("that pack has more than {MAX_FILES} files"));
    }
    let mut manifest = None;
    let mut files = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|_| "that pack is not a valid archive".to_string())?;
        if file.is_dir() {
            continue;
        }
        let raw = file.name().replace('\\', "/");
        let name = safe_zip_name(&raw)?;
        let key = name.to_lowercase();
        if !seen.insert(key) {
            return Err(format!("{name} collides with another path"));
        }
        total = total.saturating_add(file.size());
        if total > MAX_EXPANDED {
            return Err("that pack expands past 512 MiB".into());
        }
        let mut body = Vec::new();
        file.read_to_end(&mut body).map_err(|err| err.to_string())?;
        if name == "pack.json" {
            manifest = Some(String::from_utf8(body).map_err(|_| "pack.json is not text".to_string())?);
        } else if !name.ends_with(".png") {
            return Err(format!("{name} is not a png picture"));
        } else {
            files.insert(name, body);
        }
    }
    let manifest = manifest.ok_or_else(|| "pack.json is missing".to_string())?;
    Ok((manifest, files))
}

fn install(pack: &Pack, manifest: &str, files: &BTreeMap<String, Vec<u8>>, assets_dir: &Path) -> Result<(), String> {
    let _guard = INSTALL_LOCK.lock().map_err(|_| "the importer is busy".to_string())?;
    let packs = assets_dir.join("packs");
    std::fs::create_dir_all(&packs).map_err(|err| err.to_string())?;
    let dest = packs.join(&pack.id);
    if dest.exists() {
        let text = std::fs::read_to_string(dest.join("pack.json")).unwrap_or_default();
        let existing = read_tree(&dest).unwrap_or_default();
        if content_hash(&text, &existing) == pack.content_hash {
            return Ok(());
        }
        return Err(format!("pack {} is already installed with different bytes. Use a new id.", pack.id));
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|time| time.as_nanos()).unwrap_or(0);
    let staging = packs.join(format!(".incoming-{}-{}-{stamp}", pack.id, std::process::id()));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|err| err.to_string())?;
    }
    let installed = (|| {
        std::fs::create_dir(&staging).map_err(|err| err.to_string())?;
        write_file(&staging, Path::new("pack.json"), manifest.as_bytes())?;
        for (name, bytes) in files {
            write_file(&staging, Path::new(name), bytes)?;
        }
        std::fs::rename(&staging, &dest).map_err(|err| err.to_string())?;
        Ok(())
    })();
    if installed.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    installed
}

fn write_file(root: &Path, relative: &Path, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut file = std::fs::File::create(&path).map_err(|err| err.to_string())?;
    file.write_all(bytes).map_err(|err| err.to_string())?;
    Ok(())
}

fn safe_zip_name(raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.starts_with('/') || raw.contains('\\') || raw.contains(':') || raw.contains('\0') {
        return Err(format!("{raw} is not a safe path"));
    }
    let path = Path::new(raw);
    if path.is_absolute() || path.components().any(|part| matches!(part, Component::ParentDir | Component::Prefix(_))) {
        return Err(format!("{raw} is not a safe path"));
    }
    Ok(raw.to_string())
}

fn check_path(pack: &str, file: &str) -> Result<(), String> {
    if file.starts_with("http://") || file.starts_with("https://") {
        return Err(format!("{file} is a remote address, and pack {pack} must carry the picture"));
    }
    safe_zip_name(file).map(|_| ())
}

fn remember(ids: &mut HashSet<String>, pack: &str, id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains('/') {
        return Err(format!("pack {pack} has an empty or slashed id"));
    }
    if !ids.insert(id.to_string()) {
        return Err(format!("pack {pack} repeats id {id}"));
    }
    Ok(())
}

fn check_pivot(file: &str, pivot: [u32; 2], size: [u32; 2]) -> Result<(), String> {
    if size[0] == 0 || size[1] == 0 || size[0] > MAX_SIDE || size[1] > MAX_SIDE {
        return Err(format!("{file} must be between 1 and {MAX_SIDE} pixels on a side"));
    }
    // Pivot is a pixel inside the image, counted from the top-left. The last
    // pixel is size - 1. The center of that pixel sits on the object's root.
    if pivot[0] >= size[0] || pivot[1] >= size[1] {
        return Err(format!("{file} pivot is outside its image. Count pixels from the top-left, and keep the pivot on a pixel"));
    }
    Ok(())
}

fn finite_pair(value: [f32; 2], what: &str) -> Result<(), String> {
    if value.iter().all(|item| item.is_finite() && item.abs() <= 64.0) {
        Ok(())
    } else {
        Err(format!("{what} must be a finite offset inside 64 cells"))
    }
}

fn finite_triple(value: [f32; 3], what: &str) -> Result<(), String> {
    if value.iter().all(|item| item.is_finite() && item.abs() <= 64.0) {
        Ok(())
    } else {
        Err(format!("{what} offset must be finite and inside 64 world units"))
    }
}

fn finite_polygon(points: &[[f32; 2]], what: &str) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }
    if points.len() < 3 || points.len() > 32 {
        return Err(format!("{what} needs between 3 and 32 corners"));
    }
    for point in points {
        finite_pair(*point, what)?;
    }
    Ok(())
}

struct Inspected {
    width: u32,
    height: u32,
    has_alpha: bool,
}

fn read_image(files: &BTreeMap<String, Vec<u8>>, name: &str, budget: &mut u64, decode: bool) -> Result<Inspected, String> {
    let bytes = files.get(name).ok_or_else(|| format!("{name} is missing"))?;
    let (width, height) = png_size(bytes).map_err(|_| format!("{name} is not a readable png"))?;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!("{name} is {width} by {height}, and pictures must be at most {MAX_SIDE} pixels on a side"));
    }
    let pixels = u64::from(width) * u64::from(height);
    let needed = pixels.saturating_mul(4);
    if needed > *budget {
        return Err(format!("{name} would push decoded pictures past the memory budget"));
    }
    *budget -= needed;
    if !decode {
        return Ok(Inspected { width, height, has_alpha: true });
    }
    let has_alpha = decode_rgba(bytes, needed).map_err(|_| format!("{name} is not a readable png"))?;
    Ok(Inspected { width, height, has_alpha })
}

fn png_size(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err("not a png".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| "not a png")?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| "not a png")?);
    Ok((width, height))
}

fn decode_rgba(bytes: &[u8], needed: u64) -> Result<bool, String> {
    use image::ImageDecoder;
    let mut decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes)).map_err(|err| err.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(needed.saturating_add(1024 * 1024));
    decoder.set_limits(limits).map_err(|err| err.to_string())?;
    let image = image::DynamicImage::from_decoder(decoder).map_err(|err| err.to_string())?;
    let rgba = image.to_rgba8();
    Ok(rgba.pixels().any(|pixel| pixel[3] < 250))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    fn png(width: u32, height: u32) -> Vec<u8> {
        png_colored(width, height, 255)
    }

    fn png_colored(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        let mut pixels = vec![80u8; (width * height * 4) as usize];
        if !pixels.is_empty() {
            pixels[3] = alpha;
        }
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&pixels, width, height, ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }

    fn manifest(extra: &str) -> String {
        format!(
            r#"{{
                "schemaVersion": 2,
                "id": "demo",
                "name": "Demo",
                "visualProfile": "dimetric-30-v1",
                "pixelsPerCell": 128,
                "filtering": "linear",
                "tiles": [{{
                    "id": "floor",
                    "name": "Floor",
                    "kind": "ground",
                    "sourceSizePx": [128, 128],
                    "coverageCells": [1, 1],
                    "files": ["tiles/floor.png"]
                }}],
                "objects": [{{
                    "id": "barrel",
                    "name": "Barrel",
                    "kind": "static",
                    "placement": "cell",
                    "footprintCells": [1, 1],
                    "pixelsPerWorldUnit": 128,
                    "renderLayers": [{{
                        "kind": "upright",
                        "file": "objects/barrel.png",
                        "sizePx": [32, 48],
                        "pivotPx": [16, 46]
                    }}]
                }}],
                "cast": [{{
                    "id": "bandit",
                    "name": "Bandit",
                    "pixelsPerWorldUnit": 128,
                    "views": {{
                        "north": {{"file": "cast/bandit/north.png", "sizePx": [32, 64], "pivotPx": [16, 60]}},
                        "east": {{"file": "cast/bandit/east.png", "sizePx": [32, 64], "pivotPx": [16, 60]}},
                        "south": {{"file": "cast/bandit/south.png", "sizePx": [32, 64], "pivotPx": [16, 60]}},
                        "west": {{"file": "cast/bandit/west.png", "sizePx": [32, 64], "pivotPx": [16, 60]}}
                    }}
                    {extra}
                }}]
            }}"#
        )
    }

    fn zip_with(manifest: &str, files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("pack.json", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(manifest.as_bytes()).unwrap();
        for (name, bytes) in files {
            writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn sample_files() -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("tiles/floor.png", png(128, 128)),
            ("objects/barrel.png", png_colored(32, 48, 0)),
            ("cast/bandit/north.png", png_colored(32, 64, 0)),
            ("cast/bandit/east.png", png_colored(32, 64, 0)),
            ("cast/bandit/south.png", png_colored(32, 64, 0)),
            ("cast/bandit/west.png", png_colored(32, 64, 0)),
        ]
    }

    fn temp_assets() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("longtable-pack-{}-{}", std::process::id(), uuid_bit()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("packs")).unwrap();
        dir
    }

    fn uuid_bit() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
    }

    #[test]
    fn a_check_reports_the_pack_and_installs_nothing() {
        let report = check_bytes(&zip_with(&manifest(""), &sample_files())).unwrap();
        assert_eq!(report.id, "demo");
        assert_eq!(report.tiles, 1);
        assert_eq!(report.objects, 1);
        assert_eq!(report.cast, 1);
        let err = check_bytes(&zip_with(&manifest(""), &[("tiles/floor.png", png(128, 128))])).unwrap_err();
        assert!(err.contains("missing"), "{err}");
        assert!(!std::env::temp_dir().join("packs").join("demo").exists());
    }

    #[test]
    fn a_complete_pack_installs_tiles_objects_and_cast() {
        let dir = temp_assets();
        let bytes = zip_with(&manifest(""), &sample_files());
        let pack = import_bytes(&bytes, &dir).unwrap();
        assert_eq!(pack.tiles.len(), 1);
        assert_eq!(pack.objects.len(), 1);
        assert_eq!(pack.cast.len(), 1);
        let catalog = crate::assets::load_catalog(&dir).unwrap();
        assert!(catalog.packs.iter().any(|item| item.id == "demo" && item.cast.len() == 1));
        assert!(catalog.warnings.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_frame_installs_nothing() {
        let dir = temp_assets();
        let mut files = sample_files();
        files.retain(|(name, _)| *name != "cast/bandit/north.png");
        let err = import_bytes(&zip_with(&manifest(""), &files), &dir).unwrap_err();
        assert!(err.contains("cast/bandit/north.png is missing"), "{err}");
        assert!(!dir.join("packs/demo").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_portrait_is_allowed() {
        let pack = crate::assets::parse_pack(&manifest("")).unwrap();
        assert!(pack.cast[0].portrait.is_none());
    }

    #[test]
    fn the_wrong_ground_size_is_rejected() {
        let mut files = sample_files();
        files[0].1 = png(64, 64);
        let err = import_bytes(&zip_with(&manifest(""), &files), &temp_assets()).unwrap_err();
        assert!(err.contains("tiles/floor.png"), "{err}");
    }

    #[test]
    fn a_repeated_id_is_rejected() {
        let text = manifest("").replace("\"id\": \"barrel\"", "\"id\": \"floor\"");
        let err = import_bytes(&zip_with(&text, &sample_files()), &temp_assets()).unwrap_err();
        assert!(err.contains("repeats id floor"), "{err}");
    }

    #[test]
    fn an_unknown_profile_is_rejected() {
        let text = manifest("").replace("dimetric-30-v1", "classic");
        let err = import_bytes(&zip_with(&text, &sample_files()), &temp_assets()).unwrap_err();
        assert!(err.contains("classic"), "{err}");
    }

    #[test]
    fn an_unsafe_path_is_rejected() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("../pack.json", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"{}").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let err = import_bytes(&bytes, &temp_assets()).unwrap_err();
        assert!(err.contains("safe path") || err.contains("pack.json is missing"), "{err}");
    }

    #[test]
    fn a_conflicting_reimport_keeps_the_first_pack() {
        let dir = temp_assets();
        import_bytes(&zip_with(&manifest(""), &sample_files()), &dir).unwrap();
        let mut changed = sample_files();
        changed[1].1 = png(32, 40);
        let text = manifest("").replace("\"sizePx\": [32, 48]", "\"sizePx\": [32, 40]").replace("\"pivotPx\": [16, 46]", "\"pivotPx\": [16, 38]");
        let err = import_bytes(&zip_with(&text, &changed), &dir).unwrap_err();
        assert!(err.contains("already installed"), "{err}");
        let catalog = crate::assets::load_catalog(&dir).unwrap();
        assert_eq!(catalog.packs.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_pack_does_not_hide_a_good_one() {
        let dir = temp_assets();
        import_bytes(&zip_with(&manifest(""), &sample_files()), &dir).unwrap();
        std::fs::create_dir_all(dir.join("packs/broken")).unwrap();
        std::fs::write(dir.join("packs/broken/pack.json"), "{\"schemaVersion\":2,\"id\":\"broken\"}").unwrap();
        let catalog = crate::assets::load_catalog(&dir).unwrap();
        assert!(catalog.packs.iter().any(|pack| pack.id == "demo"));
        assert!(catalog.warnings.iter().any(|warning| warning.contains("broken")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pivot_outside_the_image_is_rejected() {
        let dir = temp_assets();
        let text = manifest("").replace("\"pivotPx\": [16, 46]", "\"pivotPx\": [32, 46]");
        let err = import_bytes(&zip_with(&text, &sample_files()), &dir).unwrap_err();
        assert!(err.contains("pivot is outside"), "{err}");
        assert!(!dir.join("packs/demo").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_zero_object_scale_is_rejected() {
        let dir = temp_assets();
        let text = manifest("").replacen("\"pixelsPerWorldUnit\": 128", "\"pixelsPerWorldUnit\": 0", 1);
        let err = import_bytes(&zip_with(&text, &sample_files()), &dir).unwrap_err();
        assert!(err.contains("scale must be between 1 and 4096"), "{err}");
        assert!(!dir.join("packs/demo").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn architecture_profiles_validate_dimensions_paths_and_matching_edge_kinds() {
        let mut value: serde_json::Value = serde_json::from_str(&manifest("")).unwrap();
        value["objects"][0]["kind"] = "wall".into();
        value["objects"][0]["placement"] = "edge".into();
        value["objects"][0]["structure"] = serde_json::json!({
            "profile": "log-wall", "height": 2.65, "thickness": 0.24, "texture": "objects/wood.png"
        });
        let mut pack: Pack = serde_json::from_value(value).unwrap();
        validate_v2_shape(&pack).unwrap();
        let serialized = serde_json::to_value(&pack).unwrap();
        assert_eq!(serialized["objects"][0]["structure"]["height"], 2.65_f32);
        pack.objects[0].kind = "static".into();
        assert!(validate_v2_shape(&pack).unwrap_err().contains("edge wall"));
        pack.objects[0].kind = "wall".into();
        pack.objects[0].structure.as_mut().unwrap().height = f32::INFINITY;
        assert!(validate_v2_shape(&pack).unwrap_err().contains("height"));
        pack.objects[0].structure.as_mut().unwrap().height = 2.65;
        pack.objects[0].structure.as_mut().unwrap().texture = "../wood.png".into();
        assert!(validate_v2_shape(&pack).is_err());
    }

    #[test]
    fn an_oversized_image_is_rejected() {
        let dir = temp_assets();
        let mut files = sample_files();
        let mut header = b"\x89PNG\r\n\x1a\n".to_vec();
        header.extend_from_slice(&13u32.to_be_bytes());
        header.extend_from_slice(b"IHDR");
        header.extend_from_slice(&3_000u32.to_be_bytes());
        header.extend_from_slice(&32u32.to_be_bytes());
        files[1].1 = header;
        let err = import_bytes(&zip_with(&manifest(""), &files), &dir).unwrap_err();
        assert!(err.contains("at most"), "{err}");
        assert!(!dir.join("packs/demo").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_corrupt_image_is_rejected() {
        let dir = temp_assets();
        let mut files = sample_files();
        files[1].1 = b"not a png".to_vec();
        let err = import_bytes(&zip_with(&manifest(""), &files), &dir).unwrap_err();
        assert!(err.contains("not a readable png"), "{err}");
        assert!(!dir.join("packs/demo").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_flat_scene_imports_compiles_publishes_and_reloads() {
        let dir = temp_assets();
        let manifest = r#"{
            "schemaVersion": 2,
            "id": "gate",
            "name": "Gate",
            "visualProfile": "dimetric-30-v1",
            "pixelsPerCell": 128,
            "filtering": "linear",
            "tiles": [{
                "id": "floor",
                "name": "Floor",
                "kind": "ground",
                "sourceSizePx": [128, 128],
                "coverageCells": [1, 1],
                "files": ["tiles/floor.png"]
            }],
            "objects": [{
                "id": "barrel",
                "name": "Barrel",
                "kind": "static",
                "placement": "cell",
                "footprintCells": [1, 1],
                "blocksMovement": true,
                "blocksVision": false,
                "pixelsPerWorldUnit": 128,
                "renderLayers": [{
                    "kind": "upright",
                    "file": "objects/barrel.png",
                    "sizePx": [32, 48],
                    "pivotPx": [16, 46]
                }]
            }, {
                "id": "oak",
                "name": "Oak",
                "kind": "static",
                "placement": "cell",
                "footprintCells": [1, 1],
                "blocksMovement": true,
                "blocksVision": true,
                "pixelsPerWorldUnit": 128,
                "collisionPolygon": [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]],
                "visionVolumes": [{ "polygon": [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]], "baseHeight": 0, "height": 2 }],
                "renderLayers": [{
                    "kind": "upright",
                    "file": "objects/oak.png",
                    "sizePx": [96, 128],
                    "pivotPx": [48, 120]
                }]
            }, {
                "id": "gate",
                "name": "Gate",
                "kind": "door",
                "placement": "edge",
                "footprintCells": [1, 1],
                "blocksMovement": true,
                "blocksVision": true,
                "pixelsPerWorldUnit": 128,
                "renderLayers": [
                    { "kind": "upright", "file": "objects/door.png", "sizePx": [16, 32], "pivotPx": [8, 30], "axis": "h", "state": "closed" },
                    { "kind": "upright", "file": "objects/door.png", "sizePx": [16, 32], "pivotPx": [8, 30], "axis": "v", "state": "closed" },
                    { "kind": "upright", "file": "objects/door.png", "sizePx": [16, 32], "pivotPx": [8, 30], "axis": "h", "state": "open" },
                    { "kind": "upright", "file": "objects/door.png", "sizePx": [16, 32], "pivotPx": [8, 30], "axis": "v", "state": "open" }
                ]
            }],
            "cast": [{
                "id": "bandit",
                "name": "Bandit",
                "pixelsPerWorldUnit": 128,
                "views": {
                    "north": {"file": "cast/bandit/north.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "east": {"file": "cast/bandit/east.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "south": {"file": "cast/bandit/south.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
                    "west": {"file": "cast/bandit/west.png", "sizePx": [32, 64], "pivotPx": [16, 60]}
                }
            }]
        }"#;
        let files = vec![
            ("tiles/floor.png", png(128, 128)),
            ("objects/barrel.png", png_colored(32, 48, 0)),
            ("objects/oak.png", png_colored(96, 128, 0)),
            ("objects/door.png", png_colored(16, 32, 0)),
            ("cast/bandit/north.png", png_colored(32, 64, 0)),
            ("cast/bandit/east.png", png_colored(32, 64, 0)),
            ("cast/bandit/south.png", png_colored(32, 64, 0)),
            ("cast/bandit/west.png", png_colored(32, 64, 0)),
        ];
        import_bytes(&zip_with(manifest, &files), &dir).unwrap();
        let catalog = crate::assets::load_catalog(&dir).unwrap();
        assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
        assert!(catalog.tile("gate/floor").is_some());
        assert!(catalog.cast("gate/bandit").is_some());

        let mut layout = crate::map::MapLayout::blank(12, 12, "gate/floor");
        layout.props.push(crate::map::layout::PlacedProp { id: "barrel".into(), asset: "gate/barrel".into(), x: 3.0, y: 4.0, mirror: false });
        layout.props.push(crate::map::layout::PlacedProp { id: "oak".into(), asset: "gate/oak".into(), x: 6.0, y: 6.0, mirror: false });
        layout.pieces.push(crate::map::layout::EdgePiece {
            id: "door".into(),
            asset: "gate/gate".into(),
            x: 2,
            y: 2,
            axis: crate::map::layout::EdgeAxis::H,
            kind: crate::map::layout::PieceKind::Door,
            open: false,
        });
        let compiled = crate::map::compile::compile(&layout, &catalog).unwrap();
        assert!(compiled.walls.is_empty(), "objects do not become walls");
        assert_eq!(compiled.move_blockers.len(), 8, "the barrel and the oak both block walking");
        assert!(compiled.sight.iter().any(|block| (block.top - 2.0).abs() < 0.01), "the oak blocks sight at its authored height");
        assert!(compiled.sight.iter().all(|block| block.a.x > 5.0), "the barrel does not block sight");
        assert_eq!(compiled.doors.len(), 1);
        assert!(!compiled.doors[0].open);

        let stairs = r#"{"schemaVersion":2,"id":"steps","name":"Steps","visualProfile":"dimetric-30-v1","pixelsPerCell":128,"filtering":"linear","tiles":[],"objects":[{"id":"stair","name":"Stair","kind":"stairs","placement":"connection","footprintCells":[1,1],"pixelsPerWorldUnit":128,"renderLayers":[{"kind":"upright","file":"objects/stair.png","sizePx":[16,32],"pivotPx":[8,30]}]}]}"#;
        let stairs_dir = temp_assets();
        import_bytes(&zip_with(stairs, &[("objects/stair.png", png_colored(16, 32, 255))]), &stairs_dir).unwrap();
        let stair_catalog = crate::assets::load_catalog(&stairs_dir).unwrap();
        assert!(stair_catalog.object("steps/stair").is_some(), "{:?}", stair_catalog.warnings);
        let _ = std::fs::remove_dir_all(&stairs_dir);

        let mut raised = layout.clone();
        let mut levels = vec![0_u8; 144];
        levels[0] = 1;
        raised.levels = Some(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &levels));
        raised.validate().unwrap();
        levels[0] = 3;
        raised.levels = Some(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &levels));
        let err = raised.validate().unwrap_err();
        assert!(err.contains("0, 1, or 2"), "{err}");

        let mut table = crate::session::state::Table::new();
        table.passphrase = "table".into();
        table.gm_key = "AB12CD".into();
        let gm_seat = table.take_seat("Mina", "table", true, Some("AB12CD"), None).unwrap();
        let player_seat = table.take_seat("Ada", "table", false, None, None).unwrap();
        let gm = crate::session::state::Viewer {
            player_id: gm_seat.player_id.clone(),
            role: crate::net::protocol::Role::Gm,
            view_as: None,
            session_token: "gm".into(),
        };
        let player = crate::session::state::Viewer {
            player_id: player_seat.player_id.clone(),
            role: crate::net::protocol::Role::Player,
            view_as: None,
            session_token: "player".into(),
        };
        let first = table.publish_built(&gm, "Gate".into(), layout.clone(), compiled, None).unwrap();
        let again = crate::map::compile::compile(&layout, &catalog).unwrap();
        let second = table.publish_built(&gm, "Gate".into(), layout.clone(), again, Some(first.clone())).unwrap();
        assert_eq!(first, second);
        table.command(&gm, crate::net::protocol::ClientMsg::CreateToken {
            name: "Ada".into(),
            x: 2.0,
            y: 2.0,
            owner_id: Some(player_seat.player_id.clone()),
            hidden: false,
            vision_radius: 6.0,
            grants_vision: true,
            color: "#d4a656".into(),
            character_id: None,
            cast_asset: Some("gate/bandit".into()),
            facing: Some("east".into()),
        }, None).unwrap();
        table.command(&gm, crate::net::protocol::ClientMsg::CreateToken {
            name: "Lurker".into(),
            x: 8.0,
            y: 8.0,
            owner_id: None,
            hidden: true,
            vision_radius: 0.0,
            grants_vision: false,
            color: "#444444".into(),
            character_id: None,
            cast_asset: Some("gate/bandit".into()),
            facing: None,
        }, None).unwrap();
        let gm_view = crate::session::permissions::snapshot(&table, &gm);
        let player_view = crate::session::permissions::snapshot(&table, &player);
        assert!(gm_view.tokens.iter().any(|token| token.cast_asset.as_deref() == Some("gate/bandit") && token.facing == "east"));
        assert!(player_view.tokens.iter().any(|token| token.name == "Ada" && token.cast_asset.as_deref() == Some("gate/bandit")));
        assert!(player_view.tokens.iter().all(|token| token.name != "Lurker"));

        let save_dir = temp_assets();
        crate::storage::write(&table, &save_dir).unwrap();
        let mut restored = crate::session::state::Table::new();
        crate::storage::load_into(&mut restored, &save_dir);
        assert_eq!(restored.scene.as_ref().map(|scene| scene.id.as_str()), Some(first.as_str()));
        assert!(restored.tokens.iter().any(|token| token.cast_asset.as_deref() == Some("gate/bandit") && token.facing == "east"));
        assert!(restored.tokens.iter().any(|token| token.hidden && token.name == "Lurker"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&save_dir);
    }
}
