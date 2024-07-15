mod camera;
mod map;
mod util;

use bevy::{
    app::AppExit,
    input::keyboard::KeyboardInput,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use camera::{
    camera_rotation, freelook_input, freelook_input_reset, freelook_movement, FreelookCameraBundle,
};
use color_eyre::eyre::Result;
use map::{deploy_added_elements, MapElement, PropFeature};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum EditorState {
    #[default]
    Select,
    Fly,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<EditorState>()
        .add_systems(Startup, setup)
        .add_systems(First, deploy_added_elements)
        .add_systems(
            PreUpdate,
            (
                editor_state_change,
                exit_listener,
                freelook_input,
                camera_rotation.run_if(in_state(EditorState::Fly)),
            ),
        )
        .add_systems(Update, freelook_movement)
        .add_systems(OnEnter(EditorState::Fly), grab_mouse)
        .add_systems(
            OnExit(EditorState::Fly),
            (release_mouse, freelook_input_reset),
        )
        .run();

    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // mut ambient_light: ResMut<AmbientLight>,
) {
    // ambient_light.color = Color::WHITE;
    // ambient_light.brightness = 1.0;

    commands.spawn(FreelookCameraBundle::default());

    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });

    commands.spawn(MapElement::Prop {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        features: vec![PropFeature::PointLightSource],
    });

    commands.spawn(MapElement::Brush {
        start: IVec3::ZERO,
        end: IVec3::ONE,
    });
}

fn grab_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = q_window.single_mut();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;
}

fn release_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = q_window.single_mut();
    window.cursor.grab_mode = CursorGrabMode::None;
    window.cursor.visible = true;
}

fn editor_state_change(
    mut input: EventReader<KeyboardInput>,
    current_state: Res<State<EditorState>>,
    mut next_state: ResMut<NextState<EditorState>>,
) {
    for event in input.read() {
        if event.key_code == KeyCode::Tab && event.state.is_pressed() {
            next_state.set(match current_state.get() {
                EditorState::Select => EditorState::Fly,
                EditorState::Fly => EditorState::Select,
            });
        }
    }
}

fn exit_listener(mut input: EventReader<KeyboardInput>, mut exit_events: ResMut<Events<AppExit>>) {
    for event in input.read() {
        if event.key_code == KeyCode::Escape && event.state.is_pressed() {
            exit_events.send_default();
        }
    }
}
