pub mod actions;
pub mod freelook;
pub mod selection;

use crate::core::{
    map::Map,
    view::{Gimbal, GimbalPos},
    AppState,
};
use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use freelook::Freelook;

fn init_editor(mut commands: Commands, existing_map: Option<Res<Map>>) {
    let spawn_pos = if let Some(map) = existing_map {
        map.editor_context.camera_pos
    } else {
        GimbalPos {
            pos: vec3(0.0, 2.0, 0.0),
            rot: Gimbal {
                pitch_yaw: vec2(15_f32.to_radians(), 0.0),
                roll: 0.0,
            },
        }
    };

    commands.spawn((
        StateScoped(AppState::InEditor),
        Transform::from_translation(spawn_pos.pos),
        spawn_pos.rot,
        Freelook::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0_f32.to_radians(),
            ..default()
        }),
    ));
}

fn teardown_editor(_: Commands) {
    //Remove resources etc...
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorSystems;

pub fn plugin(app: &mut App) {
    app.add_plugins((freelook::plugin, selection::plugin, actions::plugin))
        .configure_sets(
            PreUpdate,
            EditorSystems.run_if(in_state(AppState::InEditor)),
        )
        .configure_sets(Update, EditorSystems.run_if(in_state(AppState::InEditor)))
        .add_systems(OnEnter(AppState::InEditor), init_editor)
        .add_systems(OnExit(AppState::InEditor), teardown_editor);
}
