mod visuals;

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
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
use serde::{Deserialize, Serialize};
use visuals::{
    draw_axis_line_gizmos, draw_sel_grid_gizmos, draw_sel_target_gizmos, install_gizmos,
    SelHighlightGizmos,
};

use crate::{
    core::{
        binds::{Binding, InputBindingSystem},
        map::{brush::Brush, LiveMapNodeId, MapNodeId},
    },
    editor::freelook::FreelookState,
    util::{input_just_toggled, Facing3d},
};

use super::EditorSystems;

const SEL_DIST_LIMIT: f32 = 64.0;

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpatialAxis {
    X,
    #[default]
    Y,
    Z,
}

impl SpatialAxis {
    pub fn as_unit_vec(&self) -> Vec3 {
        match self {
            SpatialAxis::X => Vec3::X,
            SpatialAxis::Y => Vec3::Y,
            SpatialAxis::Z => Vec3::Z,
        }
    }

    pub fn as_plane(&self) -> InfinitePlane3d {
        InfinitePlane3d::new(Dir3::new_unchecked(self.as_unit_vec()))
    }
}

#[derive(Serialize, Deserialize, Clone, Resource, Default)]
pub struct SpatialCursor {
    pub axis: SpatialAxis,
    pub axis_offset: f32,
    pub snap: bool,
    pub origin: Vec3,
}

impl SpatialCursor {
    pub fn grid_center(&self) -> Vec3 {
        let axis_offs_aligned = if self.snap {
            self.axis_offset.round()
        } else {
            self.axis_offset
        };
        match self.axis {
            SpatialAxis::X => Vec3::new(axis_offs_aligned, self.origin.y, self.origin.z),
            SpatialAxis::Y => Vec3::new(self.origin.x, axis_offs_aligned, self.origin.z),
            SpatialAxis::Z => Vec3::new(self.origin.x, self.origin.y, axis_offs_aligned),
        }
    }

    pub fn min_pos(&self) -> Vec3 {
        self.origin - Vec3::ONE * SEL_DIST_LIMIT
    }

    pub fn max_pos(&self) -> Vec3 {
        self.origin + Vec3::ONE * SEL_DIST_LIMIT
    }

    /// Returns the point snapped to the cursor's snapping grid.
    pub fn snapped(&self, point: Vec3) -> Vec3 {
        if self.snap {
            Vec3::round(point)
        } else {
            point
        }
    }

    /// Returns the point if within cursor bounds, None if outside.
    pub fn bounds_checked(&self, point: Vec3) -> Option<Vec3> {
        (point.cmpgt(self.min_pos()) == BVec3::TRUE && point.cmplt(self.max_pos()) == BVec3::TRUE)
            .then_some(point)
    }
}

/// Exists when a position is selected via the spatial cursor.
#[derive(Resource, Deref, Clone, Copy)]
pub struct SelectedPos(pub Vec3);

pub trait SelectedPosGet {
    fn or_default(&self) -> Vec3;
}

impl SelectedPosGet for Option<Res<'_, SelectedPos>> {
    fn or_default(&self) -> Vec3 {
        self.as_ref().map(|res| res.0).unwrap_or(Vec3::ZERO)
    }
}

/// Exists when one or more map nodes are intersecting the selected position.
#[derive(Resource)]
pub struct SelectionTargets {
    pub intersecting: Vec<LiveMapNodeId>,
    pub focused: LiveMapNodeId,
}

/// Exists when a map node has been explicitly selected.
// TODO: Enum for multi-selection?
#[derive(Resource, Deref)]
pub struct SelectedNode(pub LiveMapNodeId);

/// Fired when selected position changes
/// TODO: MOre events for different happenings? Store related data in event?
#[derive(Event)]
pub struct SelectionChanged;

#[derive(States, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SpatialCursorMode {
    #[default]
    Normal,
    AxisLocked(SpatialAxis),
}

impl SpatialCursorMode {
    pub fn is_axis_locked(&self) -> bool {
        match self {
            SpatialCursorMode::Normal => false,
            SpatialCursorMode::AxisLocked(_) => true,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct SelIsAxisLocked;
impl ComputedStates for SelIsAxisLocked {
    type SourceStates = SpatialCursorMode;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        sources.is_axis_locked().then_some(SelIsAxisLocked)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct LockedAxis(SpatialAxis);
impl ComputedStates for LockedAxis {
    type SourceStates = SpatialCursorMode;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            SpatialCursorMode::Normal => None,
            SpatialCursorMode::AxisLocked(sel_axis) => Some(LockedAxis(sel_axis.clone())),
        }
    }
}

impl LockedAxis {
    pub fn get_axis(&self) -> &SpatialAxis {
        &self.0
    }
}

fn switch_sel_axis(
    new_axis: SpatialAxis,
) -> impl Fn(ResMut<SpatialCursor>, Option<Res<SelectedPos>>) {
    move |mut cursor, sel_pos| {
        let sel_pos = sel_pos.or_default();
        cursor.axis_offset = match new_axis {
            SpatialAxis::X => sel_pos.x,
            SpatialAxis::Y => sel_pos.y,
            SpatialAxis::Z => sel_pos.z,
        };
        cursor.axis = new_axis.clone();
    }
}

fn set_axis_lock(
    axis: SpatialAxis,
) -> impl Fn(Res<State<SpatialCursorMode>>, ResMut<NextState<SpatialCursorMode>>) {
    move |cur_sel_mode, mut next_sel_mode| {
        if cur_sel_mode.get() != &SpatialCursorMode::AxisLocked(axis.clone()) {
            next_sel_mode.set(SpatialCursorMode::AxisLocked(axis.clone()));
        } else {
            next_sel_mode.set(SpatialCursorMode::Normal);
        }
    }
}

fn reset_sel_mode(mut next_sel_mode: ResMut<NextState<SpatialCursorMode>>) {
    next_sel_mode.set(SpatialCursorMode::default())
}

fn set_axis_lock_selected(
    sel: Res<SpatialCursor>,
    mut next_sel_mode: ResMut<NextState<SpatialCursorMode>>,
) {
    next_sel_mode.set(SpatialCursorMode::AxisLocked(sel.axis.clone()));
}

fn toggle_snap(mut sel: ResMut<SpatialCursor>, mut sel_changed: EventWriter<SelectionChanged>) {
    sel.snap = !sel.snap;
    sel_changed.send(SelectionChanged);
    // TODO: Should the grid offset snap into place when toggling snap? Right now it de-snaps again
    // when snap is disabled.
}

fn move_grid_origin_to_camera(
    q_camera: Query<&GlobalTransform, (With<Camera>, Changed<GlobalTransform>)>,
    mut sel: ResMut<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
) {
    if let Ok(camera_transform) = q_camera.get_single() {
        sel.origin = camera_transform.translation().round();
        let cur_offs = sel.axis_offset;
        match sel.axis {
            SpatialAxis::X => {
                sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().x, sel.max_pos().x)
            }
            SpatialAxis::Y => {
                sel.axis_offset = sel.axis_offset.clamp(sel.min_pos().y, sel.max_pos().y)
            }
            SpatialAxis::Z => {
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
    cursor: Res<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let window = q_window.single();

    let setpos = || {
        let mouse_pos = window.cursor_position()?;
        let (cam, cam_trans) = q_camera.get_single().ok()?;
        let ray = cam.viewport_to_world(cam_trans, mouse_pos).ok()?;
        let plane = cursor.axis.as_plane();
        let dist = ray.intersect_plane(cursor.grid_center(), plane)?;
        cursor.bounds_checked(cursor.snapped(ray.get_point(dist)))
    };

    if let Some(pos) = setpos() {
        commands.insert_resource(SelectedPos(pos));
    } else {
        commands.remove_resource::<SelectedPos>();
    }
    sel_changed.send(SelectionChanged);
}

fn select_locked(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    locked_axis: Res<State<LockedAxis>>,
    selected_pos: Option<Res<SelectedPos>>,
    mut cursor: ResMut<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let axis = locked_axis.get_axis();
    let lockpos = selected_pos.or_default();

    let setpos = || {
        let window = q_window.get_single().ok()?;
        let mouse_pos = window.cursor_position()?;
        let (cam, cam_trans) = q_camera.get_single().ok()?;
        let ray = cam.viewport_to_world(cam_trans, mouse_pos).ok()?;
        let mut towards_cam = lockpos - cam_trans.translation();
        match axis {
            SpatialAxis::X => towards_cam.x = 0.0,
            SpatialAxis::Y => towards_cam.y = 0.0,
            SpatialAxis::Z => towards_cam.z = 0.0,
        }
        towards_cam = towards_cam.normalize();
        let plane = InfinitePlane3d::new(towards_cam);

        let dist = ray.intersect_plane(lockpos, plane)?;
        cursor.bounds_checked(cursor.snapped(ray.get_point(dist)))
    };

    if let Some(pos) = setpos() {
        match axis {
            SpatialAxis::X => {
                if axis == &cursor.axis {
                    cursor.axis_offset = pos.x
                };
                commands.insert_resource(SelectedPos(lockpos.with_x(pos.x)));
            }
            SpatialAxis::Y => {
                if axis == &cursor.axis {
                    cursor.axis_offset = pos.y;
                };
                commands.insert_resource(SelectedPos(lockpos.with_y(pos.y)));
            }
            SpatialAxis::Z => {
                if axis == &cursor.axis {
                    cursor.axis_offset = pos.z;
                };
                commands.insert_resource(SelectedPos(lockpos.with_z(pos.z)));
            }
        }
    } else {
        commands.remove_resource::<SelectedPos>();
    }
    sel_changed.send(SelectionChanged);
}

fn find_targets_at_selection(
    sel_pos: Option<Res<SelectedPos>>,
    spatial_query: SpatialQuery,
    q_map_nodes: Query<&MapNodeId>,
    sel_targets: Option<ResMut<SelectionTargets>>,
    mut commands: Commands,
) {
    if let Some(sel_pos) = sel_pos {
        let intersecting_nodes = spatial_query
            .point_intersections(**sel_pos, &SpatialQueryFilter::default())
            .iter()
            .filter_map(|entity_id| {
                q_map_nodes
                    .get(*entity_id)
                    .ok()
                    .map(|node_id| LiveMapNodeId {
                        node_id: *node_id,
                        entity: *entity_id,
                    })
            })
            .collect_vec();

        if intersecting_nodes.is_empty() {
            commands.remove_resource::<SelectionTargets>();
        } else if let Some(mut existing_targets) = sel_targets {
            // Check if the focused target is still intersecting and switch focus if not.
            if !intersecting_nodes.contains(&existing_targets.focused) {
                existing_targets.focused = intersecting_nodes[0];
            }
            // Update intersecting targets vec.
            existing_targets.intersecting = intersecting_nodes;
        } else {
            commands.insert_resource(SelectionTargets {
                focused: intersecting_nodes[0],
                intersecting: intersecting_nodes,
            });
        }
    } else {
        commands.remove_resource::<SelectionTargets>();
    }
}

fn scroll_intersecting(num: i32) -> impl Fn(ResMut<SelectionTargets>) {
    move |mut sel_target| {
        if let Some(idx) = sel_target
            .intersecting
            .iter()
            .position(|n| n == &sel_target.focused)
        {
            let len = sel_target.intersecting.len() as i32;
            let mut next = idx as i32 + num;
            if next >= len {
                next = 0;
            }
            if next < 0 {
                next = len - 1;
            }

            sel_target.focused = sel_target.intersecting[next as usize];
        }
    }
}

fn reset_axis_offset(
    sel_pos: Option<Res<SelectedPos>>,
    mut cursor: ResMut<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let selpos = sel_pos.or_default();

    cursor.axis_offset = 0.0;
    match cursor.axis {
        SpatialAxis::X => commands.insert_resource(SelectedPos(selpos.with_x(0.0))),
        SpatialAxis::Y => commands.insert_resource(SelectedPos(selpos.with_y(0.0))),
        SpatialAxis::Z => commands.insert_resource(SelectedPos(selpos.with_z(0.0))),
    };
    sel_changed.send(SelectionChanged);
}

#[derive(Resource, Deref)]
pub struct SelTargetBrushSide(pub Facing3d);

fn sel_brush_test(
    sel_pos: Res<SelectedPos>,
    sel_targets: Option<Res<SelectionTargets>>,
    sel_brush_target_side: Option<Res<SelTargetBrushSide>>,
    brushes: Query<&Brush>,
    mut gizmos: Gizmos<SelHighlightGizmos>,
    mut commands: Commands,
) {
    if let Some(sel_targets) = sel_targets {
        if let Ok(brush) = brushes.get(sel_targets.focused.entity) {
            let closest_side = brush
                .bounds
                .sides_world()
                .sorted_by(|side_a, side_b| {
                    sel_pos
                        .distance(side_a.pos)
                        .total_cmp(&sel_pos.distance(side_b.pos))
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
    install_gizmos(app)
        .init_resource::<SpatialCursor>()
        .add_event::<SelectionChanged>()
        .init_state::<SpatialCursorMode>()
        .add_computed_state::<SelIsAxisLocked>()
        .add_computed_state::<LockedAxis>()
        .add_systems(
            PreUpdate,
            (
                // Switching selectin axis (what axis is da grid on)
                (
                    switch_sel_axis(SpatialAxis::X)
                        .run_if(input_just_pressed(Binding::SetSelAxisX)),
                    switch_sel_axis(SpatialAxis::Y)
                        .run_if(input_just_pressed(Binding::SetSelAxisY)),
                    switch_sel_axis(SpatialAxis::Z)
                        .run_if(input_just_pressed(Binding::SetSelAxisZ)),
                )
                    .run_if(in_state(SpatialCursorMode::Normal)),
                // Axis locking (sel mode 2) and offset
                set_axis_lock(SpatialAxis::X).run_if(input_just_pressed(Binding::AxisLockX)),
                set_axis_lock(SpatialAxis::Y).run_if(input_just_pressed(Binding::AxisLockY)),
                set_axis_lock(SpatialAxis::Z).run_if(input_just_pressed(Binding::AxisLockZ)),
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
                (
                    scroll_intersecting(1).run_if(input_just_pressed(Binding::SelNext)),
                    scroll_intersecting(-1).run_if(input_just_pressed(Binding::SelPrev)),
                )
                    .run_if(resource_exists::<SelectionTargets>),
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
                    select_normal
                        .run_if(in_state(SpatialCursorMode::Normal).and(on_event::<MouseMotion>)),
                    select_locked.run_if(in_state(SelIsAxisLocked).and(on_event::<MouseMotion>)),
                )
                    .run_if(in_state(FreelookState::Unlocked)),
                find_targets_at_selection.run_if(on_event::<SelectionChanged>),
                sel_brush_test.run_if(resource_exists::<SelectedPos>),
            )
                .chain()
                .in_set(EditorSystems),
        )
        .add_systems(
            PostUpdate,
            (
                draw_sel_grid_gizmos,
                draw_axis_line_gizmos.run_if(resource_exists::<SelectedPos>),
                draw_sel_target_gizmos.run_if(resource_exists::<SelectionTargets>),
            )
                .after(TransformSystem::TransformPropagate)
                .in_set(EditorSystems),
        );
}
