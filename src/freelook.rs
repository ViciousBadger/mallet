use bevy::{
    input::{common_conditions::input_just_pressed, mouse::MouseMotion},
    prelude::*,
    window::RequestRedraw,
};

use crate::{
    input_binding::{Binding, InputBindingSystem},
    util::{grab_mouse, release_mouse, Gimbal},
    EditorState,
};

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
            speed: 5,
            target_move: Vec3::ZERO,
            velocity: Vec3::ZERO,
        }
    }
}

// Should be shared w/ first-person look code at some point.
fn gimbal_mouse_rotation(
    mut q_gimbal: Query<(&mut Gimbal, &mut Transform)>,
    mut mouse_motion: EventReader<MouseMotion>,
) {
    for motion in mouse_motion.read() {
        let (mut rotator, mut transform) = q_gimbal.single_mut();
        rotator.yaw_pitch += motion.delta * 0.0022;

        transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            -rotator.yaw_pitch.x,
            -rotator.yaw_pitch.y,
            0.0,
        )
    }
}

fn freelook_input(mut q_freelook: Query<&mut Freelook>, input: Res<ButtonInput<Binding>>) {
    let mut freelook = q_freelook.single_mut();
    let mut raw_move = Vec3::ZERO;

    if input.pressed(Binding::MoveForwards) {
        raw_move -= Vec3::Z
    }
    if input.pressed(Binding::MoveLeft) {
        raw_move -= Vec3::X
    }
    if input.pressed(Binding::MoveBackwards) {
        raw_move += Vec3::Z
    }
    if input.pressed(Binding::MoveRight) {
        raw_move += Vec3::X
    }
    if input.pressed(Binding::MoveDown) {
        raw_move -= Vec3::Y
    }
    if input.pressed(Binding::MoveUp) {
        raw_move += Vec3::Y
    }

    freelook.target_move = raw_move.normalize_or_zero();
}

fn modify_freelook_speed(change: i32) -> impl Fn(Query<&mut Freelook>) {
    move |mut q_freelook| {
        let mut freelook = q_freelook.single_mut();
        freelook.speed = (freelook.speed + change).clamp(1, 10);
    }
}

fn freelook_input_reset(mut q_freelook: Query<&mut Freelook>) {
    q_freelook.single_mut().target_move = Vec3::ZERO;
}

fn freelook_movement(mut q_freelook: Query<(&mut Freelook, &mut Transform)>, time: Res<Time>) {
    let (mut freelook, mut transform) = q_freelook.single_mut();

    let xz_movement = freelook.target_move.xz().rotate(Vec2::from_angle(
        -transform.rotation.to_euler(EulerRot::YXZ).0,
    ));

    let max_speed = (freelook.speed as f32).powf(1.5);
    let accel = max_speed * 8.0;

    let adjusted_move = Vec3::new(xz_movement.x, freelook.target_move.y, xz_movement.y) * max_speed;

    freelook.velocity = freelook
        .velocity
        .move_towards(adjusted_move, time.delta_secs() * accel);

    transform.translation += freelook.velocity * time.delta_secs();
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        PreUpdate,
        (
            freelook_input,
            modify_freelook_speed(1).run_if(input_just_pressed(Binding::FlySpeedUp)),
            modify_freelook_speed(-1).run_if(input_just_pressed(Binding::FlySpeedDown)),
            gimbal_mouse_rotation.run_if(in_state(EditorState::Fly)),
        )
            .after(InputBindingSystem),
    )
    .add_systems(Update, freelook_movement)
    .add_systems(OnEnter(EditorState::Fly), grab_mouse)
    .add_systems(
        OnExit(EditorState::Fly),
        (release_mouse, freelook_input_reset),
    );
}
