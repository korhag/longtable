// ------------------------------------------------------------
// Module: asset_import
// Purpose: Check a schema-2 pack and install it beside the
//          others. A bad archive installs nothing.
// ------------------------------------------------------------

use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::assets::Pack;
use crate::map::DIMETRIC_PROFILE;

pub const MAX_COMPRESSED: usize = 100 * 1024 * 1024;
pub const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
pub const MAX_FILES: usize = 2_000;
pub const MAX_SIDE: u32 = 2_048;

pub fn revision_of(packs: &[Pack]) -> u64 {
    let mut hasher = Sha256::new();
    for pack in packs {
        hasher.update(pack.id.as_bytes());
        hasher.update(pack.content_hash.as_bytes());
    }
    let digest = hasher.finalize();
    u64::from_le_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

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
        if tile.files.is_empty() {
            return Err(format!("tile {}/{} has no file", pack.id, tile.id));
        }
        for file in &tile.files {
            check_path(&pack.id, file)?;
        }
    }
    for object in &pack.objects {
        remember(&mut ids, &pack.id, &object.id)?;
        if !matches!(object.kind.as_str(), "static" | "wall" | "door" | "stairs") {
            return Err(format!("object {}/{} has an unknown kind {}", pack.id, object.id, object.kind));
        }
        if !matches!(object.placement.as_str(), "cell" | "edge" | "connection") {
            return Err(format!("object {}/{} has an unknown placement {}", pack.id, object.id, object.placement));
        }
        if object.render_layers.is_empty() {
            return Err(format!("object {}/{} has no picture", pack.id, object.id));
        }
        for layer in &object.render_layers {
            check_path(&pack.id, &layer.file)?;
            if layer.pivot_px[0] >= layer.size_px[0] || layer.pivot_px[1] >= layer.size_px[1] {
                return Err(format!("{} pivot is outside its image", layer.file));
            }
        }
    }
    for cast in &pack.cast {
        remember(&mut ids, &pack.id, &cast.id)?;
        for frame in [&cast.views.north, &cast.views.east, &cast.views.south, &cast.views.west] {
            check_path(&pack.id, &frame.file)?;
            if frame.pivot_px[0] >= frame.size_px[0] || frame.pivot_px[1] >= frame.size_px[1] {
                return Err(format!("{} pivot is outside its image", frame.file));
            }
        }
        if let Some(portrait) = &cast.portrait {
            check_path(&pack.id, portrait)?;
        }
    }
    Ok(())
}

pub fn validate_v2_files(pack: &Pack, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    validate_v2_shape(pack)?;
    for tile in &pack.tiles {
        for file in &tile.files {
            let (width, height) = decoded(files, file)?;
            if width != tile.source_size_px[0] || height != tile.source_size_px[1] {
                return Err(format!("{file} is {width} by {height}, not {} by {}", tile.source_size_px[0], tile.source_size_px[1]));
            }
            if width > MAX_SIDE || height > MAX_SIDE {
                return Err(format!("{file} is larger than {MAX_SIDE} pixels"));
            }
        }
    }
    for object in &pack.objects {
        for layer in &object.render_layers {
            let (width, height) = decoded(files, &layer.file)?;
            if width != layer.size_px[0] || height != layer.size_px[1] {
                return Err(format!("{} is {width} by {height}, not the size the pack declares", layer.file));
            }
        }
    }
    for cast in &pack.cast {
        for frame in [&cast.views.north, &cast.views.east, &cast.views.south, &cast.views.west] {
            let (width, height) = decoded(files, &frame.file)?;
            if width != frame.size_px[0] || height != frame.size_px[1] {
                return Err(format!("{} is {width} by {height}, not the size the pack declares", frame.file));
            }
        }
        if let Some(portrait) = &cast.portrait {
            if !files.contains_key(portrait) {
                return Err(format!("{portrait} is missing"));
            }
        }
    }
    Ok(())
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

pub fn import_bytes(bytes: &[u8], assets_dir: &Path) -> Result<Pack, String> {
    if bytes.len() > MAX_COMPRESSED {
        return Err("that pack is larger than 100 MiB compressed".into());
    }
    let (manifest, files) = read_zip(bytes)?;
    let mut pack = crate::assets::parse_pack(&manifest)?;
    if pack.schema_version != 2 {
        return Err("an imported pack must use schema 2".into());
    }
    validate_v2_files(&pack, &files)?;
    pack.content_hash = content_hash(&manifest, &files);
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
    let staging = packs.join(format!(".incoming-{}", pack.id));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|err| err.to_string())?;
    }
    std::fs::create_dir(&staging).map_err(|err| err.to_string())?;
    write_file(&staging, Path::new("pack.json"), manifest.as_bytes())?;
    for (name, bytes) in files {
        write_file(&staging, Path::new(name), bytes)?;
    }
    std::fs::rename(&staging, &dest).map_err(|err| err.to_string())?;
    Ok(())
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

fn decoded(files: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<(u32, u32), String> {
    let bytes = files.get(name).ok_or_else(|| format!("{name} is missing"))?;
    let image = image::load_from_memory(bytes).map_err(|_| format!("{name} is not a readable png"))?;
    Ok((image.width(), image.height()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![80u8; (width * height * 4) as usize];
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
            ("objects/barrel.png", png(32, 48)),
            ("cast/bandit/north.png", png(32, 64)),
            ("cast/bandit/east.png", png(32, 64)),
            ("cast/bandit/south.png", png(32, 64)),
            ("cast/bandit/west.png", png(32, 64)),
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
}
