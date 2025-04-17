use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::RenderAssetUsages,
    input::{
        common_conditions::{input_just_pressed, input_just_released},
        mouse::MouseMotion,
    },
    math::VectorSpace,
    prelude::*,
    window::PrimaryWindow,
    winit::select_monitor,
};

#[derive(Default, Clone, PartialEq, Eq)]
pub enum SelAxis {
    X,
    #[default]
    Y,
    Z,
}

impl SelAxis {
    pub fn as_plane(&self) -> InfinitePlane3d {
        match self {
            SelAxis::X => InfinitePlane3d::new(Dir3::X),
            SelAxis::Y => InfinitePlane3d::new(Dir3::Y),
            SelAxis::Z => InfinitePlane3d::new(Dir3::Z),
        }
    }
    pub fn as_plane_parallel(&self) -> InfinitePlane3d {
        match self {
            SelAxis::X => InfinitePlane3d::new(Dir3::Y),
            SelAxis::Y => InfinitePlane3d::new(Dir3::X),
            SelAxis::Z => InfinitePlane3d::new(Dir3::Y),
        }
    }
    pub fn as_unit_vec(&self) -> Vec3 {
        match self {
            SelAxis::X => Vec3::X,
            SelAxis::Y => Vec3::Y,
            SelAxis::Z => Vec3::Z,
        }
    }
}

#[derive(Resource, Default)]
pub struct Sel {
    pub position: Option<Vec3>,
    // target object (entity id?)
    pub axis: SelAxis,
    pub axis_offset: f32,
    pub snap: bool,
}

impl Sel {
    pub fn grid_center(&self) -> Vec3 {
        self.axis.as_unit_vec()
            * if self.snap {
                self.axis_offset.round()
            } else {
                self.axis_offset
            }
    }
}

#[derive(Component)]
pub struct SelMarker;

#[derive(Component)]
pub struct SelGrid;

#[derive(Event)]
pub struct SelChanged;

#[derive(States, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SelMode {
    #[default]
    Normal,
    // i guess this should be more complex than a state, its one of those actions you want to be
    // able to press escape to cancel, like in blender.
    MoveAxisOffset,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_event::<SelChanged>()
        .add_systems(Startup, (init_selection, create_grid))
        .init_state::<SelMode>()
        .add_systems(
            PreUpdate,
            (
                update_sel_from_mouse
                    .run_if(on_event::<MouseMotion>.and(in_state(SelMode::Normal))),
                move_axis_offset
                    .run_if(on_event::<MouseMotion>.and(in_state(SelMode::MoveAxisOffset))),
                switch_sel_axis(SelAxis::X).run_if(input_just_pressed(KeyCode::KeyX)),
                switch_sel_axis(SelAxis::Z).run_if(input_just_pressed(KeyCode::KeyZ)),
                switch_sel_axis(SelAxis::Y).run_if(input_just_pressed(KeyCode::KeyC)),
                toggle_snap.run_if(input_just_pressed(KeyCode::KeyT)),
                toggle_snap.run_if(input_just_pressed(KeyCode::AltLeft)),
                toggle_snap.run_if(input_just_released(KeyCode::AltLeft)),
            ),
        )
        .add_systems(Update, reposition_sel_grid.run_if(on_event::<SelChanged>));
}

fn init_selection(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    commands.spawn((
        SelMarker,
        Mesh3d(meshes.add(Sphere::new(0.1))),
        MeshMaterial3d(materials.add(Color::srgb_u8(255, 102, 144))),
    ));
}

fn create_grid(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let mut grid_mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineList,
        RenderAssetUsages::RENDER_WORLD,
    );

    const GRID_SIZE: i32 = 128;

    let mut vertices = Vec::<[f32; 3]>::new();

    for i in -GRID_SIZE..=GRID_SIZE {
        vertices.push([i as f32, 0.0, -GRID_SIZE as f32]);
        vertices.push([i as f32, 0.0, GRID_SIZE as f32]);
        vertices.push([-GRID_SIZE as f32, 0.0, i as f32]);
        vertices.push([GRID_SIZE as f32, 0.0, i as f32]);
    }

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    commands.spawn((
        SelGrid,
        Mesh3d(meshes.add(grid_mesh)),
        MeshMaterial3d(materials.add(Color::srgba(1.0, 1.0, 1.0, 0.5))),
    ));
}

fn reposition_sel_grid(sel: Res<Sel>, mut q_grid: Query<&mut Transform, With<SelGrid>>) {
    let mut trans = q_grid.single_mut();
    trans.translation = sel.grid_center();
    trans.rotation = match sel.axis {
        SelAxis::X => Quat::from_rotation_z(FRAC_PI_2),
        SelAxis::Y => Quat::IDENTITY,
        SelAxis::Z => Quat::from_rotation_x(FRAC_PI_2),
    }
}

pub fn switch_sel_axis(
    new_axis: SelAxis,
) -> impl Fn(ResMut<Sel>, Res<State<SelMode>>, ResMut<NextState<SelMode>>, EventWriter<SelChanged>)
{
    move |mut sel, current_sel_mode, mut next_sel_mode, mut sel_changed| {
        if sel.axis == new_axis {
            next_sel_mode.set(if current_sel_mode.get() == &SelMode::MoveAxisOffset {
                SelMode::Normal
            } else {
                SelMode::MoveAxisOffset
            });
        } else {
            next_sel_mode.set(SelMode::Normal);
            if let Some(selected_pos) = sel.position {
                sel.axis_offset = match new_axis {
                    SelAxis::X => selected_pos.x,
                    SelAxis::Y => selected_pos.y,
                    SelAxis::Z => selected_pos.z,
                }
            }
            sel.axis = new_axis.clone();
            sel_changed.send(SelChanged);
        }
    }
}

pub fn toggle_snap(mut sel: ResMut<Sel>, mut sel_changed: EventWriter<SelChanged>) {
    sel.snap = !sel.snap;
    sel_changed.send(SelChanged);
    // TODO: Should the grid offset snap into place when toggling snap? Right now it de-snaps again
    // when snap is disabled.
}

pub fn move_axis_offset(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut q_sel_marker: Query<&mut Transform, With<SelMarker>>,
    mut sel: ResMut<Sel>,
    mut sel_changed: EventWriter<SelChanged>,
) {
    let window = q_window.single();
    if let Some(mouse_pos) = window.cursor_position() {
        let (cam, cam_trans) = q_camera.single();
        if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
            let plane = sel.axis.as_plane_parallel();
            if let Some(dist) = ray.intersect_plane(Vec3::ZERO, plane) {
                let point = ray.get_point(dist);
                let point = if sel.snap { Vec3::round(point) } else { point };

                sel.axis_offset = point.y;

                sel_changed.send(SelChanged);
            }
        }
    }
}

pub fn update_sel_from_mouse(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut q_sel_marker: Query<&mut Transform, With<SelMarker>>,
    mut sel: ResMut<Sel>,
) {
    let window = q_window.single();
    if let Some(mouse_pos) = window.cursor_position() {
        let (cam, cam_trans) = q_camera.single();
        if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
            let plane = sel.axis.as_plane();
            if let Some(dist) = ray.intersect_plane(sel.grid_center(), plane) {
                let point = ray.get_point(dist);
                let point = if sel.snap { Vec3::round(point) } else { point };
                sel.position = Some(point);
                if let Ok(mut marker_tranform) = q_sel_marker.get_single_mut() {
                    marker_tranform.translation = point;
                }
            }
        }
    }
}
