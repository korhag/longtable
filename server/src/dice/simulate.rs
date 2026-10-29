// ------------------------------------------------------------
// Module: simulate
// Purpose: Throw the parsed dice in a Rapier tray and record
//          the tumble. The result is the face that is up (or
//          down, for a d4) once the die sleeps. Clients never
//          decide a number; they only play these frames.
// ------------------------------------------------------------

use rapier3d::prelude::*;

use super::formula::RollFormula;
use super::geometry::{read_face, DieTemplate};

/// One simulated die, ready to be copied into a network message.
#[derive(Clone, Debug)]
pub struct ThrownDie {
    pub sides: u8,
    pub value: u8,
    pub sign: i8,
    pub tag: Option<String>,
    pub keyframes: Vec<SimFrame>,
}

#[derive(Clone, Debug)]
pub struct SimFrame {
    pub p: [f32; 3],
    pub q: [f32; 4],
}

pub const KEYFRAME_DT: f32 = 1.0 / 30.0;
const STEP_DT: f32 = 1.0 / 60.0;
const MAX_STEPS: usize = 60 * 3;
const FLAT_ALIGNMENT: f32 = 0.82;

pub fn simulate(formula: &RollFormula, rng: &mut impl rand::RngExt) -> Vec<ThrownDie> {
    if formula.dice().is_empty() {
        return Vec::new();
    }
    let mut last = Vec::new();
    for _attempt in 0..3 {
        last = throw_once(formula, rng);
        if last.iter().all(rests_inside) {
            return last;
        }
    }
    last
}

fn rests_inside(die: &ThrownDie) -> bool {
    let Some(frame) = die.keyframes.last() else {
        return false;
    };
    frame.p[0].abs() < 8.0 && frame.p[2].abs() < 5.0 && frame.p[1] < 2.0
}

fn throw_once(formula: &RollFormula, rng: &mut impl rand::RngExt) -> Vec<ThrownDie> {
    let spec = formula.dice();
    let mut pipeline = PhysicsPipeline::new();
    let gravity = vec3(0.0, -18.0, 0.0);
    let mut integration = IntegrationParameters::default();
    integration.dt = STEP_DT;
    let mut islands = IslandManager::new();
    let mut broad_phase = BroadPhaseBvh::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut impulse_joints = ImpulseJointSet::new();
    let mut multibody_joints = MultibodyJointSet::new();
    let mut soft_bodies = SoftBodySet::new();
    let mut ccd = CCDSolver::new();

    spawn_tray(&mut bodies, &mut colliders);

    let mut handles = Vec::new();
    let columns = (spec.len() as f32).sqrt().ceil().max(1.0) as usize;
    for (index, (kind, _sign, _role)) in spec.iter().enumerate() {
        let template = DieTemplate::get(*kind);
        let column = index % columns;
        let x = ((column as f32 - (columns.saturating_sub(1) as f32) * 0.5) * 1.15).clamp(-6.0, 6.0);
        let body = RigidBodyBuilder::dynamic()
            .translation(vec3(
                x + rng.random_range(-0.15..0.15),
                3.2 + index as f32 * 0.35,
                7.5 + rng.random_range(-0.2..0.2),
            ))
            .rotation(vec3(
                rng.random_range(-3.0..3.0),
                rng.random_range(-3.0..3.0),
                rng.random_range(-3.0..3.0),
            ))
            .linvel(vec3(
                rng.random_range(-1.2..1.2),
                rng.random_range(1.2..2.4),
                rng.random_range(-9.0..-7.0),
            ))
            .angvel(vec3(
                rng.random_range(-9.0..9.0),
                rng.random_range(-9.0..9.0),
                rng.random_range(-9.0..9.0),
            ))
            .linear_damping(0.32)
            .angular_damping(0.4)
            .ccd_enabled(false)
            .can_sleep(true)
            .build();
        let handle = bodies.insert(body);
        let points: Vec<Vector> = template
            .vertices
            .iter()
            .map(|v| vec3(v[0], v[1], v[2]))
            .collect();
        let collider = ColliderBuilder::convex_hull(&points)
            .unwrap_or_else(|| ColliderBuilder::ball(0.42))
            .friction(0.72)
            .restitution(0.28)
            .build();
        colliders.insert_with_parent(collider, handle, &mut bodies);
        handles.push(handle);
    }

    let mut frames: Vec<Vec<SimFrame>> = vec![Vec::new(); handles.len()];
    let mut nudges = 0;
    record(&bodies, &handles, &mut frames);
    for step in 0..MAX_STEPS {
        pipeline.step(
            gravity,
            &integration,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut bodies,
            &mut colliders,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut soft_bodies,
            &mut ccd,
            &(),
            &(),
        );
        if step % 2 == 0 {
            record(&bodies, &handles, &mut frames);
        }

        let settled = step > 20
            && handles.iter().all(|handle| bodies.get(*handle).is_some_and(|body| body.is_sleeping()));
        if settled {
            let flat = handles.iter().enumerate().all(|(index, handle)| {
                let (kind, _, _) = spec[index];
                alignment_of(DieTemplate::get(kind), bodies.get(*handle).unwrap()) >= FLAT_ALIGNMENT
            });
            if flat || nudges >= 4 {
                record(&bodies, &handles, &mut frames);
                break;
            }
            nudges += 1;
            for handle in &handles {
                if let Some(body) = bodies.get_mut(*handle) {
                    body.apply_impulse(
                        vec3(rng.random_range(-0.35..0.35), 0.55, rng.random_range(-0.35..0.35)),
                        true,
                    );
                }
            }
        }
    }

    spec.into_iter()
        .enumerate()
        .map(|(index, (kind, sign, tag))| {
            let template = DieTemplate::get(kind);
            let rotation = bodies.get(handles[index]).map(|body| *body.rotation());
            let (value, _) = match rotation {
                Some(rotation) => read_face(template, [rotation.x, rotation.y, rotation.z, rotation.w]),
                None => (1, 0.0),
            };
            let mut keyframes = frames[index].clone();
            if keyframes.is_empty() {
                keyframes.push(SimFrame { p: [0.0, 0.5, 0.0], q: [0.0, 0.0, 0.0, 1.0] });
            }
            ThrownDie {
                sides: kind.sides(),
                value,
                sign,
                tag,
                keyframes,
            }
        })
        .collect()
}

fn spawn_tray(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) {
    let floor = bodies.insert(
        RigidBodyBuilder::fixed().translation(vec3(0.0, -0.2, 0.0)).build(),
    );
    colliders.insert_with_parent(
        ColliderBuilder::cuboid(16.0, 0.2, 12.0)
            .friction(0.95)
            .restitution(0.18)
            .build(),
        floor,
        bodies,
    );
    let walls = [
        (8.5, 1.0, 0.0, 0.2, 1.2, 6.0),
        (-8.5, 1.0, 0.0, 0.2, 1.2, 6.0),
        (0.0, 1.0, 5.5, 8.5, 1.2, 0.2),
        (0.0, 1.0, -5.5, 8.5, 1.2, 0.2),
    ];
    for (x, y, z, hx, hy, hz) in walls {
        let body = bodies.insert(RigidBodyBuilder::fixed().translation(vec3(x, y, z)).build());
        colliders.insert_with_parent(
            ColliderBuilder::cuboid(hx, hy, hz).friction(0.6).restitution(0.15).build(),
            body,
            bodies,
        );
    }
}

fn record(bodies: &RigidBodySet, handles: &[RigidBodyHandle], frames: &mut [Vec<SimFrame>]) {
    for (index, handle) in handles.iter().enumerate() {
        let Some(body) = bodies.get(*handle) else {
            continue;
        };
        let translation = body.position().translation;
        let rotation = body.rotation();
        frames[index].push(SimFrame {
            p: [round3(translation.x), round3(translation.y), round3(translation.z)],
            q: [round3(rotation.x), round3(rotation.y), round3(rotation.z), round3(rotation.w)],
        });
    }
}

fn alignment_of(die: &DieTemplate, body: &RigidBody) -> f32 {
    let rotation = body.rotation();
    read_face(die, [rotation.x, rotation.y, rotation.z, rotation.w]).1
}

fn vec3(x: f32, y: f32, z: f32) -> Vector {
    Vector::new(x, y, z)
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::parse_formula;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn a_thrown_die_lands_on_a_legal_face() {
        let formula = parse_formula("1d6+2d4").unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        let dice = simulate(&formula, &mut rng);
        assert_eq!(dice.len(), 3);
        assert!((1..=6).contains(&dice[0].value));
        assert_eq!(dice[0].sides, 6);
        assert!((1..=4).contains(&dice[1].value));
        assert!(dice[0].keyframes.len() > 5);
        assert!(dice.iter().all(|die| die.keyframes.len() == dice[0].keyframes.len()));
    }

    #[test]
    fn dice_enter_from_outside_and_rest_inside() {
        let formula = parse_formula("2d12").unwrap();
        for seed in [3_u64, 11, 19] {
            let mut rng = StdRng::seed_from_u64(seed);
            let dice = simulate(&formula, &mut rng);
            assert_eq!(dice.len(), 2);
            for die in &dice {
                let start = die.keyframes.first().expect("a throw has a first frame");
                let end = die.keyframes.last().expect("a throw has a last frame");
                assert!(start.p[2] > 5.5, "seed {seed} started at {:?}", start.p);
                assert!(
                    end.p[0].abs() < 8.3 && end.p[2].abs() < 5.3,
                    "seed {seed} ended at {:?}",
                    end.p
                );
            }
        }
    }
}
