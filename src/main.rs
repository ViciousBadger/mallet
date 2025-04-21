mod action;
mod app_data;
mod camera;
mod map;
mod selection;
mod util;

use action::EditorAction;
use bevy::{
    app::AppExit,
    asset::{io::AssetSourceBuilder, RenderAssetUsages},
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    winit::WinitSettings,
};
use camera::{
    freelook_input, freelook_input_reset, freelook_movement, gimbal_mouse_rotation,
    redraw_window_on_velocity, Freelook,
};
use color_eyre::eyre::Result;
use util::{enter_state, grab_mouse, release_mouse, IdGen};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum EditorState {
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
                //swap_editor_state.run_if(input_just_pressed(MouseButton::Right)),
                enter_state(EditorState::Fly).run_if(input_just_pressed(MouseButton::Right)),
                enter_state(EditorState::Select).run_if(input_just_released(MouseButton::Right)),
                exit_app
                    .run_if(in_state(EditorAction::None).and(input_just_pressed(KeyCode::Escape))),
                freelook_input,
                gimbal_mouse_rotation.run_if(in_state(EditorState::Fly)),
            ),
        )
        .add_systems(Update, (freelook_movement, redraw_window_on_velocity))
        .add_systems(OnEnter(EditorState::Fly), grab_mouse)
        .add_systems(
            OnExit(EditorState::Fly),
            (release_mouse, freelook_input_reset),
        )
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

fn exit_app(mut exit_events: ResMut<Events<AppExit>>) {
    exit_events.send_default();
}
