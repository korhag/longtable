//! Crop and align an already-drawn 4 × 6 atlas. This never generates poses or removes backgrounds.
//! Run with: cargo run --manifest-path server/Cargo.toml --example normalize_animation_atlas -- --help

use std::path::{Path, PathBuf};

use image::{ImageBuffer, Rgba, Rgba32FImage, RgbaImage, imageops};
use serde::Serialize;

const COLUMNS: u32 = 4;
const ROWS: u32 = 6;
const CELL: u32 = 256;
const PIVOT: [u32; 2] = [128, 224];
const INK_ALPHA: u8 = 16;
const FOOT_ALPHA: u8 = 128;
const COMPONENT_ALPHA: u8 = 16;
const ACTIONS: [&str; 6] = ["idle", "melee", "ranged", "move", "hurt", "spell"];

#[derive(Clone)]
struct Options {
    input: PathBuf,
    reference: PathBuf,
    output: PathBuf,
    report: PathBuf,
    pixels_per_world_unit: f64,
    reference_pixels_per_world_unit: f64,
    component_alpha: u8,
    force: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Rect { x: u32, y: u32, width: u32, height: u32 }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameReport {
    action: &'static str,
    frame: u32,
    source_region: Rect,
    source_crop: Rect,
    source_foot: [f64; 2],
    resized_size: [u32; 2],
    destination: Rect,
    core_pixels: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    status: &'static str,
    input: String,
    reference: String,
    output: String,
    source_size: [u32; 2],
    output_size: [u32; 2],
    frame_size_px: [u32; 2],
    pivot_px: [u32; 2],
    pixels_per_world_unit: f64,
    requested_pixels_per_world_unit: f64,
    reference_visible_height: u32,
    target_standing_height: f64,
    source_idle_height: f64,
    requested_scale: f64,
    applied_scale: f64,
    reassigned_soft_pixels: u32,
    component_alpha: u8,
    frames: Vec<FrameReport>,
    warnings: Vec<String>,
}

struct Sprite {
    row: u32,
    column: u32,
    region: Rect,
    crop: Rect,
    foot: [f64; 2],
    core_pixels: u32,
    visible_height: u32,
}

fn main() {
    match arguments().and_then(run) {
        Ok(report) => println!("{}", serde_json::json!({
            "status": report.status, "output": report.output, "frames": report.frames.len(),
            "scale": report.applied_scale, "warnings": report.warnings,
        })),
        Err(error) => { eprintln!("{}", serde_json::json!({"status": "rejected", "error": error})); std::process::exit(2); }
    }
}

fn arguments() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let (mut input, mut reference, mut output, mut report) = (None, None, None, None);
    let (mut ppu, mut reference_ppu, mut component_alpha, mut force) = (105.0, 128.0, COMPONENT_ALPHA, false);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!("Normalize an already-drawn transparent 4x6 PNG atlas; preserve the source.\n\
                --input RAW.png --reference STATIC.png --output ALIGNED.png [--report STATS.json]\n\
                [--pixels-per-world-unit 105] [--reference-pixels-per-world-unit 128]\n\
                [--component-alpha 16] [--force]\n\
                Output: 1024x1536, 256x256 cells, foot pivot [optimized X,224].\n\
                Extracts24 separate connected figures; detached small effects follow their nearest figure.\n\
                Use report pixelsPerWorldUnit and pivotPx as per-view overrides to preserve body size.");
            std::process::exit(0);
        }
        if arg == "--force" { force = true; continue; }
        let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
        match arg.as_str() {
            "--input" => input = Some(PathBuf::from(value)),
            "--reference" => reference = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--report" => report = Some(PathBuf::from(value)),
            "--pixels-per-world-unit" => ppu = value.parse().map_err(|_| "invalid pixels-per-world-unit")?,
            "--reference-pixels-per-world-unit" => reference_ppu = value.parse().map_err(|_| "invalid reference-pixels-per-world-unit")?,
            "--component-alpha" => component_alpha = value.parse().map_err(|_| "invalid component-alpha")?,
            _ => return Err(format!("unknown option {arg}")),
        }
    }
    let output = output.ok_or("--output is required")?;
    if !f64::is_finite(ppu) || !(1.0..=4096.0).contains(&ppu)
        || !f64::is_finite(reference_ppu) || !(1.0..=4096.0).contains(&reference_ppu) {
        return Err("pixel scales must be finite and between 1 and 4096".into());
    }
    if component_alpha >= FOOT_ALPHA {
        return Err("component-alpha must be below128".into());
    }
    Ok(Options {
        input: input.ok_or("--input is required")?, reference: reference.ok_or("--reference is required")?,
        report: report.unwrap_or_else(|| output.with_extension("json")), output,
        pixels_per_world_unit: ppu, reference_pixels_per_world_unit: reference_ppu,
        component_alpha, force,
    })
}

fn read_png(path: &Path) -> Result<RgbaImage, String> {
    let reader = image::ImageReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?
        .with_guessed_format().map_err(|e| e.to_string())?;
    if reader.format() != Some(image::ImageFormat::Png) { return Err(format!("{} is not a PNG", path.display())); }
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 || u64::from(width) * u64::from(height) > 16_777_216 {
        return Err(format!("{} exceeds the 8192-side/16-megapixel input limit", path.display()));
    }
    let mut reader = image::ImageReader::open(path).map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map(|image| image.to_rgba8()).map_err(|e| e.to_string())
}

fn run(options: Options) -> Result<Report, String> {
    for destination in [&options.output, &options.report] {
        if destination.exists() && !options.force { return Err(format!("{} exists; choose a new output or --force", destination.display())); }
        for source in [&options.input, &options.reference] {
            if destination == source || (destination.exists() && destination.canonicalize().ok() == source.canonicalize().ok()) {
                return Err("output and report must not replace either source image".into());
            }
        }
    }
    if options.output == options.report { return Err("image and report need different output paths".into()); }
    let source = read_png(&options.input)?;
    let reference = read_png(&options.reference)?;
    let (output, report) = normalize(&source, &reference, &options)?;
    for destination in [&options.output, &options.report] {
        if let Some(parent) = destination.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    output.save_with_format(&options.output, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    std::fs::write(&options.report, serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(report)
}

fn normalize(source: &RgbaImage, reference: &RgbaImage, options: &Options) -> Result<(RgbaImage, Report), String> {
    let (width, height) = source.dimensions();
    if width % COLUMNS != 0 || height % ROWS != 0 || width / COLUMNS < 16 || height / ROWS < 16 {
        return Err("source dimensions must divide into four columns and six rows, at least16px per nominal cell".into());
    }
    let (mut sprites, component_owners, mut warnings) = extract_components(source, options.component_alpha)?;
    // Components can have overlapping bounding boxes. Keep their actual pixel masks,
    // and associate only the very faint remaining fringe with the closest component.
    let (owners, reassigned_soft_pixels) = assign_soft_edges(source, component_owners);
    let mut owned_bounds = vec![[u32::MAX, u32::MAX, 0, 0]; sprites.len()];
    for y in 0..height { for x in 0..width {
        if source.get_pixel(x, y)[3] > INK_ALPHA {
            let bounds = &mut owned_bounds[owners[(y * width + x) as usize] as usize];
            bounds[0] = bounds[0].min(x); bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x); bounds[3] = bounds[3].max(y);
        }
    } }
    for (sprite, bounds) in sprites.iter_mut().zip(owned_bounds) {
        let left = bounds[0].saturating_sub(2);
        let top = bounds[1].saturating_sub(2);
        sprite.visible_height = bounds[3] - bounds[1] + 1;
        sprite.crop = Rect { x: left, y: top, width: (bounds[2] + 3).min(width) - left, height: (bounds[3] + 3).min(height) - top };
    }
    let reference_bounds = bounds(reference, Rect { x: 0, y: 0, width: reference.width(), height: reference.height() }, INK_ALPHA)
        .ok_or("reference has no visible art")?;
    let mut idle_heights: Vec<f64> = sprites.iter().take(4).map(|sprite| f64::from(sprite.visible_height)).collect();
    idle_heights.sort_by(f64::total_cmp);
    let idle_height = (idle_heights[1] + idle_heights[2]) / 2.0;
    let target_height = f64::from(reference_bounds.height) * options.pixels_per_world_unit / options.reference_pixels_per_world_unit;
    let requested_scale = target_height / idle_height;
    let mut scale = requested_scale;
    let max_left = sprites.iter().map(|sprite| sprite.foot[0] - f64::from(sprite.crop.x) + 1.0).fold(0.0, f64::max);
    let max_right = sprites.iter().map(|sprite| f64::from(sprite.crop.x + sprite.crop.width) - sprite.foot[0]).fold(0.0, f64::max);
    scale = scale.min(224.0 / (max_left + max_right));
    for sprite in &sprites {
        let crop = sprite.crop;
        // Fit the whole crop and keep the anchored feet from pushing asymmetric weapons off-canvas.
        for (available, extent) in [
            (224.0, f64::from(crop.width)), (208.0, f64::from(crop.height)),
            (208.0, sprite.foot[1] - f64::from(crop.y) + 1.0),
            (16.0, f64::from(crop.y + crop.height) - sprite.foot[1]),
        ] { if extent > 0.0 { scale = scale.min(available / extent); } }
    }
    if !scale.is_finite() || scale <= 0.0 { return Err("could not derive a positive uniform scale".into()); }
    let pivot = [(16.0 + max_left * scale).round().clamp(16.0, 240.0) as u32, PIVOT[1]];
    let effective_ppu = options.pixels_per_world_unit * scale / requested_scale;
    let mut output = RgbaImage::new(CELL * COLUMNS, CELL * ROWS);
    let mut frames = Vec::new();
    for (index, sprite) in sprites.into_iter().enumerate() {
        let crop = sprite.crop;
        let image = RgbaImage::from_fn(crop.width, crop.height, |x, y| {
            if owners[((crop.y + y) * width + crop.x + x) as usize] as usize == index {
                *source.get_pixel(crop.x + x, crop.y + y)
            } else { Rgba([0, 0, 0, 0]) }
        });
        let size = [(f64::from(crop.width) * scale).round().max(1.0) as u32,
            (f64::from(crop.height) * scale).round().max(1.0) as u32];
        let resized = resize_alpha(&image, size[0], size[1]);
        let mapped_foot = [
            (sprite.foot[0] - f64::from(crop.x) + 0.5) * f64::from(size[0]) / f64::from(crop.width) - 0.5,
            (sprite.foot[1] - f64::from(crop.y) + 0.5) * f64::from(size[1]) / f64::from(crop.height) - 0.5,
        ];
        let offset = [(f64::from(pivot[0]) - mapped_foot[0]).round() as i64, (f64::from(pivot[1]) - mapped_foot[1]).round() as i64];
        if offset[0] < 0 || offset[1] < 0 || offset[0] + i64::from(size[0]) > i64::from(CELL)
            || offset[1] + i64::from(size[1]) > i64::from(CELL) {
            return Err(format!("row{} frame{} would clip after foot alignment", sprite.row, sprite.column));
        }
        let destination = Rect { x: sprite.column * CELL + offset[0] as u32, y: sprite.row * CELL + offset[1] as u32,
            width: size[0], height: size[1] };
        imageops::overlay(&mut output, &resized, i64::from(destination.x), i64::from(destination.y));
        frames.push(FrameReport { action: ACTIONS[sprite.row as usize], frame: sprite.column, source_region: sprite.region,
            source_crop: crop, source_foot: sprite.foot, resized_size: size, destination, core_pixels: sprite.core_pixels });
    }
    if scale < requested_scale * 0.95 { warnings.push("wide/tall effects use reduced pixel resolution; effective pixelsPerWorldUnit preserves reference body size".into()); }
    if reassigned_soft_pixels > 0 {
            warnings.push(format!("{reassigned_soft_pixels} very faint edge pixels were associated with the closest component; inspect effects"));
    }
    let report = Report {
        status: if warnings.is_empty() { "normalized" } else { "review" },
        input: options.input.display().to_string(), reference: options.reference.display().to_string(), output: options.output.display().to_string(),
        source_size: [width, height], output_size: [CELL * COLUMNS, CELL * ROWS], frame_size_px: [CELL, CELL], pivot_px: pivot,
        pixels_per_world_unit: effective_ppu, requested_pixels_per_world_unit: options.pixels_per_world_unit, reference_visible_height: reference_bounds.height,
        target_standing_height: target_height, source_idle_height: idle_height, requested_scale, applied_scale: scale, reassigned_soft_pixels,
        component_alpha: options.component_alpha, frames, warnings,
    };
    Ok((output, report))
}

struct Component {
    pixels: Vec<usize>,
    bounds: Rect,
    center: [f64; 2],
}

fn extract_components(source: &RgbaImage, alpha: u8) -> Result<(Vec<Sprite>, Vec<u8>, Vec<String>), String> {
    let (width, height) = source.dimensions();
    let mut visited = vec![false; (width * height) as usize];
    let mut components = Vec::<Component>::new();
    for seed in 0..visited.len() {
        if visited[seed] || source.get_pixel(seed as u32 % width, seed as u32 / width)[3] <= alpha { continue; }
        visited[seed] = true;
        let mut pixels = vec![seed];
        let (mut head, mut left, mut top, mut right, mut bottom, mut sum_x, mut sum_y) = (0, width, height, 0, 0, 0u64, 0u64);
        while head < pixels.len() {
            let pixel = pixels[head]; head += 1;
            let (x, y) = (pixel as u32 % width, pixel as u32 / width);
            left = left.min(x); top = top.min(y); right = right.max(x); bottom = bottom.max(y);
            sum_x += u64::from(x); sum_y += u64::from(y);
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let next = (ny * width + nx) as usize;
                    if !visited[next] && source.get_pixel(nx, ny)[3] > alpha { visited[next] = true; pixels.push(next); }
                }
            }
        }
        let center = [sum_x as f64 / pixels.len() as f64, sum_y as f64 / pixels.len() as f64];
        components.push(Component { pixels, center, bounds: Rect { x: left, y: top, width: right - left + 1, height: bottom - top + 1 } });
    }
    let mut order: Vec<usize> = (0..components.len()).collect();
    order.sort_by_key(|index| std::cmp::Reverse(components[*index].pixels.len()));
    let minimum = (width as usize * height as usize / (24 * 128)).max(32);
    if order.len() < 24 || components[order[23]].pixels.len() < minimum {
        return Err(format!("expected24 separate solid figures; found only{} substantial components at alpha{alpha}; inspect overlaps or missing poses",
            order.iter().filter(|index| components[**index].pixels.len() >= minimum).count()));
    }
    if order.len() > 24 && components[order[24]].pixels.len() * 4 > components[order[23]].pixels.len() {
        return Err("more than24 substantial components; cannot safely distinguish extra figures from detached effects".into());
    }
    let mut main = order[..24].to_vec();
    main.sort_by(|a, b| components[*a].center[1].total_cmp(&components[*b].center[1]));
    for row in main.chunks_mut(4) { row.sort_by(|a, b| components[*a].center[0].total_cmp(&components[*b].center[0])); }
    let mut owners = vec![u8::MAX; (width * height) as usize];
    let mut sprites = Vec::new();
    for (index, component_id) in main.iter().enumerate() {
        let component = &components[*component_id];
        let (row, column) = (index as u32 / 4, index as u32 % 4);
        let expected = [(f64::from(column) + 0.5) * f64::from(width) / 4.0, (f64::from(row) + 0.5) * f64::from(height) / 6.0];
        if (component.center[0] - expected[0]).abs() > f64::from(width) / 4.0 * 0.7
            || (component.center[1] - expected[1]).abs() > f64::from(height) / 6.0 * 0.7 {
            return Err(format!("component for row{row} frame{column} is outside its expected grid area; review frame ordering"));
        }
        let (mut left, mut top, mut right, mut bottom, mut core_pixels) = (width, height, 0, 0, 0);
        for &pixel in &component.pixels {
            owners[pixel] = index as u8;
            let (x, y) = (pixel as u32 % width, pixel as u32 / width);
            if source.get_pixel(x, y)[3] > FOOT_ALPHA {
                left = left.min(x); top = top.min(y); right = right.max(x); bottom = bottom.max(y); core_pixels += 1;
            }
        }
        if core_pixels < 32 { return Err(format!("row{row} frame{column} has no solid body")); }
        if left == 0 || top == 0 || right + 1 == width || bottom + 1 == height {
            return Err(format!("row{row} frame{column} solid art touches the source edge; source is clipped"));
        }
        let feet_top = top + (bottom - top + 1) * 3 / 4;
        let (mut feet_left, mut feet_right) = (width, 0);
        for &pixel in &component.pixels {
            let (x, y) = (pixel as u32 % width, pixel as u32 / width);
            if y >= feet_top && source.get_pixel(x, y)[3] > FOOT_ALPHA { feet_left = feet_left.min(x); feet_right = feet_right.max(x); }
        }
        let foot = [f64::from(feet_left + feet_right) / 2.0, f64::from(bottom)];
        sprites.push(Sprite { row, column, region: component.bounds, crop: component.bounds, foot, core_pixels, visible_height: component.bounds.height });
    }
    let mut meaningful_fragments = 0;
    for component_id in order.into_iter().skip(24) {
        let component = &components[component_id];
        let closest = main.iter().enumerate().min_by(|(_, a), (_, b)| {
            distance_to_component(component.center, &components[**a]).total_cmp(&distance_to_component(component.center, &components[**b]))
        }).unwrap().0;
        for &pixel in &component.pixels { owners[pixel] = closest as u8; }
        if component.pixels.len() >= 8 { meaningful_fragments += 1; }
    }
    let mut warnings = Vec::new();
    if meaningful_fragments > 0 { warnings.push(format!("{meaningful_fragments} small detached weapon/effect components associated with nearest figures; inspect assignments")); }
    Ok((sprites, owners, warnings))
}

fn distance_to_component(point: [f64; 2], component: &Component) -> f64 {
    let rect = component.bounds;
    let dx = (f64::from(rect.x) - point[0]).max(0.0).max(point[0] - f64::from(rect.x + rect.width - 1));
    let dy = (f64::from(rect.y) - point[1]).max(0.0).max(point[1] - f64::from(rect.y + rect.height - 1));
    dx * dx + dy * dy + ((point[0] - component.center[0]).powi(2) + (point[1] - component.center[1]).powi(2)) * 0.001
}

fn assign_soft_edges(source: &RgbaImage, mut owners: Vec<u8>) -> (Vec<u8>, u32) {
    let (width, height) = source.dimensions();
    let mut queue = Vec::with_capacity(owners.len());
    let mut reassigned = 0;
    for (pixel, owner) in owners.iter().enumerate() { if *owner != u8::MAX { queue.push(pixel); } }
    let mut head = 0;
    while head < queue.len() {
        let pixel = queue[head];
        head += 1;
        let x = pixel % width as usize;
        let y = pixel / width as usize;
        for neighbor in [
            (x > 0).then(|| pixel - 1), (x + 1 < width as usize).then(|| pixel + 1),
            (y > 0).then(|| pixel - width as usize), (y + 1 < height as usize).then(|| pixel + width as usize),
        ].into_iter().flatten() {
            if owners[neighbor] == u8::MAX {
                if source.get_pixel((neighbor % width as usize) as u32, (neighbor / width as usize) as u32)[3] > 0 { reassigned += 1; }
                owners[neighbor] = owners[pixel]; queue.push(neighbor);
            }
        }
    }
    (owners, reassigned)
}

fn bounds(image: &RgbaImage, region: Rect, alpha: u8) -> Option<Rect> {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    for y in region.y..region.y + region.height {
        for x in region.x..region.x + region.width {
            if image.get_pixel(x, y)[3] > alpha {
                left = left.min(x); top = top.min(y); right = right.max(x); bottom = bottom.max(y);
            }
        }
    }
    (left != u32::MAX).then(|| Rect { x: left, y: top, width: right - left + 1, height: bottom - top + 1 })
}

/// Premultiply before filtering so invisible RGB/background colors cannot bleed into the cutout edges.
fn resize_alpha(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let premultiplied: Rgba32FImage = ImageBuffer::from_fn(source.width(), source.height(), |x, y| {
        let pixel = source.get_pixel(x, y);
        let alpha = f32::from(pixel[3]) / 255.0;
        Rgba([f32::from(pixel[0]) / 255.0 * alpha, f32::from(pixel[1]) / 255.0 * alpha, f32::from(pixel[2]) / 255.0 * alpha, alpha])
    });
    let resized = imageops::resize(&premultiplied, width, height, imageops::FilterType::Lanczos3);
    ImageBuffer::from_fn(width, height, |x, y| {
        let pixel = resized.get_pixel(x, y);
        let alpha = pixel[3].clamp(0.0, 1.0);
        if alpha <= 0.00001 { return Rgba([0, 0, 0, 0]); }
        Rgba([(pixel[0] / alpha * 255.0).round().clamp(0.0, 255.0) as u8,
            (pixel[1] / alpha * 255.0).round().clamp(0.0, 255.0) as u8,
            (pixel[2] / alpha * 255.0).round().clamp(0.0, 255.0) as u8,
            (alpha * 255.0).round() as u8])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options { input: "raw.png".into(), reference: "reference.png".into(), output: "normalized.png".into(), report: "report.json".into(),
            pixels_per_world_unit: 105.0, reference_pixels_per_world_unit: 128.0, component_alpha: COMPONENT_ALPHA, force: false }
    }

    fn rectangle(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, alpha: u8) {
        for py in y..y + height { for px in x..x + width { image.put_pixel(px, py, Rgba([60, 120, 180, alpha])); } }
    }

    fn fixture() -> (RgbaImage, RgbaImage) {
        let mut image = RgbaImage::new(256, 384);
        for row in 0..6 { for column in 0..4 {
            rectangle(&mut image, column * 64 + 26, row * 64 + 18, 18, 40, 255);
        } }
        // A sword crosses the nominal64px column; the actual gap is to its right.
        rectangle(&mut image, 42, 90, 36, 3, 255);
        let mut reference = RgbaImage::new(64, 96);
        rectangle(&mut reference, 22, 12, 20, 70, 255);
        (image, reference)
    }

    #[test]
    fn finds_row_local_column_gutters_and_aligns_all_twenty_four_feet() {
        let (source, reference) = fixture();
        let (output, report) = normalize(&source, &reference, &options()).unwrap();
        assert_eq!(report.frames.len(), 24);
        assert_eq!(output.dimensions(), (1024, 1536));
        let sword = &report.frames[4];
        assert!(sword.source_crop.x + sword.source_crop.width > 64, "the crossing sword was preserved");
        for frame in &report.frames {
            assert!(frame.resized_size[0] <= 224 && frame.resized_size[1] <= 208);
            let row = ACTIONS.iter().position(|action| *action == frame.action).unwrap() as u32;
            assert!(output.get_pixel(frame.frame * CELL + report.pivot_px[0], row * CELL + report.pivot_px[1])[3] > 100);
        }
        assert!((report.target_standing_height - 70.0 * 105.0 / 128.0).abs() < 0.0001);
        assert!((report.source_idle_height * report.applied_scale / report.pixels_per_world_unit - 70.0 / 128.0).abs() < 0.0001);
    }

    #[test]
    fn overlapping_solid_art_is_rejected_instead_of_sliced() {
        let (mut source, reference) = fixture();
        rectangle(&mut source, 0, 32, 256, 66, 255);
        let error = normalize(&source, &reference, &options()).unwrap_err();
        assert!(error.contains("24 separate solid figures"), "{error}");
    }

    #[test]
    fn soft_glow_at_a_gutter_is_reported_for_review() {
        let (mut source, reference) = fixture();
        rectangle(&mut source, 0, 32, 256, 66, 8);
        // Restore the solid figures under the soft overlay.
        for column in 0..4 { rectangle(&mut source, column * 64 + 26, 18, 18, 40, 255); }
        let (_, report) = normalize(&source, &reference, &options()).unwrap();
        assert_eq!(report.status, "review");
        assert!(report.warnings.iter().any(|warning| warning.contains("very faint edge pixels")));
    }

    #[test]
    fn resizing_does_not_leak_invisible_background_rgb() {
        let mut source = RgbaImage::new(2, 1);
        source.put_pixel(0, 0, Rgba([255, 0, 0, 0]));
        source.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
        let resized = resize_alpha(&source, 8, 2);
        for pixel in resized.pixels().filter(|pixel| pixel[3] > 0) {
            assert_eq!(pixel[0], 0);
            assert_eq!(pixel[2], 255);
        }
    }
}
