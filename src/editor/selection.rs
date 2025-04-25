use avian3d::prelude::{AnyCollider, Collider, SpatialQuery, SpatialQueryFilter};
use bevy::{
    color::palettes::css,
    input::{
        common_conditions::{input_just_pressed, input_just_released},
        mouse::MouseMotion,
    },
    prelude::*,
    text::cosmic_text::Editor,
    window::PrimaryWindow,
};

use crate::{
    core::binds::{Binding, InputBindingSystem},
    editor::freelook::FreelookState,
    util::input_just_toggled,
};

use super::EditorSystems;

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
    pub axis: SelAxis,
    pub axis_offset: f32,
    pub snap: bool,
}

impl Sel {
    pub fn grid_center(&self) -> Vec3 {
        let axis_offs_aligned = if self.snap {
            self.axis_offset.round()
        } else {
            self.axis_offset
        };
        match self.axis {
            SelAxis::X => Vec3::new(axis_offs_aligned, self.origin.y, self.origin.z),
            SelAxis::Y => Vec3::new(self.origin.x, axis_offs_aligned, self.origin.z),
            SelAxis::Z => Vec3::new(self.origin.x, self.origin.y, axis_offs_aligned),
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

#[derive(Resource, Default)]
pub struct SelTarget {
    pub primary: Option<Entity>,
    pub intersecting: Vec<Entity>,
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
        sources.is_axis_locked().then_some(SelIsAxisLocked)
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

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelGridGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelAxisGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelTargetGizmos {}

fn draw_sel_grid_gizmos(sel: Res<Sel>, mut gizmos: Gizmos<SelGridGizmos>) {
    let grid_line_color = css::DIM_GRAY.with_alpha(0.33);

    let mut iso = sel.axis.as_plane().isometry_from_xy(sel.grid_center());
    iso.translation = sel.grid_center().into();
    gizmos.grid(
        iso,
        UVec2::new(SEL_DIST_LIMIT as u32 * 2, SEL_DIST_LIMIT as u32 * 2),
        Vec2::ONE,
        grid_line_color,
    );
}

fn draw_axis_line_gizmos(
    sel: Res<Sel>,
    sel_mode: Res<State<SelMode>>,
    mut axis_gizmos: Gizmos<SelAxisGizmos>,
) {
    let sel_color = css::GOLD;
    let min = sel.min_pos();
    let max = sel.max_pos();

    // X axis
    axis_gizmos.line(
        Vec3::new(min.x, sel.position.y, sel.position.z),
        Vec3::new(max.x, sel.position.y, sel.position.z),
        if *sel_mode == SelMode::AxisLocked(SelAxis::X) {
            sel_color
        } else {
            css::BLUE_VIOLET.with_alpha(0.3)
        },
    );

    // Y axis
    axis_gizmos.line(
        Vec3::new(sel.position.x, min.y, sel.position.z),
        Vec3::new(sel.position.x, max.y, sel.position.z),
        if *sel_mode == SelMode::AxisLocked(SelAxis::Y) {
            sel_color
        } else {
            css::INDIAN_RED.with_alpha(0.3)
        },
    );

    // Z axis
    axis_gizmos.line(
        Vec3::new(sel.position.x, sel.position.y, min.z),
        Vec3::new(sel.position.x, sel.position.y, max.z),
        if *sel_mode == SelMode::AxisLocked(SelAxis::Z) {
            sel_color
        } else {
            css::SPRING_GREEN.with_alpha(0.3)
        },
    );
}

fn draw_sel_target_gizmos(
    sel_target: Res<SelTarget>,
    q_colliders: Query<(&Collider, &GlobalTransform)>,
    mut gizmos: Gizmos<SelTargetGizmos>,
) {
    for entity in sel_target.intersecting.iter() {
        if let Ok((coll, coll_transform)) = q_colliders.get(*entity) {
            let aabb = coll.aabb(coll_transform.translation(), coll_transform.rotation());
            //.grow(Vec3::ONE * 0.01);
            //
            let col = if sel_target.primary.is_some_and(|e| &e == entity) {
                css::GOLD.with_alpha(0.5)
            } else {
                css::DARK_GRAY.with_alpha(0.1)
            };

            gizmos.cuboid(
                Transform::IDENTITY
                    .with_translation(aabb.center())
                    .with_scale(aabb.size()),
                col,
            );
        }
    }
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
    next_sel_mode.set(SelMode::default())
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
    q_camera: Query<&GlobalTransform, (With<Camera>, Changed<GlobalTransform>)>,
    mut sel: ResMut<Sel>,
    mut sel_changed: EventWriter<SelChanged>,
) {
    if let Ok(camera_transform) = q_camera.get_single() {
        sel.origin = camera_transform.translation().round();
        let cur_offs = sel.axis_offset;
        match sel.axis {
            SelAxis::X => sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().x, sel.max_pos().x),
            SelAxis::Y => sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().y, sel.max_pos().y),
            SelAxis::Z => sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().z, sel.max_pos().z),
        }
        if sel.axis_offset != cur_offs {
            sel_changed.send(SelChanged);
        }
    }
}

fn select_normal(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut sel: ResMut<Sel>,
    mut sel_changed: EventWriter<SelChanged>,
) {
    let window = q_window.single();

    if let Some(mouse_pos) = window.cursor_position() {
        if let Ok((cam, cam_trans)) = q_camera.get_single() {
            if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
                let plane = sel.axis.as_plane();

                if let Some(dist) = ray.intersect_plane(sel.grid_center(), plane) {
                    let point = ray.get_point(dist);
                    let point = if sel.snap { Vec3::round(point) } else { point };
                    let point = point.clamp(sel.min_pos(), sel.max_pos());
                    sel.position = point;
                    sel_changed.send(SelChanged);
                }
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

fn find_entites_in_selection(
    sel: Res<Sel>,
    spatial_query: SpatialQuery,
    mut sel_target: ResMut<SelTarget>,
) {
    let inter = spatial_query.point_intersections(sel.position, &SpatialQueryFilter::default());

    if inter.is_empty() {
        sel_target.primary = None;
    } else if let Some(existing_prim) = sel_target.primary {
        if !inter.contains(&existing_prim) {
            sel_target.primary = Some(inter[0]);
        }
    } else {
        sel_target.primary = Some(inter[0]);
    }

    sel_target.intersecting = inter;
}

fn scroll_intersecting(num: i32) -> impl Fn(ResMut<SelTarget>) {
    move |mut sel_target| {
        if let Some(existing_prim) = sel_target.primary {
            if let Some(idx) = sel_target
                .intersecting
                .iter()
                .position(|n| n == &existing_prim)
            {
                let len = sel_target.intersecting.len() as i32;
                let mut next = idx as i32 + num;
                if next >= len {
                    next = 0;
                }
                if next < 0 {
                    next = len - 1;
                }

                sel_target.primary = Some(sel_target.intersecting[next as usize]);
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

// fn sel_brush_test(sel_target: Res<SelTarget>, brushes: Query< mut gizmos: Gizmos<SelTargetGizmos>) {
// }

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .init_resource::<SelTarget>()
        .insert_gizmo_config(
            SelGridGizmos {},
            GizmoConfig {
                line_width: 1.5,
                ..default()
            },
        )
        .insert_gizmo_config(
            SelAxisGizmos {},
            GizmoConfig {
                depth_bias: -0.01,
                ..default()
            },
        )
        .insert_gizmo_config(
            SelTargetGizmos {},
            GizmoConfig {
                line_width: 4.0,
                depth_bias: -1.0,
                ..default()
            },
        )
        .add_event::<SelChanged>()
        .init_state::<SelMode>()
        .add_computed_state::<SelIsAxisLocked>()
        .add_computed_state::<LockedAxis>()
        .add_systems(
            PreUpdate,
            (
                // Switching selectin axis (what axis is da grid on)
                (
                    switch_sel_axis(SelAxis::X).run_if(input_just_pressed(Binding::SetSelAxisX)),
                    switch_sel_axis(SelAxis::Y).run_if(input_just_pressed(Binding::SetSelAxisY)),
                    switch_sel_axis(SelAxis::Z).run_if(input_just_pressed(Binding::SetSelAxisZ)),
                )
                    .run_if(in_state(SelMode::Normal)),
                // Axis locking (sel mode 2) and offset
                set_axis_lock(SelAxis::X).run_if(input_just_pressed(Binding::AxisLockX)),
                set_axis_lock(SelAxis::Y).run_if(input_just_pressed(Binding::AxisLockY)),
                set_axis_lock(SelAxis::Z).run_if(input_just_pressed(Binding::AxisLockZ)),
                set_axis_lock_selected.run_if(input_just_pressed(Binding::AxisLockSelected)),
                reset_sel_mode.run_if(
                    input_just_released(Binding::AxisLockX).or(input_just_released(
                        Binding::AxisLockY,
                    )
                    .or(input_just_released(Binding::AxisLockZ)
                        .or(input_just_released(Binding::AxisLockSelected)))),
                ),
                reset_axis_offset.run_if(input_just_pressed(Binding::ResetSelAxisOffset)),
                // Selected targets
                scroll_intersecting(1).run_if(input_just_pressed(Binding::SelNext)),
                scroll_intersecting(-1).run_if(input_just_pressed(Binding::SelPrev)),
                // Snapping
                toggle_snap.run_if(
                    input_just_pressed(KeyCode::KeyT).or(input_just_toggled(KeyCode::AltLeft)),
                ),
            )
                .after(InputBindingSystem)
                .run_if(in_state(FreelookState::Unlocked))
                .in_set(EditorSystems),
        )
        .add_systems(
            Update,
            (
                move_grid_origin_to_camera,
                (
                    select_normal.run_if(in_state(SelMode::Normal).and(on_event::<MouseMotion>)),
                    select_locked.run_if(in_state(SelIsAxisLocked).and(on_event::<MouseMotion>)),
                )
                    .run_if(in_state(FreelookState::Unlocked)),
                find_entites_in_selection.run_if(on_event::<SelChanged>),
                (
                    draw_sel_grid_gizmos,
                    draw_axis_line_gizmos,
                    draw_sel_target_gizmos,
                ),
            )
                .chain()
                .in_set(EditorSystems),
        );
}
