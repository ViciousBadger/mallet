use bevy::{
    input::{keyboard::KeyboardInput, mouse::MouseMotion, ButtonState},
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, cursor_grab)
        .add_systems(Update, camera_rotation)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 72.0_f32.to_radians(),
                ..default()
            }),
            ..default()
        },
        FirstPersonCameraController::default(),
    ));

    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::rgb_u8(124, 144, 255)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
}

#[derive(Component, Default)]
struct FirstPersonCameraController {
    yaw_pitch: Vec2,
}

fn cursor_grab(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut input: EventReader<KeyboardInput>,
) {
    for event in input.read() {
        if event.key_code == KeyCode::Tab && event.state.is_pressed() {
            let mut window = windows.single_mut();

            if window.cursor.grab_mode == CursorGrabMode::None {
                window.cursor.grab_mode = CursorGrabMode::Locked;
                window.cursor.visible = false;
            } else {
                window.cursor.grab_mode = CursorGrabMode::None;
                window.cursor.visible = false;
                window.cursor.visible = true;
            }
        }
    }
}

fn camera_rotation(
    mut cameras: Query<(&mut FirstPersonCameraController, &mut Transform)>,
    mut mouse_motion: EventReader<MouseMotion>,
) {
    for motion in mouse_motion.read() {
        if let Ok((mut controller, mut transform)) = cameras.get_single_mut() {
            controller.yaw_pitch += motion.delta * 0.0022;

            transform.rotation = Quat::from_euler(
                EulerRot::YXZ,
                -controller.yaw_pitch.x,
                -controller.yaw_pitch.y,
                0.0,
            )
        } else {
            // TODO: Handle no controllers / too many controllers gracefully..
            panic!("Exactly one FPCC is needed!");
        }
    }
}
