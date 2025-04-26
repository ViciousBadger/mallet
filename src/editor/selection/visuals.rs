use avian3d::prelude::{AnyCollider, Collider};
use bevy::{color::palettes::css, prelude::*};

use super::{SelectionTargets, SpatialAxis, SpatialCursor, SpatialCursorMode, SEL_DIST_LIMIT};

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelGridGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelAxisGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelTargetGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelHighlightGizmos {}

pub fn draw_sel_grid_gizmos(sel: Res<SpatialCursor>, mut gizmos: Gizmos<SelGridGizmos>) {
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

pub fn draw_axis_line_gizmos(
    sel: Res<SpatialCursor>,
    sel_mode: Res<State<SpatialCursorMode>>,
    mut axis_gizmos: Gizmos<SelAxisGizmos>,
) {
    let sel_color = css::GOLD;
    let min = sel.min_pos();
    let max = sel.max_pos();

    // X axis
    axis_gizmos.line(
        Vec3::new(min.x, sel.position.y, sel.position.z),
        Vec3::new(max.x, sel.position.y, sel.position.z),
        if *sel_mode == SpatialCursorMode::AxisLocked(SpatialAxis::X) {
            sel_color
        } else {
            css::BLUE_VIOLET.with_alpha(0.3)
        },
    );

    // Y axis
    axis_gizmos.line(
        Vec3::new(sel.position.x, min.y, sel.position.z),
        Vec3::new(sel.position.x, max.y, sel.position.z),
        if *sel_mode == SpatialCursorMode::AxisLocked(SpatialAxis::Y) {
            sel_color
        } else {
            css::INDIAN_RED.with_alpha(0.3)
        },
    );

    // Z axis
    axis_gizmos.line(
        Vec3::new(sel.position.x, sel.position.y, min.z),
        Vec3::new(sel.position.x, sel.position.y, max.z),
        if *sel_mode == SpatialCursorMode::AxisLocked(SpatialAxis::Z) {
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
