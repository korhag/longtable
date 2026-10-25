// ------------------------------------------------------------
// Module: geometry
// Purpose: One definition of each die. Vertices and numbered
//          faces are built here, sent to clients for the mesh,
//          and used by the server to read the resting face.
//          A d4 is read from the face that lands down. Every
//          other die is read from the face that lands up.
//          Opposite faces sum to sides + 1, as on a table die.
// ------------------------------------------------------------

use std::sync::OnceLock;

use rapier3d::prelude::nalgebra::{UnitQuaternion, Vector3};

/// The six dice this table knows how to throw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DieKind {
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
}

impl DieKind {
    pub fn sides(self) -> u8 {
        match self {
            Self::D4 => 4,
            Self::D6 => 6,
            Self::D8 => 8,
            Self::D10 => 10,
            Self::D12 => 12,
            Self::D20 => 20,
        }
    }

    pub fn from_sides(sides: u32) -> Option<Self> {
        Some(match sides {
            4 => Self::D4,
            6 => Self::D6,
            8 => Self::D8,
            10 => Self::D10,
            12 => Self::D12,
            20 => Self::D20,
            _ => return None,
        })
    }

    pub fn reads_bottom(self) -> bool {
        matches!(self, Self::D4)
    }
}

/// One numbered face. `normal` points outward in local space.
#[derive(Clone, Debug)]
pub struct Face {
    pub indices: Vec<u16>,
    pub value: u8,
    pub normal: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct DieTemplate {
    pub kind: DieKind,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Face>,
}

impl DieTemplate {
    pub fn get(kind: DieKind) -> &'static DieTemplate {
        templates()[kind.sides() as usize]
            .as_ref()
            .expect("every standard die is built at startup")
    }
}

/// Indexed by side count. Only 4, 6, 8, 10, 12, and 20 are filled.
pub fn templates() -> &'static Vec<Option<DieTemplate>> {
    static TEMPLATES: OnceLock<Vec<Option<DieTemplate>>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let mut slots = vec![None; 21];
        for kind in [DieKind::D4, DieKind::D6, DieKind::D8, DieKind::D10, DieKind::D12, DieKind::D20] {
            let die = build(kind);
            slots[kind.sides() as usize] = Some(die);
        }
        slots
    })
}

/// Read the face a rested die is showing.
/// `rotation` is a unit quaternion in x, y, z, w order.
/// The second value is how squarely that face points at the reader (1 is flat).
pub fn read_face(die: &DieTemplate, rotation: [f32; 4]) -> (u8, f32) {
    let target = if die.kind.reads_bottom() {
        [0.0, -1.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let mut best_value = 1u8;
    let mut best_dot = f32::MIN;
    for face in &die.faces {
        let normal = rotate_xyzw(rotation, face.normal);
        let alignment = normal[0] * target[0] + normal[1] * target[1] + normal[2] * target[2];
        if alignment > best_dot {
            best_dot = alignment;
            best_value = face.value;
        }
    }
    (best_value, best_dot)
}

fn rotate_xyzw(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let tx = 2.0 * (y * v[2] - z * v[1]);
    let ty = 2.0 * (z * v[0] - x * v[2]);
    let tz = 2.0 * (x * v[1] - y * v[0]);
    [
        v[0] + w * tx + (y * tz - z * ty),
        v[1] + w * ty + (z * tx - x * tz),
        v[2] + w * tz + (x * ty - y * tx),
    ]
}

fn build(kind: DieKind) -> DieTemplate {
    let raw = match kind {
        DieKind::D4 => vec![[1.0, 1.0, 1.0], [1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [-1.0, -1.0, 1.0]],
        DieKind::D6 => cube_vertices(),
        DieKind::D8 => vec![
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ],
        DieKind::D10 => d10_vertices(),
        DieKind::D12 => d12_vertices(),
        DieKind::D20 => d20_vertices(),
    };
    let scaled = scale_to_radius(&raw, 0.46);
    let mut faces = extract_faces(&scaled);
    number_faces(&mut faces, kind.sides());
    let vertices = scaled
        .into_iter()
        .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
        .collect();
    DieTemplate { kind, vertices, faces }
}

fn cube_vertices() -> Vec<[f64; 3]> {
    let mut vertices = Vec::new();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                vertices.push([x, y, z]);
            }
        }
    }
    vertices
}

/// A gaming d10: a pentagonal trapezohedron with every vertex on a sphere,
/// so the ten kite faces are fair. The latitude is the value that makes
/// each kite planar.
fn d10_vertices() -> Vec<[f64; 3]> {
    let latitude = d10_latitude();
    let radius = (1.0 - latitude * latitude).sqrt();
    let mut vertices = vec![[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
    for index in 0..5 {
        let angle = index as f64 * std::f64::consts::TAU / 5.0;
        vertices.push([radius * angle.cos(), latitude, radius * angle.sin()]);
    }
    for index in 0..5 {
        let angle = (index as f64 + 0.5) * std::f64::consts::TAU / 5.0;
        vertices.push([radius * angle.cos(), -latitude, radius * angle.sin()]);
    }
    vertices
}

fn d10_latitude() -> f64 {
    let mut lo = 0.05;
    let mut hi = 0.2;
    for _ in 0..60 {
        let mid = (lo + hi) * 0.5;
        if d10_triple(lo) * d10_triple(mid) <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (lo + hi) * 0.5
}

fn d10_triple(latitude: f64) -> f64 {
    let radius = (1.0 - latitude * latitude).sqrt();
    let c72 = (std::f64::consts::TAU / 5.0).cos();
    let s72 = (std::f64::consts::TAU / 5.0).sin();
    let c36 = (std::f64::consts::TAU / 10.0).cos();
    let s36 = (std::f64::consts::TAU / 10.0).sin();
    // Kite: north pole, upper 0, lower between them, upper 1.
    let u0 = [radius, latitude - 1.0, 0.0];
    let u1 = [radius * c72, latitude - 1.0, radius * s72];
    let lower = [radius * c36, -latitude - 1.0, radius * s36];
    let cross = [
        u1[1] * lower[2] - u1[2] * lower[1],
        u1[2] * lower[0] - u1[0] * lower[2],
        u1[0] * lower[1] - u1[1] * lower[0],
    ];
    u0[0] * cross[0] + u0[1] * cross[1] + u0[2] * cross[2]
}

fn d12_vertices() -> Vec<[f64; 3]> {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let inverse = 1.0 / phi;
    let mut vertices = cyclic_signed(0.0, inverse, phi);
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                vertices.push([x, y, z]);
            }
        }
    }
    dedupe(vertices)
}

fn d20_vertices() -> Vec<[f64; 3]> {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    dedupe(cyclic_signed(0.0, -1.0, phi))
}

fn cyclic_signed(x: f64, y: f64, z: f64) -> Vec<[f64; 3]> {
    let cycles = [[x, y, z], [y, z, x], [z, x, y]];
    let mut vertices = Vec::new();
    for cycle in cycles {
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    vertices.push([cycle[0] * sx, cycle[1] * sy, cycle[2] * sz]);
                }
            }
        }
    }
    vertices
}

fn dedupe(vertices: Vec<[f64; 3]>) -> Vec<[f64; 3]> {
    let mut unique = Vec::new();
    for vertex in vertices {
        let known = unique.iter().any(|other: &[f64; 3]| {
            (vertex[0] - other[0]).abs() < 1e-9
                && (vertex[1] - other[1]).abs() < 1e-9
                && (vertex[2] - other[2]).abs() < 1e-9
        });
        if !known {
            unique.push(vertex);
        }
    }
    unique
}

fn scale_to_radius(vertices: &[[f64; 3]], radius: f64) -> Vec<[f64; 3]> {
    let max = vertices
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .fold(0.0, f64::max)
        .max(1e-6);
    let scale = radius / max;
    vertices
        .iter()
        .map(|v| [v[0] * scale, v[1] * scale, v[2] * scale])
        .collect()
}

struct RawFace {
    indices: Vec<u16>,
    normal: [f64; 3],
}

fn extract_faces(vertices: &[[f64; 3]]) -> Vec<Face> {
    let count = vertices.len();
    let mut seen = Vec::<Vec<usize>>::new();
    let mut raw = Vec::new();
    for i in 0..count {
        for j in (i + 1)..count {
            for k in (j + 1)..count {
                let Some(mut normal) = plane_normal(vertices[i], vertices[j], vertices[k]) else {
                    continue;
                };
                let distance = dot(normal, vertices[i]);
                let mut on_plane = Vec::new();
                let mut positive = false;
                let mut negative = false;
                for (index, vertex) in vertices.iter().enumerate() {
                    let side = dot(normal, *vertex) - distance;
                    if side > 1e-6 {
                        positive = true;
                    } else if side < -1e-6 {
                        negative = true;
                    } else {
                        on_plane.push(index);
                    }
                    if positive && negative {
                        break;
                    }
                }
                if positive && negative || on_plane.len() < 3 {
                    continue;
                }
                // Outward normals point away from the origin. The dice are centered.
                if distance < 0.0 {
                    normal = [-normal[0], -normal[1], -normal[2]];
                }
                let mut key = on_plane.clone();
                key.sort_unstable();
                if seen.iter().any(|existing| existing == &key) {
                    continue;
                }
                seen.push(key);
                let ordered = order_around(vertices, &on_plane, normal);
                raw.push(RawFace { indices: ordered, normal });
            }
        }
    }
    raw.sort_by(|a, b| {
        a.normal[1]
            .total_cmp(&b.normal[1])
            .then(a.normal[0].total_cmp(&b.normal[0]))
            .then(a.normal[2].total_cmp(&b.normal[2]))
    });
    raw.into_iter()
        .map(|face| Face {
            indices: face.indices,
            value: 0,
            normal: [face.normal[0] as f32, face.normal[1] as f32, face.normal[2] as f32],
        })
        .collect()
}

fn plane_normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[f64; 3]> {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if length < 1e-10 {
        None
    } else {
        Some([cross[0] / length, cross[1] / length, cross[2] / length])
    }
}

fn order_around(vertices: &[[f64; 3]], indices: &[usize], normal: [f64; 3]) -> Vec<u16> {
    let center = average(vertices, indices);
    let mut tangent = cross(normal, [1.0, 0.0, 0.0]);
    if length(tangent) < 1e-6 {
        tangent = cross(normal, [0.0, 1.0, 0.0]);
    }
    tangent = normalize(tangent);
    let bitangent = normalize(cross(normal, tangent));
    let mut angled: Vec<(usize, f64)> = indices
        .iter()
        .map(|&index| {
            let delta = [
                vertices[index][0] - center[0],
                vertices[index][1] - center[1],
                vertices[index][2] - center[2],
            ];
            let angle = dot(delta, bitangent).atan2(dot(delta, tangent));
            (index, angle)
        })
        .collect();
    angled.sort_by(|a, b| a.1.total_cmp(&b.1));
    angled.into_iter().map(|(index, _)| index as u16).collect()
}

fn number_faces(faces: &mut [Face], sides: u8) {
    let count = faces.len();
    let mut partner = vec![None; count];
    for i in 0..count {
        if partner[i].is_some() {
            continue;
        }
        let mut best: Option<usize> = None;
        let mut best_dot = f32::MAX;
        for j in (i + 1)..count {
            if partner[j].is_some() {
                continue;
            }
            let alignment = dot_f32(faces[i].normal, faces[j].normal);
            if alignment < best_dot {
                best_dot = alignment;
                best = Some(j);
            }
        }
        if let Some(j) = best {
            if best_dot < -0.85 {
                partner[i] = Some(j);
                partner[j] = Some(i);
            }
        }
    }

    let mut next = 1u8;
    let mut used = vec![false; count];
    for i in 0..count {
        if used[i] {
            continue;
        }
        if let Some(j) = partner[i] {
            faces[i].value = next;
            faces[j].value = sides + 1 - next;
            used[i] = true;
            used[j] = true;
            next += 1;
        }
    }
    for face in faces.iter_mut() {
        if face.value == 0 {
            face.value = next;
            next += 1;
        }
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn dot_f32(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let len = length(v).max(1e-12);
    [v[0] / len, v[1] / len, v[2] / len]
}

fn average(vertices: &[[f64; 3]], indices: &[usize]) -> [f64; 3] {
    let mut sum = [0.0, 0.0, 0.0];
    for &index in indices {
        sum[0] += vertices[index][0];
        sum[1] += vertices[index][1];
        sum[2] += vertices[index][2];
    }
    let scale = indices.len() as f64;
    [sum[0] / scale, sum[1] / scale, sum[2] / scale]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn die(kind: DieKind) -> &'static DieTemplate {
        DieTemplate::get(kind)
    }

    /// `rotation_between` refuses a 180° turn, which is exactly how an opposite face lies.
    fn rotation_aligning(from: Vector3<f32>, to: Vector3<f32>) -> UnitQuaternion<f32> {
        if let Some(rotation) = UnitQuaternion::rotation_between(&from, &to) {
            return rotation;
        }
        let mut axis = from.cross(&Vector3::x());
        if axis.norm() < 1e-4 {
            axis = from.cross(&Vector3::y());
        }
        UnitQuaternion::from_scaled_axis(axis.normalize() * std::f32::consts::PI)
    }

    #[test]
    fn every_die_has_the_right_faces_and_numbers() {
        for kind in [DieKind::D4, DieKind::D6, DieKind::D8, DieKind::D10, DieKind::D12, DieKind::D20] {
            let template = die(kind);
            assert_eq!(template.faces.len(), kind.sides() as usize, "{kind:?}");
            let mut values: Vec<u8> = template.faces.iter().map(|face| face.value).collect();
            values.sort_unstable();
            let expected: Vec<u8> = (1..=kind.sides()).collect();
            assert_eq!(values, expected, "{kind:?}");
        }
    }

    #[test]
    fn turning_a_face_up_reads_that_face() {
        for kind in [DieKind::D4, DieKind::D6, DieKind::D8, DieKind::D10, DieKind::D12, DieKind::D20] {
            let template = die(kind);
            let target = if kind.reads_bottom() {
                Vector3::new(0.0, -1.0, 0.0)
            } else {
                Vector3::y()
            };
            for face in &template.faces {
                let normal = Vector3::new(face.normal[0], face.normal[1], face.normal[2]);
                let rotation = rotation_aligning(normal, target);
                let coords = rotation.as_vector();
                let (value, alignment) = read_face(template, [coords.x, coords.y, coords.z, coords.w]);
                assert_eq!(value, face.value, "{kind:?}");
                assert!(alignment > 0.99, "{kind:?} alignment {alignment}");
            }
        }
    }

    #[test]
    fn opposite_faces_sum_to_one_more_than_the_sides() {
        for kind in [DieKind::D6, DieKind::D8, DieKind::D10, DieKind::D12, DieKind::D20] {
            let template = die(kind);
            for face in &template.faces {
                let opposite = template
                    .faces
                    .iter()
                    .filter(|other| dot_f32(face.normal, other.normal) < -0.85)
                    .collect::<Vec<_>>();
                assert_eq!(opposite.len(), 1, "{kind:?}");
                assert_eq!(face.value + opposite[0].value, kind.sides() + 1, "{kind:?}");
            }
        }
    }
}
