use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css,
    gizmos::grid,
    input::{
        common_conditions::{input_just_pressed, input_just_released},
        mouse::MouseMotion,
    },
    prelude::*,
    window::PrimaryWindow,
};

#[derive(Default, Clone, PartialEq, Eq)]
pub enum SelAxis {
    X,
    #[default]
    Y,
    Z,
}

impl SelAxis {
    pub fn as_unit_vec(&self) -> Vec3 {
        match self {
            SelAxis::X => Vec3::X,
            SelAxis::Y => Vec3::Y,
            SelAxis::Z => Vec3::Z,
        }
    }

    pub fn as_plane(&self) -> InfinitePlane3d {
        InfinitePlane3d::new(Dir3::new_unchecked(self.as_unit_vec()))
    }
}

#[derive(Resource, Default)]
pub struct Sel {
    pub position: Vec3,
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
    pub fn as_isometry(&self) -> Isometry3d {
        self.axis.as_plane().isometry_from_xy(self.grid_center())
    }
}

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
        .init_state::<SelMode>()
        .add_systems(
            PreUpdate,
            (
                move_selected_pos.run_if(in_state(SelMode::Normal).and(on_event::<MouseMotion>)),
                move_axis_offset
                    .run_if(in_state(SelMode::MoveAxisOffset).and(on_event::<MouseMotion>)),
                switch_sel_axis(SelAxis::X).run_if(input_just_pressed(KeyCode::KeyX)),
                switch_sel_axis(SelAxis::Z).run_if(input_just_pressed(KeyCode::KeyZ)),
                switch_sel_axis(SelAxis::Y).run_if(input_just_pressed(KeyCode::KeyC)),
                toggle_snap.run_if(input_just_pressed(KeyCode::KeyT)),
                toggle_snap.run_if(input_just_pressed(KeyCode::AltLeft)),
                toggle_snap.run_if(input_just_released(KeyCode::AltLeft)),
            ),
        )
        // .add_systems(Update, reposition_sel_grid.run_if(on_event::<SelChanged>))
        .add_systems(PostUpdate, draw_sel_gizmos);
}

fn draw_sel_gizmos(sel: Res<Sel>, sel_mode: Res<State<SelMode>>, mut gizmos: Gizmos) {
    let grid_line_color = css::DIM_GRAY.with_alpha(0.1);

    gizmos.grid(
        sel.as_isometry(),
        UVec2::new(128, 128),
        Vec2::ONE,
        grid_line_color,
    );

    let sel_trans = Transform::IDENTITY
        .with_translation(sel.position)
        .with_scale(Vec3::ONE * 0.1);
    gizmos.cuboid(sel_trans, css::INDIAN_RED);

    if sel_mode.get() == &SelMode::MoveAxisOffset {
        gizmos.line(
            sel.position + sel.axis.as_unit_vec() * -100.0,
            sel.position + sel.axis.as_unit_vec() * 100.0,
            css::INDIAN_RED,
        );
    }
}

fn switch_sel_axis(
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
            sel.axis_offset = match new_axis {
                SelAxis::X => sel.position.x,
                SelAxis::Y => sel.position.y,
                SelAxis::Z => sel.position.z,
            };
            sel.axis = new_axis.clone();
            sel_changed.send(SelChanged);
        }
    }
}

fn toggle_snap(mut sel: ResMut<Sel>, mut sel_changed: EventWriter<SelChanged>) {
    sel.snap = !sel.snap;
    sel_changed.send(SelChanged);
    // TODO: Should the grid offset snap into place when toggling snap? Right now it de-snaps again
    // when snap is disabled.
}

fn move_selected_pos(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
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
                sel.position = point;
            }
        }
    }
}

fn move_axis_offset(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut sel: ResMut<Sel>,
    mut sel_changed: EventWriter<SelChanged>,
) {
    let window = q_window.single();

    if let Some(mouse_pos) = window.cursor_position() {
        let (cam, cam_trans) = q_camera.single();

        if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
            let mut towards_cam = sel.position - cam_trans.translation();
            match sel.axis {
                SelAxis::X => towards_cam.x = 0.0,
                SelAxis::Y => towards_cam.y = 0.0,
                SelAxis::Z => towards_cam.z = 0.0,
            }
            towards_cam = towards_cam.normalize();
            let plane = InfinitePlane3d::new(towards_cam);

            if let Some(dist) = ray.intersect_plane(sel.position, plane) {
                let point = ray.get_point(dist);
                let point = if sel.snap { Vec3::round(point) } else { point };

                match sel.axis {
                    SelAxis::X => {
                        sel.axis_offset = point.x;
                        sel.position.x = point.x;
                    }
                    SelAxis::Y => {
                        sel.axis_offset = point.y;
                        sel.position.y = point.y;
                    }
                    SelAxis::Z => {
                        sel.axis_offset = point.z;
                        sel.position.z = point.z;
                    }
                }
                sel_changed.send(SelChanged);
            }
        }
    }
}
