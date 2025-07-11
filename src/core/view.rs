use bevy::{input::mouse::MouseMotion, math::vec2, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        binds::{BindingAxis, BindingAxisFns, InputBindingSystem},
        AppState,
    },
    editor::freelook::FreelookState,
    util::FromPitchYawRoll,
};

/// For gimbal-locked rotation.
/// Pitch=X, Yaw=Y, Roll=Z
#[derive(Component, Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[require(Transform)]
pub struct Gimbal {
    pub pitch_yaw: Vec2,
    pub roll: f32,
}

impl Gimbal {
    pub fn new(pitch_yaw: Vec2, roll: f32) -> Self {
        Self { pitch_yaw, roll }
    }
}

impl FromPitchYawRoll for Gimbal {
    fn from_pitch_yaw_roll(pitch: f32, yaw: f32, roll: f32) -> Self {
        Self {
            pitch_yaw: vec2(pitch, yaw),
            roll,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct GimbalPos {
    pub pos: Vec3,
    pub rot: Gimbal,
}

impl GimbalPos {
    pub fn new(pos: Vec3, rot: Gimbal) -> Self {
        Self { pos, rot }
    }

    pub fn pitch_yaw_roll(pos: Vec3, pitch_yaw: Vec2, roll: f32) -> Self {
        Self {
            pos,
            rot: Gimbal { pitch_yaw, roll },
        }
    }
}

#[derive(Component)]
pub struct GimbalRotatesParent;

#[derive(Event, Deref)]
pub struct TPCameraTo(pub GimbalPos);

const PITCH_LIMIT: f32 = 88.0_f32.to_radians();

// impl Gimbal {
//     pub fn new(yaw: f32, pitch: f32) -> Self {
//         Self {
//             pitch_yaw: Vec2 { x: yaw, y: pitch },
//             roll: 0.0,
//         }
//     }
// }
//

fn teleport(
    mut tp_reader: EventReader<TPCameraTo>,
    mut q_gimbal_cam: Query<(&mut Transform, &mut Gimbal)>,
) -> Result {
    // TODO: Should take care of transform hiearchy
    if let Some(tp) = tp_reader.read().last() {
        let (mut cam_t, mut cam_g) = q_gimbal_cam.single_mut()?;
        cam_t.translation = tp.pos;
        *cam_g = tp.rot;
    }
    Ok(())
}

fn gimbal_mouse_input(
    mut mouse_motion: EventReader<MouseMotion>,
    mut q_gimbal: Query<&mut Gimbal>,
) {
    for motion in mouse_motion.read() {
        if let Ok(mut gimbal) = q_gimbal.single_mut() {
            gimbal.pitch_yaw += motion.delta.yx() * 0.0022;
        }
    }
}

fn gimbal_binding_input(
    input: Res<Axis<BindingAxis>>,
    time: Res<Time>,
    mut q_gimbal: Query<&mut Gimbal>,
) {
    if let Ok(mut gimbal) = q_gimbal.single_mut() {
        let look_vec = input.look_vec();
        gimbal.pitch_yaw += Vec2::new(-look_vec.y, look_vec.x) * time.delta_secs() * 1.5;
    }
}

fn gimbal_limit(mut q_gimbal: Query<&mut Gimbal, Changed<Gimbal>>) {
    if let Ok(mut gimbal) = q_gimbal.single_mut() {
        gimbal.pitch_yaw.x = gimbal.pitch_yaw.x.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
}

fn gimbal_rotation(
    mut q_gimbal_changed: Query<
        (&Gimbal, &mut Transform, Has<GimbalRotatesParent>),
        Changed<Gimbal>,
    >,
) {
    for (gimbal, mut transform, should_rotate_parent) in q_gimbal_changed.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            if should_rotate_parent {
                0.0
            } else {
                -gimbal.pitch_yaw.y
            },
            -gimbal.pitch_yaw.x,
            gimbal.roll,
        )
    }
}

#[allow(clippy::type_complexity)]
fn gimbal_parent_rotation(
    q_gimbal_changed: Query<(&Gimbal, &ChildOf), (Changed<Gimbal>, With<GimbalRotatesParent>)>,
    mut q_transforms: Query<&mut Transform, Without<GimbalRotatesParent>>, // ensure parallel compability
) {
    for (gimbal, child_of) in q_gimbal_changed.iter() {
        if let Ok(mut parent_transform) = q_transforms.get_mut(child_of.parent()) {
            parent_transform.rotation =
                Quat::from_euler(EulerRot::YXZ, -gimbal.pitch_yaw.y, 0.0, 0.0)
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_event::<TPCameraTo>();
    app.add_systems(
        PreUpdate,
        (
            gimbal_mouse_input.run_if(
                on_event::<MouseMotion>
                    .and(in_state(AppState::InGame).or(in_state(FreelookState::Locked))),
            ),
            gimbal_binding_input,
            gimbal_limit,
        )
            .chain()
            .after(InputBindingSystem),
    );
    app.add_systems(Update, (gimbal_rotation, gimbal_parent_rotation, teleport));
}
