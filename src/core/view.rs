use std::f32::consts::PI;

use bevy::{input::mouse::MouseMotion, prelude::*};

use crate::{
    editor::freelook::FreelookState,
    input_binding::{Binding, BindingAxis, BindingAxisFns, InputBindingSystem, InputBindings},
    AppState,
};

/// For gimbal-locked rotation.
/// Pitch=X, Yaw=Y, Roll=Z
#[derive(Component, Default, Clone)]
#[require(Transform)]
pub struct Gimbal {
    pub pitch_yaw: Vec2,
    pub roll: f32,
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

fn gimbal_rotation(mut q_gimbal_changed: Query<(&Gimbal, &mut Transform), Changed<Gimbal>>) {
    for (gimbal, mut transform) in q_gimbal_changed.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            -gimbal.pitch_yaw.y,
            -gimbal.pitch_yaw.x,
            gimbal.roll,
        )
    }
}

pub fn plugin(app: &mut App) {
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
    )
    .add_systems(Update, gimbal_rotation);
}
