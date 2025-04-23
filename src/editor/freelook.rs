use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
};

use crate::{
    core::{
        input_binding::{Binding, BindingAxis, BindingAxisFns, InputBindingSystem},
        view::Gimbal,
    },
    util::{enter_state, grab_mouse, release_mouse},
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

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FreelookState {
    #[default]
    Unlocked,
    Locked,
}

fn freelook_input(input: Res<Axis<BindingAxis>>, mut q_freelook: Query<&mut Freelook>) {
    if let Ok(mut freelook) = q_freelook.get_single_mut() {
        freelook.target_move = input.movement_vec();
    }
}

fn modify_freelook_speed(change: i32) -> impl Fn(Query<&mut Freelook>) {
    move |mut q_freelook| {
        if let Ok(mut freelook) = q_freelook.get_single_mut() {
            freelook.speed = (freelook.speed + change).clamp(1, 10);
        }
    }
}

fn freelook_input_reset(mut q_freelook: Query<&mut Freelook>) {
    if let Ok(mut freelook) = q_freelook.get_single_mut() {
        freelook.target_move = Vec3::ZERO;
    }
}

fn freelook_movement(mut q_freelook: Query<(&mut Freelook, &mut Transform)>, time: Res<Time>) {
    if let Ok((mut freelook, mut transform)) = q_freelook.get_single_mut() {
        let xz_movement = freelook.target_move.xz().rotate(Vec2::from_angle(
            -transform.rotation.to_euler(EulerRot::YXZ).0,
        ));

        let max_speed = (freelook.speed as f32).powf(1.5);
        let accel = max_speed * 8.0;

        let adjusted_move =
            Vec3::new(xz_movement.x, freelook.target_move.y, xz_movement.y) * max_speed;

        freelook.velocity = freelook
            .velocity
            .move_towards(adjusted_move, time.delta_secs() * accel);

        transform.translation += freelook.velocity * time.delta_secs();
    }
}

pub fn plugin(app: &mut App) {
    app.init_state::<FreelookState>()
        .add_systems(
            PreUpdate,
            (
                freelook_input,
                modify_freelook_speed(1).run_if(input_just_pressed(Binding::FlySpeedUp)),
                modify_freelook_speed(-1).run_if(input_just_pressed(Binding::FlySpeedDown)),
                enter_state(FreelookState::Locked).run_if(input_just_pressed(Binding::FlyMode)),
                enter_state(FreelookState::Unlocked).run_if(input_just_released(Binding::FlyMode)),
            )
                .after(InputBindingSystem),
        )
        .add_systems(Update, freelook_movement)
        .add_systems(OnEnter(FreelookState::Locked), grab_mouse)
        .add_systems(
            OnExit(FreelookState::Locked),
            (release_mouse, freelook_input_reset),
        );
}
