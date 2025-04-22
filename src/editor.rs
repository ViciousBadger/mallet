pub mod actions;
pub mod freelook;
pub mod selection;

use crate::AppState;
use bevy::prelude::*;
use freelook::Freelook;

pub fn plugin(app: &mut App) {
    app.add_plugins((freelook::plugin, selection::plugin, actions::plugin))
        .add_systems(OnEnter(AppState::InEditor), init_editor)
        .add_systems(OnExit(AppState::InEditor), teardown_editor);
}

fn init_editor(mut commands: Commands) {
    commands.spawn((
        StateScoped(AppState::InEditor),
        Freelook::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0_f32.to_radians(),
            ..default()
        }),
    ));
    commands.spawn((
        StateScoped(AppState::InEditor),
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_x(-0.8)),
    ));
}

fn teardown_editor(_: Commands) {
    //Remove resources etc...
}
