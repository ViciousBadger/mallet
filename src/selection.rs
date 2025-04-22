use bevy::{
    color::palettes::css,
    input::{
        common_conditions::{input_just_pressed, input_just_released},
        mouse::MouseMotion,
    },
    prelude::*,
    window::PrimaryWindow,
};

use crate::{keybinds::KeyBind, util::input_just_toggled};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
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
    pub origin: Vec3,
    pub position: Vec3,
    pub target: Option<Entity>,
    pub axis: SelAxis,
    pub axis_offset: f32,
    pub snap: bool,
}

impl Sel {
    pub fn plane_center(&self) -> Vec3 {
        self.axis.as_unit_vec()
            * if self.snap {
                self.axis_offset.round()
            } else {
                self.axis_offset
            }
    }

    pub fn min_pos(&self) -> Vec3 {
        self.origin - Vec3::ONE * SEL_DIST_LIMIT
    }

    pub fn max_pos(&self) -> Vec3 {
        self.origin + Vec3::ONE * SEL_DIST_LIMIT
    }

    pub fn clamp_vec(&self, vec: Vec3) -> Vec3 {
        vec.clamp(self.min_pos(), self.max_pos())
    }
}

#[derive(Event)]
pub struct SelChanged;

#[derive(States, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SelMode {
    #[default]
    Normal,
    AxisLocked(SelAxis),
}

impl SelMode {
    pub fn is_axis_locked(&self) -> bool {
        match self {
            SelMode::Normal => false,
            SelMode::AxisLocked(_) => true,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct SelIsAxisLocked;
impl ComputedStates for SelIsAxisLocked {
    type SourceStates = SelMode;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        sources.is_axis_locked().then(|| SelIsAxisLocked)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct LockedAxis(SelAxis);
impl ComputedStates for LockedAxis {
    type SourceStates = SelMode;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            SelMode::Normal => None,
            SelMode::AxisLocked(sel_axis) => Some(LockedAxis(sel_axis.clone())),
        }
    }
}

impl LockedAxis {
    pub fn get_axis(&self) -> &SelAxis {
        &self.0
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_event::<SelChanged>()
        .init_state::<SelMode>()
        .add_computed_state::<SelIsAxisLocked>()
        .add_computed_state::<LockedAxis>()
        .add_systems(
            PreUpdate,
            (
                select_normal.run_if(in_state(SelMode::Normal).and(on_event::<MouseMotion>)),
                select_locked.run_if(in_state(SelIsAxisLocked).and(on_event::<MouseMotion>)),
                reset_axis_offset.run_if(input_just_pressed(KeyBind::ResetSelAxisOffset)),
                (
                    switch_sel_axis(SelAxis::X).run_if(input_just_pressed(KeyBind::SetSelAxisX)),
                    switch_sel_axis(SelAxis::Y).run_if(input_just_pressed(KeyBind::SetSelAxisY)),
                    switch_sel_axis(SelAxis::Z).run_if(input_just_pressed(KeyBind::SetSelAxisZ)),
                )
                    .run_if(in_state(SelMode::Normal)),
                set_axis_lock(SelAxis::X).run_if(input_just_pressed(KeyBind::AxisLockX)),
                set_axis_lock(SelAxis::Y).run_if(input_just_pressed(KeyBind::AxisLockY)),
                set_axis_lock(SelAxis::Z).run_if(input_just_pressed(KeyBind::AxisLockZ)),
                reset_sel_mode.run_if(
                    input_just_released(KeyBind::AxisLockX).or(input_just_released(
                        KeyBind::AxisLockY,
                    )
                    .or(input_just_released(KeyBind::AxisLockZ)
                        .or(input_just_released(KeyBind::AxisLockSelected)))),
                ),
                set_axis_lock(SelAxis::Y).run_if(input_just_pressed(KeyBind::AxisLockY)),
                set_axis_lock(SelAxis::Z).run_if(input_just_pressed(KeyBind::AxisLockZ)),
                set_axis_lock_selected.run_if(input_just_pressed(KeyBind::AxisLockSelected)),
                toggle_snap.run_if(
                    input_just_pressed(KeyCode::KeyT).or(input_just_toggled(KeyCode::AltLeft)),
                ),
            ),
        )
        // .add_systems(Update, reposition_sel_grid.run_if(on_event::<SelChanged>))
        .add_systems(PostUpdate, (draw_sel_gizmos, move_grid_origin_to_camera));
}

fn draw_sel_gizmos(sel: Res<Sel>, sel_mode: Res<State<SelMode>>, mut gizmos: Gizmos) {
    let grid_line_color = css::DIM_GRAY.with_alpha(0.1);

    let grid_center = match sel.axis {
        SelAxis::X => Vec3::new(sel.axis_offset, sel.origin.y, sel.origin.z),
        SelAxis::Y => Vec3::new(sel.origin.x, sel.axis_offset, sel.origin.z),
        SelAxis::Z => Vec3::new(sel.origin.x, sel.origin.y, sel.axis_offset),
    };
    let mut iso = sel.axis.as_plane().isometry_from_xy(sel.plane_center());
    iso.translation = grid_center.into();
    gizmos.grid(
        iso,
        UVec2::new(SEL_DIST_LIMIT as u32 * 2, SEL_DIST_LIMIT as u32 * 2),
        Vec2::ONE,
        grid_line_color,
    );

    let sel_trans = Transform::IDENTITY
        .with_translation(sel.position)
        .with_scale(Vec3::ONE * 0.1);
    gizmos.cuboid(sel_trans, css::INDIAN_RED);

    let sel_color = css::GOLD;

    let min = sel.min_pos();
    let max = sel.max_pos();

    // X axis
    gizmos.line(
        Vec3::new(min.x, sel.position.y, sel.position.z),
        Vec3::new(max.x, sel.position.y, sel.position.z),
        if *sel_mode == SelMode::AxisLocked(SelAxis::X) {
            sel_color
        } else {
            css::BLUE_VIOLET.with_alpha(0.3)
        },
    );

    // Y axis
    gizmos.line(
        Vec3::new(sel.position.x, min.y, sel.position.z),
        Vec3::new(sel.position.x, max.y, sel.position.z),
        if *sel_mode == SelMode::AxisLocked(SelAxis::Y) {
            sel_color
        } else {
            css::INDIAN_RED.with_alpha(0.3)
        },
    );

    // Z axis
    gizmos.line(
        Vec3::new(sel.position.x, sel.position.y, min.z),
        Vec3::new(sel.position.x, sel.position.y, max.z),
        if *sel_mode == SelMode::AxisLocked(SelAxis::Z) {
            sel_color
        } else {
            css::SPRING_GREEN.with_alpha(0.3)
        },
    );
}

fn switch_sel_axis(new_axis: SelAxis) -> impl Fn(ResMut<Sel>) {
    move |mut sel| {
        sel.axis_offset = match new_axis {
            SelAxis::X => sel.position.x,
            SelAxis::Y => sel.position.y,
            SelAxis::Z => sel.position.z,
        };
        sel.axis = new_axis.clone();
    }
}

fn set_axis_lock(axis: SelAxis) -> impl Fn(Res<State<SelMode>>, ResMut<NextState<SelMode>>) {
    move |cur_sel_mode, mut next_sel_mode| {
        if cur_sel_mode.get() != &SelMode::AxisLocked(axis.clone()) {
            next_sel_mode.set(SelMode::AxisLocked(axis.clone()));
        } else {
            next_sel_mode.set(SelMode::Normal);
        }
    }
}

fn reset_sel_mode(mut next_sel_mode: ResMut<NextState<SelMode>>) {
    next_sel_mode.set(SelMode::Normal)
}

fn set_axis_lock_selected(sel: Res<Sel>, mut next_sel_mode: ResMut<NextState<SelMode>>) {
    next_sel_mode.set(SelMode::AxisLocked(sel.axis.clone()));
}

fn toggle_snap(mut sel: ResMut<Sel>, mut sel_changed: EventWriter<SelChanged>) {
    sel.snap = !sel.snap;
    sel_changed.send(SelChanged);
    // TODO: Should the grid offset snap into place when toggling snap? Right now it de-snaps again
    // when snap is disabled.
}

const SEL_DIST_LIMIT: f32 = 64.0;

fn move_grid_origin_to_camera(
    q_camera: Query<&GlobalTransform, With<Camera>>,
    mut sel: ResMut<Sel>,
) {
    if let Ok(camera_transform) = q_camera.get_single() {
        sel.origin = camera_transform.translation().round();
        // Ignore limits for current axis.. this may be dumb (u can move the axis offset really far away)
        match sel.axis {
            SelAxis::X => sel.origin.x = sel.axis_offset,
            SelAxis::Y => sel.origin.y = sel.axis_offset,
            SelAxis::Z => sel.origin.z = sel.axis_offset,
        }
    }
}

fn select_normal(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut sel: ResMut<Sel>,
) {
    let window = q_window.single();

    if let Some(mouse_pos) = window.cursor_position() {
        let (cam, cam_trans) = q_camera.single();

        if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
            let plane = sel.axis.as_plane();

            if let Some(dist) = ray.intersect_plane(sel.plane_center(), plane) {
                let point = ray.get_point(dist);
                let point = if sel.snap { Vec3::round(point) } else { point };
                let point = point.clamp(sel.min_pos(), sel.max_pos());
                sel.position = point;
            }
        }
    }
}

fn select_locked(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    locked_axis: Res<State<LockedAxis>>,
    mut sel: ResMut<Sel>,
    mut sel_changed: EventWriter<SelChanged>,
) {
    let window = q_window.single();

    let axis = locked_axis.get_axis();

    if let Some(mouse_pos) = window.cursor_position() {
        let (cam, cam_trans) = q_camera.single();

        if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
            let mut towards_cam = sel.position - cam_trans.translation();
            match axis {
                SelAxis::X => towards_cam.x = 0.0,
                SelAxis::Y => towards_cam.y = 0.0,
                SelAxis::Z => towards_cam.z = 0.0,
            }
            towards_cam = towards_cam.normalize();
            let plane = InfinitePlane3d::new(towards_cam);

            if let Some(dist) = ray.intersect_plane(sel.position, plane) {
                let point = ray.get_point(dist);
                let point = if sel.snap { Vec3::round(point) } else { point };
                let point = sel.clamp_vec(point);

                match axis {
                    SelAxis::X => {
                        if axis == &sel.axis {
                            sel.axis_offset = point.x
                        };
                        sel.position.x = point.x;
                    }
                    SelAxis::Y => {
                        if axis == &sel.axis {
                            sel.axis_offset = point.y;
                        };
                        sel.position.y = point.y;
                    }
                    SelAxis::Z => {
                        if axis == &sel.axis {
                            sel.axis_offset = point.z;
                        };
                        sel.position.z = point.z;
                    }
                }
                sel_changed.send(SelChanged);
            }
        }
    }
}

fn reset_axis_offset(mut sel: ResMut<Sel>, mut sel_changed: EventWriter<SelChanged>) {
    sel.axis_offset = 0.0;
    match sel.axis {
        SelAxis::X => sel.position.x = 0.0,
        SelAxis::Y => sel.position.y = 0.0,
        SelAxis::Z => sel.position.z = 0.0,
    };
    sel_changed.send(SelChanged);
}
