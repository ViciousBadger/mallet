mod camera;
mod map;
mod util;

use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    input::keyboard::KeyboardInput,
    prelude::*,
    utils::tracing::instrument::WithSubscriber,
    window::{CursorGrabMode, PrimaryWindow},
};
use camera::{camera_rotation, freelook_input, freelook_input_reset, freelook_movement, Freelook};
use color_eyre::eyre::Result;
use csgrs::float_types::parry3d::na::Point3;
use map::{deploy_added_elements, MapElement, PropFeature};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum EditorState {
    #[default]
    Select,
    Fly,
}

type CSG = csgrs::csg::CSG<()>;

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

    //commands.spawn(FreelookCameraBundle::default());
    commands.spawn((
        Freelook::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0_f32.to_radians(),
            ..default()
        }),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    commands.spawn(MapElement::Prop {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        features: vec![PropFeature::PointLightSource],
    });

    // commands.spawn(MapElement::Brush {
    //     start: IVec3::ZERO,
    //     end: IVec3::ONE,
    // });

    let cube = CSG::cube(None);
    let sphere = CSG::sphere(None);

    let sub = cube.subtract(&sphere);

    commands.spawn((
        Mesh3d(meshes.add(csg_to_mesh(&sub))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
    ));
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

fn csg_to_mesh(input: &CSG) -> Mesh {
    let mut vertices = Vec::<Vec3>::new();
    let mut indices = Vec::<u32>::new();
    let mut index_offset = 0;

    for poly in input.polygons.iter() {
        let tris = poly.triangulate();
        for tri in &tris {
            // Each tri is [Vertex; 3]
            //  push the positions into `vertices`
            //  build the index triplet for `indices`
            for v in tri {
                vertices.push(Vec3::new(v.pos.x as f32, v.pos.y as f32, v.pos.z as f32));
            }
            indices.push(index_offset);
            indices.push(index_offset + 1);
            indices.push(index_offset + 2);
            index_offset += 3;
        }
    }

    Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))

    // TriMesh::new(Vec<[Real; 3]>, Vec<[u32; 3]>)
}
