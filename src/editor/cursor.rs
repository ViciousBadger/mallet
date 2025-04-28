use avian3d::prelude::*;
use bevy::{
    color::palettes::css,
    input::{common_conditions::input_just_pressed, mouse::MouseMotion},
    prelude::*,
    window::PrimaryWindow,
};
use serde::{Deserialize, Serialize};

use crate::{
    core::binds::{Binding, InputBindingSystem},
    editor::{
        freelook::FreelookState,
        selection::{SelectedPos, SelectedPosOrDefault, SelectionChanged},
        EditorSystems,
    },
    util::input_just_toggled,
};

// TODO: rename to CURSOR_DIST_LIMIT
const SEL_DIST_LIMIT: f32 = 64.0;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Resource, Serialize, Deserialize, Default, Debug, Clone)]
pub struct SpatialCursor {
    pub mode: CursorMode,
    pub snap: bool,
    pub origin: Vec3,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum CursorModeKind {
    #[default]
    Pick,
    ViewPlane,
    AxisPlane(SpatialAxis),
    AxisLocked(SpatialAxis),
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum CursorMode {
    #[default]
    Pick,
    ViewPlane {
        dist: f32,
    },
    AxisPlane {
        axis: SpatialAxis,
        offset: f32,
    },
    AxisLocked {
        axis: SpatialAxis,
        origin: Vec3,
    },
}

impl SpatialCursor {
    // pub fn grid_center(&self) -> Vec3 {
    //     if let Some(axis) = self.axis() {
    //         let axis_offs_aligned = if self.snap {
    //             self.axis_offset.round()
    //         } else {
    //             self.axis_offset
    //         };
    //         match axis {
    //             SpatialAxis::X => Vec3::new(axis_offs_aligned, self.origin.y, self.origin.z),
    //             SpatialAxis::Y => Vec3::new(self.origin.x, axis_offs_aligned, self.origin.z),
    //             SpatialAxis::Z => Vec3::new(self.origin.x, self.origin.y, axis_offs_aligned),
    //         }
    //     } else {
    //         Vec3::ZERO
    //     }
    // }

    pub fn axis(&self) -> Option<SpatialAxis> {
        match &self.mode {
            CursorMode::Pick => None,
            CursorMode::ViewPlane { .. } => None,
            CursorMode::AxisPlane { axis, .. } => Some(*axis),
            CursorMode::AxisLocked { axis, .. } => Some(*axis),
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

#[allow(clippy::type_complexity)]
fn switch_cursor_mode(
    mode_kind: CursorModeKind,
) -> impl Fn(Option<Res<SelectedPos>>, Query<&GlobalTransform, With<Camera>>, ResMut<SpatialCursor>)
{
    move |sel_pos, q_camera, mut cursor| {
        cursor.mode = match &mode_kind {
            CursorModeKind::AxisPlane(axis) => {
                let offset = if let Some(sel_pos) = sel_pos {
                    match axis {
                        SpatialAxis::X => sel_pos.x,
                        SpatialAxis::Y => sel_pos.y,
                        SpatialAxis::Z => sel_pos.z,
                    }
                } else {
                    0.0
                };
                CursorMode::AxisPlane {
                    axis: *axis,
                    offset,
                }
            }
            CursorModeKind::AxisLocked(axis) => CursorMode::AxisLocked {
                axis: *axis,
                origin: sel_pos.or_default(),
            },
            CursorModeKind::ViewPlane => {
                let cam_transform = q_camera.single();
                CursorMode::ViewPlane {
                    dist: if let Some(sel_pos) = sel_pos {
                        cam_transform.translation().distance(**sel_pos)
                    } else {
                        10.0
                    },
                }
            }
            CursorModeKind::Pick => CursorMode::Pick,
        };
    }
}

fn toggle_snap(mut sel: ResMut<SpatialCursor>, mut sel_changed: EventWriter<SelectionChanged>) {
    sel.snap = !sel.snap;
    sel_changed.send(SelectionChanged);
    // TODO: Should the grid offset snap into place when toggling snap? Right now it de-snaps again
    // when snap is disabled.
}

fn update_cursor_origin(
    q_camera: Query<&GlobalTransform, (With<Camera>, Changed<GlobalTransform>)>,
    //cursor_axis: Option<Res<State<CursorAxis>>>,
    mut cursor: ResMut<SpatialCursor>,
    //mut cursor_changed: EventWriter<SelectionChanged>,
) {
    if let Ok(camera_transform) = q_camera.get_single() {
        cursor.origin = camera_transform.translation().round();
        // let cur_offs = cursor.axis_offset;
        // match cursor_axis.map(|res| res.to_owned().0) {
        //     Some(SpatialAxis::X) => {
        //         cursor.axis_offset = cursor
        //             .axis_offset
        //             .clamp(cursor.min_pos().x, cursor.max_pos().x);
        //     }
        //     Some(SpatialAxis::Y) => {
        //         cursor.axis_offset = cursor
        //             .axis_offset
        //             .clamp(cursor.min_pos().y, cursor.max_pos().y);
        //     }
        //     Some(SpatialAxis::Z) => {
        //         cursor.axis_offset = cursor
        //             .axis_offset
        //             .clamp(cursor.min_pos().z, cursor.max_pos().z);
        //     }
        //     None => (),
        // }
        // if cursor.axis_offset != cur_offs {
        //     cursor_changed.send(SelectionChanged);
        // }
    }
}

fn in_axis_plane_mode(cursor: Res<SpatialCursor>) -> bool {
    matches!(cursor.mode, CursorMode::AxisPlane { .. })
}

fn in_axis_locked_mode(cursor: Res<SpatialCursor>) -> bool {
    matches!(cursor.mode, CursorMode::AxisLocked { .. })
}

fn in_view_plane_mode(cursor: Res<SpatialCursor>) -> bool {
    matches!(cursor.mode, CursorMode::ViewPlane { .. })
}

fn in_pick_mode(cursor: Res<SpatialCursor>) -> bool {
    matches!(cursor.mode, CursorMode::Pick)
}

fn select_on_axis_plane(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    cursor: Res<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let window = q_window.single();
    let (axis, offset) = match cursor.mode {
        CursorMode::AxisPlane { axis, offset } => (axis, offset),
        _ => unreachable!(),
    };

    let setpos = || {
        let mouse_pos = window.cursor_position()?;
        let (cam, cam_trans) = q_camera.get_single().ok()?;
        let ray = cam.viewport_to_world(cam_trans, mouse_pos).ok()?;
        let plane = axis.as_plane();
        let grid_center = axis.as_unit_vec() * offset;
        let dist = ray.intersect_plane(grid_center, plane)?;
        cursor.bounds_checked(cursor.snapped(ray.get_point(dist)))
    };

    if let Some(pos) = setpos() {
        commands.insert_resource(SelectedPos(pos));
    } else {
        commands.remove_resource::<SelectedPos>();
    }
    sel_changed.send(SelectionChanged);
}

fn select_on_locked_axis(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    cursor: ResMut<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let (axis, origin) = match cursor.mode {
        CursorMode::AxisLocked { axis, origin } => (axis, origin),
        _ => unreachable!(),
    };

    let setpos = || {
        let window = q_window.get_single().ok()?;
        let mouse_pos = window.cursor_position()?;
        let (cam, cam_trans) = q_camera.get_single().ok()?;
        let ray = cam.viewport_to_world(cam_trans, mouse_pos).ok()?;
        let mut towards_cam = origin - cam_trans.translation();
        match axis {
            SpatialAxis::X => towards_cam.x = 0.0,
            SpatialAxis::Y => towards_cam.y = 0.0,
            SpatialAxis::Z => towards_cam.z = 0.0,
        }
        if towards_cam.length() > 0.0 {
            towards_cam = towards_cam.normalize();
            let plane = InfinitePlane3d::new(towards_cam);

            let dist = ray.intersect_plane(origin, plane)?;
            cursor.bounds_checked(cursor.snapped(ray.get_point(dist)))
        } else {
            None
        }
    };

    if let Some(pos) = setpos() {
        commands.insert_resource(SelectedPos(match axis {
            SpatialAxis::X => origin.with_x(pos.x),
            SpatialAxis::Y => origin.with_y(pos.y),
            SpatialAxis::Z => origin.with_z(pos.z),
        }));
    } else {
        commands.remove_resource::<SelectedPos>();
    }
    sel_changed.send(SelectionChanged);
}

fn select_on_view_plane(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    cursor: ResMut<SpatialCursor>,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let dist = match cursor.mode {
        CursorMode::ViewPlane { dist } => dist,
        _ => unreachable!(),
    };

    let setpos = || {
        let window = q_window.get_single().ok()?;
        let mouse_pos = window.cursor_position()?;
        let (cam, cam_trans) = q_camera.get_single().ok()?;
        let ray = cam.viewport_to_world(cam_trans, mouse_pos).ok()?;
        Some(cursor.snapped(cam_trans.translation() + ray.direction * dist))
    };

    if let Some(pos) = setpos() {
        commands.insert_resource(SelectedPos(pos));
    } else {
        commands.remove_resource::<SelectedPos>();
    }
    sel_changed.send(SelectionChanged);
}

fn select_by_picking(
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    cursor: ResMut<SpatialCursor>,
    spatial_query: SpatialQuery,
    mut sel_changed: EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    let setpos = || {
        let window = q_window.get_single().ok()?;
        let mouse_pos = window.cursor_position()?;
        let (cam, cam_trans) = q_camera.get_single().ok()?;
        let ray = cam.viewport_to_world(cam_trans, mouse_pos).ok()?;

        let hit = spatial_query.cast_ray(ray.origin, ray.direction, 1000.0, false, &default())?;
        Some(cursor.snapped(cam_trans.translation() + ray.direction * hit.distance))
    };

    if let Some(pos) = setpos() {
        commands.insert_resource(SelectedPos(pos));
    } else {
        commands.remove_resource::<SelectedPos>();
    }
    sel_changed.send(SelectionChanged);
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelGridGizmos {}

pub fn draw_axis_plane_grid(cursor: Res<SpatialCursor>, mut gizmos: Gizmos<SelGridGizmos>) {
    let grid_line_color = css::DIM_GRAY.with_alpha(0.33);

    let (axis, offset) = match cursor.mode {
        CursorMode::AxisPlane { axis, offset } => (axis, offset),
        _ => unreachable!(),
    };

    let grid_center = axis.as_unit_vec() * offset;

    let mut iso = axis.as_plane().isometry_from_xy(grid_center);
    iso.translation = grid_center.into();
    gizmos.grid(
        iso,
        UVec2::new(SEL_DIST_LIMIT as u32 * 2, SEL_DIST_LIMIT as u32 * 2),
        Vec2::ONE,
        grid_line_color,
    );

    // TODO: get axis in view-plane mode. either calc it in the cursor using origin or something else.
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SpatialCursor>();
    app.insert_gizmo_config(
        SelGridGizmos {},
        GizmoConfig {
            line_width: 1.5,
            ..default()
        },
    );
    app.add_systems(
        PreUpdate,
        (
            // Axis plane mode
            switch_cursor_mode(CursorModeKind::AxisPlane(SpatialAxis::X))
                .run_if(input_just_pressed(Binding::CursorModePlaneX)),
            switch_cursor_mode(CursorModeKind::AxisPlane(SpatialAxis::Y))
                .run_if(input_just_pressed(Binding::CursorModePlaneY)),
            switch_cursor_mode(CursorModeKind::AxisPlane(SpatialAxis::Z))
                .run_if(input_just_pressed(Binding::CursorModePlaneZ)),
            // Axis lock mode
            switch_cursor_mode(CursorModeKind::AxisLocked(SpatialAxis::X))
                .run_if(input_just_pressed(Binding::CursorModeLockX)),
            switch_cursor_mode(CursorModeKind::AxisLocked(SpatialAxis::Y))
                .run_if(input_just_pressed(Binding::CursorModeLockY)),
            switch_cursor_mode(CursorModeKind::AxisLocked(SpatialAxis::Z))
                .run_if(input_just_pressed(Binding::CursorModeLockZ)),
            // View modes
            switch_cursor_mode(CursorModeKind::Pick)
                .run_if(input_just_pressed(Binding::CursorModePick)),
            switch_cursor_mode(CursorModeKind::ViewPlane)
                .run_if(input_just_pressed(Binding::CursorModePlaneView)),
            // Snapping
            toggle_snap
                .run_if(input_just_pressed(KeyCode::KeyT).or(input_just_toggled(KeyCode::AltLeft))),
        )
            .after(InputBindingSystem)
            .run_if(in_state(FreelookState::Unlocked))
            .in_set(EditorSystems),
    );
    app.add_systems(
        Update,
        (
            update_cursor_origin,
            (
                select_on_axis_plane.run_if(in_axis_plane_mode),
                select_on_locked_axis.run_if(in_axis_locked_mode),
                select_by_picking.run_if(in_pick_mode),
                select_on_view_plane.run_if(in_view_plane_mode),
            )
                .run_if(in_state(FreelookState::Unlocked).and(on_event::<MouseMotion>)),
        ),
    );
    app.add_systems(
        PostUpdate,
        (draw_axis_plane_grid.run_if(in_axis_plane_mode),).in_set(EditorSystems),
    );
}
