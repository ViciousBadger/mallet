use bevy::{input::mouse::MouseMotion, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    core::input_binding::{BindingAxis, BindingAxisFns, InputBindingSystem},
    editor::freelook::FreelookState,
};

use super::AppState;

/// For gimbal-locked rotation.
/// Pitch=X, Yaw=Y, Roll=Z
#[derive(Component, Default, Clone, Serialize, Deserialize)]
#[require(Transform)]
pub struct Gimbal {
    pub pitch_yaw: Vec2,
    pub roll: f32,
}

#[derive(Component)]
pub struct GimbalRotatesParent;

#[derive(Event)]
pub struct TeleportGimbalCamera {
    pub new_pos: Vec3,
    pub new_gimbal: Gimbal,
}

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
    mut tp_reader: EventReader<TeleportGimbalCamera>,
    mut q_gimbal_cam: Query<(&mut Transform, &mut Gimbal)>,
) {
    // TODO: Should take care of transform hiearchy
    if let Some(last) = tp_reader.read().last() {
        let (mut cam_t, mut cam_g) = q_gimbal_cam.single_mut();
        cam_t.translation = last.new_pos;
        *cam_g = last.new_gimbal.clone();
    }
}

fn gimbal_mouse_input(
    mut mouse_motion: EventReader<MouseMotion>,
    mut q_gimbal: Query<&mut Gimbal>,
) {
    for motion in mouse_motion.read() {
        if let Ok(mut gimbal) = q_gimbal.get_single_mut() {
            gimbal.pitch_yaw += motion.delta.yx() * 0.0022;
        }
    }
}

fn gimbal_binding_input(
    input: Res<Axis<BindingAxis>>,
    time: Res<Time>,
    mut q_gimbal: Query<&mut Gimbal>,
) {
    if let Ok(mut gimbal) = q_gimbal.get_single_mut() {
        let look_vec = input.look_vec();
        gimbal.pitch_yaw += Vec2::new(-look_vec.y, look_vec.x) * time.delta_secs() * 1.5;
    }
}

fn gimbal_limit(mut q_gimbal: Query<&mut Gimbal, Changed<Gimbal>>) {
    if let Ok(mut gimbal) = q_gimbal.get_single_mut() {
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

fn gimbal_parent_rotation(
    q_gimbal_changed: Query<(&Gimbal, &Parent), (Changed<Gimbal>, With<GimbalRotatesParent>)>,
    mut q_transforms: Query<&mut Transform, Without<GimbalRotatesParent>>, // ensure parallel compability
) {
    for (gimbal, parent) in q_gimbal_changed.iter() {
        if let Ok(mut parent_transform) = q_transforms.get_mut(parent.get()) {
            parent_transform.rotation =
                Quat::from_euler(EulerRot::YXZ, -gimbal.pitch_yaw.y, 0.0, 0.0)
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_event::<TeleportGimbalCamera>()
        .add_systems(
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
        )
        .add_systems(Update, (gimbal_rotation, gimbal_parent_rotation, teleport));
}
