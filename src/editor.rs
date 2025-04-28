pub mod actions;
pub mod cursor;
pub mod freelook;
pub mod selection;
pub mod tools;
pub mod ui;

use crate::core::{
    view::{Gimbal, GimbalPos},
    AppState,
};
use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use cursor::SpatialCursor;
use freelook::Freelook;
use serde::{Deserialize, Serialize};

fn init_editor(editor_context: Res<EditorContext>, mut commands: Commands) {
    let spawn_pos = editor_context.camera_pos;
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

    commands.insert_resource(editor_context.cursor.clone());
}

fn teardown_editor(_: Commands) {
    //Remove resources etc...
}

/// Persistent editor state.
/// Used when jumping back to editor after playtesting and when saving/loading map files.
#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct EditorContext {
    pub camera_pos: GimbalPos,
    pub cursor: SpatialCursor,
}

impl Default for EditorContext {
    fn default() -> Self {
        Self {
            camera_pos: GimbalPos {
                pos: vec3(0.0, 2.0, 0.0),
                rot: Gimbal {
                    pitch_yaw: vec2(15_f32.to_radians(), 0.0),
                    roll: 0.0,
                },
            },
            cursor: default(),
        }
    }
}

pub fn update_editor_context(
    cursor: Res<SpatialCursor>,
    q_camera: Query<(&GlobalTransform, &Gimbal)>,
    mut commands: Commands,
) {
    let (cam_t, cam_g) = q_camera.single();
    let new_context = EditorContext {
        cursor: cursor.clone(),
        camera_pos: GimbalPos::new(cam_t.translation(), *cam_g),
    };
    commands.insert_resource(new_context);
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorSystems;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        freelook::plugin,
        cursor::plugin,
        selection::plugin,
        tools::plugin,
        actions::plugin,
        ui::plugin,
    ))
    .init_resource::<EditorContext>()
    .configure_sets(
        PreUpdate,
        EditorSystems.run_if(in_state(AppState::InEditor)),
    )
    .configure_sets(Update, EditorSystems.run_if(in_state(AppState::InEditor)))
    .configure_sets(
        PostUpdate,
        EditorSystems.run_if(in_state(AppState::InEditor)),
    )
    .add_systems(OnEnter(AppState::InEditor), init_editor)
    .add_systems(OnExit(AppState::InEditor), teardown_editor);
}
