use avian3d::prelude::{AnyCollider, Collider};
use bevy::{color::palettes::css, prelude::*};

use super::{
    CursorMode, SelectedPos, SelectionTargets, SpatialAxis, SpatialCursor, SEL_DIST_LIMIT,
};

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelGridGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelAxisGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelTargetGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelHighlightGizmos {}

pub fn install_gizmos(app: &mut App) -> &mut App {
    app.insert_gizmo_config(
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
}

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

pub fn draw_axis_line_gizmos(
    cursor: Res<SpatialCursor>,
    sel_pos: Res<SelectedPos>,
    mut axis_gizmos: Gizmos<SelAxisGizmos>,
) {
    let sel_color = css::GOLD;
    let min = cursor.min_pos();
    let max = cursor.max_pos();

    // X axis
    axis_gizmos.line(
        Vec3::new(min.x, sel_pos.y, sel_pos.z),
        Vec3::new(max.x, sel_pos.y, sel_pos.z),
        if matches!(
            cursor.mode,
            CursorMode::AxisLocked {
                axis: SpatialAxis::X,
                ..
            }
        ) {
            sel_color
        } else {
            css::BLUE_VIOLET.with_alpha(0.3)
        },
    );

    // Y axis
    axis_gizmos.line(
        Vec3::new(sel_pos.x, min.y, sel_pos.z),
        Vec3::new(sel_pos.x, max.y, sel_pos.z),
        if matches!(
            cursor.mode,
            CursorMode::AxisLocked {
                axis: SpatialAxis::Y,
                ..
            }
        ) {
            sel_color
        } else {
            css::INDIAN_RED.with_alpha(0.3)
        },
    );

    // Z axis
    axis_gizmos.line(
        Vec3::new(sel_pos.x, sel_pos.y, min.z),
        Vec3::new(sel_pos.x, sel_pos.y, max.z),
        if matches!(
            cursor.mode,
            CursorMode::AxisLocked {
                axis: SpatialAxis::Z,
                ..
            }
        ) {
            sel_color
        } else {
            css::SPRING_GREEN.with_alpha(0.3)
        },
    );
}

pub fn draw_sel_target_gizmos(
    sel_target: Res<SelectionTargets>,
    q_colliders: Query<(&Collider, &GlobalTransform)>,
    mut gizmos: Gizmos<SelTargetGizmos>,
) {
    for intersecting_node in sel_target.intersecting.iter() {
        if let Ok((coll, coll_transform)) = q_colliders.get(intersecting_node.entity) {
            let aabb = coll.aabb(coll_transform.translation(), coll_transform.rotation());
            //.grow(Vec3::ONE * 0.01);
            //
            let col = if &sel_target.focused == intersecting_node {
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
