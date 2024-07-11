use bevy::{
    core::Zeroable,
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use util::move_toward_3d;

mod util;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum EditorState {
    #[default]
    Select,
    Fly,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<EditorState>()
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (
                editor_state_change,
                (camera_rotation, freelook_input).run_if(in_state(EditorState::Fly)),
            ),
        )
        .add_systems(Update, freelook_movement)
        .add_systems(OnEnter(EditorState::Fly), grab_mouse)
        .add_systems(
            OnExit(EditorState::Fly),
            (release_mouse, freelook_input_reset),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    ambient_light.color = Color::BLUE;

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 72.0_f32.to_radians(),
                ..default()
            }),
            ..default()
        },
        // TODO: Bundle these
        Freelook::default(),
        CameraRotator::default(),
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
    // TODO: handle in a functional way based on game state (InGame or Editor::FlyMode..)
    for event in input.read() {
        if event.key_code == KeyCode::Tab && event.state.is_pressed() {
            next_state.set(match current_state.get() {
                EditorState::Select => EditorState::Fly,
                EditorState::Fly => EditorState::Select,
            });
        }
    }
}

#[derive(Component)]
struct Freelook {
    target_move: Vec3,
    velocity: Vec3,
    speed: i32,
}

impl Default for Freelook {
    fn default() -> Self {
        Freelook {
            target_move: Vec3::ZERO,
            velocity: Vec3::ZERO,
            speed: 5,
        }
    }
}

#[derive(Component, Default)]
struct CameraRotator {
    yaw_pitch: Vec2,
}

fn freelook_input(
    mut q_freelook: Query<&mut Freelook>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,
) {
    if let Ok(mut freelook) = q_freelook.get_single_mut() {
        let mut raw_move = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) {
            raw_move -= Vec3::Z
        }
        if keyboard.pressed(KeyCode::KeyA) {
            raw_move -= Vec3::X
        }
        if keyboard.pressed(KeyCode::KeyS) {
            raw_move += Vec3::Z
        }
        if keyboard.pressed(KeyCode::KeyD) {
            raw_move += Vec3::X
        }
        if keyboard.pressed(KeyCode::KeyF) {
            raw_move -= Vec3::Y
        }
        if keyboard.pressed(KeyCode::KeyR) {
            raw_move += Vec3::Y
        }

        freelook.target_move = raw_move.normalize_or_zero();

        for event in mouse_wheel.read() {
            freelook.speed = (freelook.speed + if event.y > 0.0 { 1 } else { -1 }).clamp(1, 10);
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
        let accel = max_speed * 3.0;

        let adjusted_move =
            Vec3::new(xz_movement.x, freelook.target_move.y, xz_movement.y) * max_speed;

        freelook.velocity = move_toward_3d(
            freelook.velocity,
            adjusted_move,
            time.delta_seconds() * accel,
        );

        if freelook.velocity.length() > max_speed {
            freelook.velocity = freelook.velocity.normalize_or_zero() * max_speed;
        }

        transform.translation += freelook.velocity * time.delta_seconds();
    }
}

fn camera_rotation(
    mut q_camera_rotator: Query<(&mut CameraRotator, &mut Transform)>,
    mut mouse_motion: EventReader<MouseMotion>,
) {
    for motion in mouse_motion.read() {
        if let Ok((mut rotator, mut transform)) = q_camera_rotator.get_single_mut() {
            rotator.yaw_pitch += motion.delta * 0.0022;

            transform.rotation = Quat::from_euler(
                EulerRot::YXZ,
                -rotator.yaw_pitch.x,
                -rotator.yaw_pitch.y,
                0.0,
            )
        } else {
            // TODO: Handle no controllers / too many controllers gracefully..
            panic!("Exactly one FPCC is needed!");
        }
    }
}
