use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::RequestRedraw,
};

use crate::util::move_toward_3d;

#[derive(Component, Default)]
pub struct Gimbal {
    yaw_pitch: Vec2,
}

pub fn gimbal_mouse_rotation(
    mut q_camera_rotator: Query<(&mut Gimbal, &mut Transform)>,
    mut mouse_motion: EventReader<MouseMotion>,
) {
    for motion in mouse_motion.read() {
        let (mut rotator, mut transform) = q_camera_rotator.single_mut();
        rotator.yaw_pitch += motion.delta * 0.0022;

        transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            -rotator.yaw_pitch.x,
            -rotator.yaw_pitch.y,
            0.0,
        )
    }
}

#[derive(Component)]
#[require(Camera3d, Gimbal)]
pub struct Freelook {
    target_move: Vec3,
    velocity: Vec3,
    speed: i32,
}

impl Default for Freelook {
    fn default() -> Self {
        Freelook {
            target_move: Vec3::ZERO,
            velocity: Vec3::ZERO,
            speed: 5,
        }
    }
}

pub fn freelook_input(
    mut q_freelook: Query<&mut Freelook>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,
) {
    let mut freelook = q_freelook.single_mut();
    let mut raw_move = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        raw_move -= Vec3::Z
    }
    if keyboard.pressed(KeyCode::KeyA) {
        raw_move -= Vec3::X
    }
    if keyboard.pressed(KeyCode::KeyS) {
        raw_move += Vec3::Z
    }
    if keyboard.pressed(KeyCode::KeyD) {
        raw_move += Vec3::X
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        raw_move -= Vec3::Y
    }
    if keyboard.pressed(KeyCode::Space) {
        raw_move += Vec3::Y
    }

    freelook.target_move = raw_move.normalize_or_zero();

    for event in mouse_wheel.read() {
        freelook.speed = (freelook.speed + if event.y > 0.0 { 1 } else { -1 }).clamp(1, 10);
    }
}

pub fn freelook_input_reset(mut q_freelook: Query<&mut Freelook>) {
    q_freelook.single_mut().target_move = Vec3::ZERO;
}

pub fn freelook_movement(mut q_freelook: Query<(&mut Freelook, &mut Transform)>, time: Res<Time>) {
    let (mut freelook, mut transform) = q_freelook.single_mut();

    let xz_movement = freelook.target_move.xz().rotate(Vec2::from_angle(
        -transform.rotation.to_euler(EulerRot::YXZ).0,
    ));

    let max_speed = (freelook.speed as f32).powf(1.5);
    let accel = max_speed * 8.0;

    let adjusted_move = Vec3::new(xz_movement.x, freelook.target_move.y, xz_movement.y) * max_speed;

    freelook.velocity = move_toward_3d(
        freelook.velocity,
        adjusted_move,
        time.delta_secs() * accel * 0.5,
    );

    // if freelook.velocity.length() > max_speed {
    //     freelook.velocity = freelook.velocity.normalize_or_zero() * max_speed;
    // }

    transform.translation += freelook.velocity * time.delta_secs();

    freelook.velocity = move_toward_3d(
        freelook.velocity,
        adjusted_move,
        time.delta_secs() * accel * 0.5,
    );
}

pub fn redraw_window_on_velocity(
    q_freelook: Query<&Freelook>,
    mut request_redraw_events: EventWriter<RequestRedraw>,
) {
    if q_freelook
        .iter()
        .any(|f| f.velocity.length() > 0.001 || f.target_move.length() > 0.001)
    {
        request_redraw_events.send(RequestRedraw);
    }
}
