use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::RenderAssetUsages,
    input::{common_conditions::input_just_pressed, mouse::MouseMotion},
    math::VectorSpace,
    prelude::*,
    window::PrimaryWindow,
};

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_event::<SelAxisChanged>()
        .add_systems(Startup, (init, create_grid))
        .add_systems(
            PreUpdate,
            (
                update_sel_from_mouse.run_if(on_event::<MouseMotion>),
                switch_sel_axis(SelAxis::X).run_if(input_just_pressed(KeyCode::KeyX)),
                switch_sel_axis(SelAxis::Z).run_if(input_just_pressed(KeyCode::KeyZ)),
                switch_sel_axis(SelAxis::Y).run_if(input_just_pressed(KeyCode::KeyC)),
            ),
        )
        .add_systems(Update, (flip_sel_grid));
}

#[derive(Default, Clone)]
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
    pub axis: SelAxis,
    pub axis_offset: f32,
}

impl Sel {
    pub fn grid_center(&self) -> Vec3 {
        self.axis.as_unit_vec() * self.axis_offset
    }
}

#[derive(Component)]
pub struct SelMarker;

#[derive(Component)]
pub struct SelGrid;

#[derive(Event)]
pub struct SelAxisChanged {
    axis: SelAxis,
    axis_offset: f32,
}

fn init(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    commands.spawn((
        SelMarker,
        Mesh3d(meshes.add(Sphere::new(1.0))),
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

fn flip_sel_grid(
    mut q_grid: Query<&mut Transform, With<SelGrid>>,
    mut changed: EventReader<SelAxisChanged>,
) {
    let mut trans = q_grid.single_mut();
    if let Some(changed_ev) = changed.read().last() {
        trans.translation = changed_ev.axis.as_unit_vec() * changed_ev.axis_offset;
        trans.rotation = match changed_ev.axis {
            SelAxis::X => Quat::from_rotation_z(FRAC_PI_2),
            SelAxis::Y => Quat::IDENTITY,
            SelAxis::Z => Quat::from_rotation_x(FRAC_PI_2),
        }
    }
}

pub fn switch_sel_axis(new_axis: SelAxis) -> impl Fn(ResMut<Sel>, EventWriter<SelAxisChanged>) {
    move |mut sel, mut sel_axis_changed| {
        if let Some(selected_pos) = sel.position {
            sel.axis_offset = match new_axis {
                SelAxis::X => selected_pos.x,
                SelAxis::Y => selected_pos.y,
                SelAxis::Z => selected_pos.z,
            }
        }

        sel.axis = new_axis.clone();
        sel_axis_changed.send(SelAxisChanged {
            axis: new_axis.clone(),
            axis_offset: sel.axis_offset,
        });
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
                sel.position = Some(point);
                if let Ok(mut marker_tranform) = q_sel_marker.get_single_mut() {
                    marker_tranform.translation = point;
                }
            }
        }
    }
}
