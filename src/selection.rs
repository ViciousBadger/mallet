use bevy::{
    input::{common_conditions::input_just_pressed, mouse::MouseMotion},
    math::VectorSpace,
    prelude::*,
    window::PrimaryWindow,
};

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_systems(Startup, init)
        .add_systems(
            PreUpdate,
            (
                update_sel_from_mouse.run_if(on_event::<MouseMotion>),
                switch_sel_axis(SelAxis::X).run_if(input_just_pressed(KeyCode::KeyX)),
                switch_sel_axis(SelAxis::Y).run_if(input_just_pressed(KeyCode::KeyY)),
                switch_sel_axis(SelAxis::Z).run_if(input_just_pressed(KeyCode::KeyZ)),
            ),
        );
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
}

#[derive(Resource, Default)]
pub struct Sel {
    pub position: Vec3,
    pub axis: SelAxis,
}

#[derive(Component)]
pub struct SelMarker;

pub fn init(
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

pub fn switch_sel_axis(new_axis: SelAxis) -> impl Fn(ResMut<Sel>) {
    move |mut sel| {
        sel.axis = new_axis.clone();
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
            if let Some(dist) = ray.intersect_plane(Vec3::ZERO, plane) {
                sel.position = ray.get_point(dist);
                info!("{:?}", sel.position);
                if let Ok(mut marker_tranform) = q_sel_marker.get_single_mut() {
                    marker_tranform.translation = sel.position;
                }
            }
        }
    }
}
