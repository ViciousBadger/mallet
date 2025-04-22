mod action;
mod app_data;
mod freelook;
mod input_binding;
mod map;
mod selection;
mod util;

use avian3d::PhysicsPlugins;
use bevy::{
    app::AppExit,
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
};
use color_eyre::eyre::Result;
use freelook::Freelook;
use input_binding::Binding;
use util::{enter_state, grab_mouse, release_mouse, IdGen};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorState {
    #[default]
    Select,
    Fly,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    App::new()
        .add_plugins((
            app_data::plugin,
            DefaultPlugins,
            PhysicsPlugins::default(),
            input_binding::plugin,
            freelook::plugin,
            selection::plugin,
            action::plugin,
            map::plugin,
        ))
        .init_state::<EditorState>()
        // Only update when there is user input. Should be disabled when in-game
        //.insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (
                enter_state(EditorState::Fly).run_if(input_just_pressed(Binding::FlyMode)),
                enter_state(EditorState::Select).run_if(input_just_released(Binding::FlyMode)),
                exit_app.run_if(input_just_pressed(Binding::Quit)),
            ),
        )
        .add_systems(PreUpdate, file_drop)
        .init_resource::<IdGen>()
        .run();

    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Freelook::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0_f32.to_radians(),
            ..default()
        }),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_x(-0.8)),
    ));
}

fn file_drop(mut evr_dnd: EventReader<FileDragAndDrop>) {
    for ev in evr_dnd.read() {
        info!("dnd event: {:?}", ev);
        if let FileDragAndDrop::DroppedFile { window, path_buf } = ev {
            info!(
                "Dropped file with path: {:?}, in window id: {:?}",
                path_buf, window
            );
        }
    }
}

fn exit_app(mut exit_events: ResMut<Events<AppExit>>) {
    exit_events.send_default();
}
