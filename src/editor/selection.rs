use avian3d::prelude::{AnyCollider, Collider, SpatialQuery, SpatialQueryFilter};
use bevy::{
    color::palettes::css,
    input::{
        common_conditions::{input_just_pressed, input_just_released},
        mouse::MouseMotion,
    },
    prelude::*,
    window::PrimaryWindow,
};
use itertools::Itertools;

use crate::{
    core::{
        binds::{Binding, InputBindingSystem},
        map::{brush::Brush, MapNodeId},
    },
    editor::freelook::FreelookState,
    util::{input_just_toggled, Facing3d},
};

use super::EditorSystems;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpaceAxis {
    X,
    #[default]
    Y,
    Z,
}

impl SpaceAxis {
    pub fn as_unit_vec(&self) -> Vec3 {
        match self {
            SpaceAxis::X => Vec3::X,
            SpaceAxis::Y => Vec3::Y,
            SpaceAxis::Z => Vec3::Z,
        }
    }

    pub fn as_plane(&self) -> InfinitePlane3d {
        InfinitePlane3d::new(Dir3::new_unchecked(self.as_unit_vec()))
    }
}

#[derive(Resource, Default)]
pub struct SpaceCursor {
    pub origin: Vec3,
    pub position: Vec3,
    pub axis: SpaceAxis,
    pub axis_offset: f32,
    pub snap: bool,
}

impl SpaceCursor {
    pub fn grid_center(&self) -> Vec3 {
        let axis_offs_aligned = if self.snap {
            self.axis_offset.round()
        } else {
            self.axis_offset
        };
        match self.axis {
            SpaceAxis::X => Vec3::new(axis_offs_aligned, self.origin.y, self.origin.z),
            SpaceAxis::Y => Vec3::new(self.origin.x, axis_offs_aligned, self.origin.z),
            SpaceAxis::Z => Vec3::new(self.origin.x, self.origin.y, axis_offs_aligned),
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
pub struct SelectionTargets {
    pub focused: Option<Entity>,
    pub intersecting: Vec<Entity>,
}

#[derive(Resource, Deref, Clone, Copy)]
pub struct SelectedNode(pub MapNodeId);

#[derive(Event)]
pub struct SelectionChanged;

#[derive(States, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SelMode {
    #[default]
    Normal,
    AxisLocked(SpaceAxis),
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
pub struct LockedAxis(SpaceAxis);
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
    pub fn get_axis(&self) -> &SpaceAxis {
        &self.0
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelGridGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelAxisGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelTargetGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SelHighlightGizmos {}

fn draw_sel_grid_gizmos(sel: Res<SpaceCursor>, mut gizmos: Gizmos<SelGridGizmos>) {
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
    sel: Res<SpaceCursor>,
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
        if *sel_mode == SelMode::AxisLocked(SpaceAxis::X) {
            sel_color
        } else {
            css::BLUE_VIOLET.with_alpha(0.3)
        },
    );

    // Y axis
    axis_gizmos.line(
        Vec3::new(sel.position.x, min.y, sel.position.z),
        Vec3::new(sel.position.x, max.y, sel.position.z),
        if *sel_mode == SelMode::AxisLocked(SpaceAxis::Y) {
            sel_color
        } else {
            css::INDIAN_RED.with_alpha(0.3)
        },
    );

    // Z axis
    axis_gizmos.line(
        Vec3::new(sel.position.x, sel.position.y, min.z),
        Vec3::new(sel.position.x, sel.position.y, max.z),
        if *sel_mode == SelMode::AxisLocked(SpaceAxis::Z) {
            sel_color
        } else {
            css::SPRING_GREEN.with_alpha(0.3)
        },
    );
}

fn draw_sel_target_gizmos(
    sel_target: Res<SelectionTargets>,
    q_colliders: Query<(&Collider, &GlobalTransform)>,
    mut gizmos: Gizmos<SelTargetGizmos>,
) {
    for entity in sel_target.intersecting.iter() {
        if let Ok((coll, coll_transform)) = q_colliders.get(*entity) {
            let aabb = coll.aabb(coll_transform.translation(), coll_transform.rotation());
            //.grow(Vec3::ONE * 0.01);
            //
            let col = if sel_target.focused.is_some_and(|e| &e == entity) {
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

fn switch_sel_axis(new_axis: SpaceAxis) -> impl Fn(ResMut<SpaceCursor>) {
    move |mut sel| {
        sel.axis_offset = match new_axis {
            SpaceAxis::X => sel.position.x,
            SpaceAxis::Y => sel.position.y,
            SpaceAxis::Z => sel.position.z,
        };
        sel.axis = new_axis.clone();
    }
}

fn set_axis_lock(axis: SpaceAxis) -> impl Fn(Res<State<SelMode>>, ResMut<NextState<SelMode>>) {
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

fn set_axis_lock_selected(sel: Res<SpaceCursor>, mut next_sel_mode: ResMut<NextState<SelMode>>) {
    next_sel_mode.set(SelMode::AxisLocked(sel.axis.clone()));
}

fn toggle_snap(mut sel: ResMut<SpaceCursor>, mut sel_changed: EventWriter<SelectionChanged>) {
    sel.snap = !sel.snap;
    sel_changed.send(SelectionChanged);
    // TODO: Should the grid offset snap into place when toggling snap? Right now it de-snaps again
    // when snap is disabled.
}

const SEL_DIST_LIMIT: f32 = 64.0;

fn move_grid_origin_to_camera(
    q_camera: Query<&GlobalTransform, (With<Camera>, Changed<GlobalTransform>)>,
    mut sel: ResMut<SpaceCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
) {
    if let Ok(camera_transform) = q_camera.get_single() {
        sel.origin = camera_transform.translation().round();
        let cur_offs = sel.axis_offset;
        match sel.axis {
            SpaceAxis::X => {
                sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().x, sel.max_pos().x)
            }
            SpaceAxis::Y => {
                sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().y, sel.max_pos().y)
            }
            SpaceAxis::Z => {
                sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().z, sel.max_pos().z)
            }
        }
        if sel.axis_offset != cur_offs {
            sel_changed.send(SelectionChanged);
        }
    }
}

fn select_normal(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut sel: ResMut<SpaceCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
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
                    sel_changed.send(SelectionChanged);
                }
            }
        }
    }
}

fn select_locked(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    locked_axis: Res<State<LockedAxis>>,
    mut sel: ResMut<SpaceCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
) {
    let window = q_window.single();

    let axis = locked_axis.get_axis();

    if let Some(mouse_pos) = window.cursor_position() {
        let (cam, cam_trans) = q_camera.single();

        if let Ok(ray) = cam.viewport_to_world(cam_trans, mouse_pos) {
            let mut towards_cam = sel.position - cam_trans.translation();
            match axis {
                SpaceAxis::X => towards_cam.x = 0.0,
                SpaceAxis::Y => towards_cam.y = 0.0,
                SpaceAxis::Z => towards_cam.z = 0.0,
            }
            towards_cam = towards_cam.normalize();
            let plane = InfinitePlane3d::new(towards_cam);

            if let Some(dist) = ray.intersect_plane(sel.position, plane) {
                let point = ray.get_point(dist);
                let point = if sel.snap { Vec3::round(point) } else { point };
                let point = sel.clamp_vec(point);

                match axis {
                    SpaceAxis::X => {
                        if axis == &sel.axis {
                            sel.axis_offset = point.x
                        };
                        sel.position.x = point.x;
                    }
                    SpaceAxis::Y => {
                        if axis == &sel.axis {
                            sel.axis_offset = point.y;
                        };
                        sel.position.y = point.y;
                    }
                    SpaceAxis::Z => {
                        if axis == &sel.axis {
                            sel.axis_offset = point.z;
                        };
                        sel.position.z = point.z;
                    }
                }
                sel_changed.send(SelectionChanged);
            }
        }
    }
}

fn find_entites_in_selection(
    sel: Res<SpaceCursor>,
    spatial_query: SpatialQuery,
    mut sel_target: ResMut<SelectionTargets>,
) {
    let inter = spatial_query.point_intersections(sel.position, &SpatialQueryFilter::default());

    if inter.is_empty() {
        sel_target.focused = None;
    } else if let Some(existing_prim) = sel_target.focused {
        if !inter.contains(&existing_prim) {
            sel_target.focused = Some(inter[0]);
        }
    } else {
        sel_target.focused = Some(inter[0]);
    }

    sel_target.intersecting = inter;
}

fn scroll_intersecting(num: i32) -> impl Fn(ResMut<SelectionTargets>) {
    move |mut sel_target| {
        if let Some(existing_prim) = sel_target.focused {
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

                sel_target.focused = Some(sel_target.intersecting[next as usize]);
            }
        }
    }
}

fn reset_axis_offset(mut sel: ResMut<SpaceCursor>, mut sel_changed: EventWriter<SelectionChanged>) {
    sel.axis_offset = 0.0;
    match sel.axis {
        SpaceAxis::X => sel.position.x = 0.0,
        SpaceAxis::Y => sel.position.y = 0.0,
        SpaceAxis::Z => sel.position.z = 0.0,
    };
    sel_changed.send(SelectionChanged);
}

#[derive(Resource, Deref)]
pub struct SelTargetBrushSide(pub Facing3d);

fn sel_brush_test(
    sel: Res<SpaceCursor>,
    sel_target: Res<SelectionTargets>,
    sel_brush_target_side: Option<Res<SelTargetBrushSide>>,
    brushes: Query<&Brush>,
    mut gizmos: Gizmos<SelHighlightGizmos>,
    mut commands: Commands,
) {
    if let Some(target) = sel_target.focused {
        if let Ok(brush) = brushes.get(target) {
            let closest_side = brush
                .bounds
                .sides_world()
                .sorted_by(|side_a, side_b| {
                    sel.position
                        .distance(side_a.pos)
                        .total_cmp(&sel.position.distance(side_b.pos))
                })
                .next()
                .unwrap();

            gizmos.rect(
                Isometry3d::new(
                    closest_side.pos,
                    Quat::from_rotation_arc(
                        Vec3::Y,
                        closest_side.plane.normal.any_orthonormal_vector(),
                    ),
                ),
                closest_side.plane.half_size * 2.,
                css::INDIAN_RED,
            );

            if sel_brush_target_side.is_none_or(|side| side.0 != closest_side.facing) {
                commands.insert_resource(SelTargetBrushSide(closest_side.facing));
            }
        }
    } else if sel_brush_target_side.is_some() {
        commands.remove_resource::<SelTargetBrushSide>();
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SpaceCursor>()
        .init_resource::<SelectionTargets>()
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
                depth_bias: -0.001,
                ..default()
            },
        )
        .insert_gizmo_config(
            SelTargetGizmos {},
            GizmoConfig {
                line_width: 4.0,
                depth_bias: -0.999,
                ..default()
            },
        )
        .insert_gizmo_config(
            SelHighlightGizmos {},
            GizmoConfig {
                line_width: 6.0,
                depth_bias: -1.0,
                ..default()
            },
        )
        .add_event::<SelectionChanged>()
        .init_state::<SelMode>()
        .add_computed_state::<SelIsAxisLocked>()
        .add_computed_state::<LockedAxis>()
        .add_systems(
            PreUpdate,
            (
                // Switching selectin axis (what axis is da grid on)
                (
                    switch_sel_axis(SpaceAxis::X).run_if(input_just_pressed(Binding::SetSelAxisX)),
                    switch_sel_axis(SpaceAxis::Y).run_if(input_just_pressed(Binding::SetSelAxisY)),
                    switch_sel_axis(SpaceAxis::Z).run_if(input_just_pressed(Binding::SetSelAxisZ)),
                )
                    .run_if(in_state(SelMode::Normal)),
                // Axis locking (sel mode 2) and offset
                set_axis_lock(SpaceAxis::X).run_if(input_just_pressed(Binding::AxisLockX)),
                set_axis_lock(SpaceAxis::Y).run_if(input_just_pressed(Binding::AxisLockY)),
                set_axis_lock(SpaceAxis::Z).run_if(input_just_pressed(Binding::AxisLockZ)),
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
                find_entites_in_selection.run_if(on_event::<SelectionChanged>),
                (
                    draw_sel_grid_gizmos,
                    draw_axis_line_gizmos,
                    draw_sel_target_gizmos,
                    sel_brush_test,
                ),
            )
                .chain()
                .in_set(EditorSystems),
        );
}
