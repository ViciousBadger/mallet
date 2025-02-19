mod bsp;
mod camera;
mod map;
mod util;

use bevy::{
    app::AppExit,
    input::common_conditions::input_just_pressed,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bsp::Room;
use camera::{camera_rotation, freelook_input, freelook_input_reset, freelook_movement, Freelook};
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
                swap_editor_state.run_if(input_just_pressed(KeyCode::Tab)),
                exit_app.run_if(input_just_pressed(KeyCode::Escape)),
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

    commands.spawn((
        Freelook::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0_f32.to_radians(),
            ..default()
        }),
    ));

    // commands.spawn((
    //     Mesh3d(meshes.add(Circle::new(4.0))),
    //     MeshMaterial3d(materials.add(Color::WHITE)),
    //     Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    // ));

    commands.spawn(MapElement::Prop {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        features: vec![PropFeature::PointLightSource],
    });

    let mut rooms: Vec<Room> = Vec::new();

    // Central room
    rooms.push(Room {
        start: Vec3::new(-2.0, 0.0, -2.0),
        end: Vec3::new(2.0, 3.0, 2.0),
    });

    // Hallways
    rooms.push(Room {
        start: Vec3::new(-4.0, 0.0, -1.0),
        end: Vec3::new(4.0, 2.0, 1.0),
    });

    for room in rooms {
        // commands.spawn((
        //     Transform::from_translation(room.center()),
        //     Mesh3d(meshes.add(Cuboid::from_size(room.size()))),
        //     MeshMaterial3d(materials.add(Color::srgba_u8(124, 144, 255, 128))),
        // ));

        for plane in room.planes() {
            commands.spawn((
                Transform::from_translation(plane.normal.as_vec3() * plane.dist),
                Mesh3d(meshes.add(Plane3d::new(plane.normal.as_vec3(), Vec2::new(1.0, 1.0)))),
                MeshMaterial3d(materials.add(Color::srgb_u8(124, 255, 144))),
            ));
        }
    }

    // commands.spawn(MapElement::Brush {
    //     start: IVec3::ZERO,
    //     end: IVec3::ONE,
    // });
}

fn grab_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = q_window.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn release_mouse(mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = q_window.single_mut();
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

fn swap_editor_state(
    current_state: Res<State<EditorState>>,
    mut next_state: ResMut<NextState<EditorState>>,
) {
    next_state.set(match current_state.get() {
        EditorState::Select => EditorState::Fly,
        EditorState::Fly => EditorState::Select,
    });
}

fn exit_app(mut exit_events: ResMut<Events<AppExit>>) {
    exit_events.send_default();
}
