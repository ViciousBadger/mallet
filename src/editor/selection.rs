use avian3d::prelude::*;
use bevy::{color::palettes::css, input::common_conditions::input_just_pressed, prelude::*};
use itertools::Itertools;

use crate::{
    core::{
        binds::{Binding, InputBindingSystem},
        map::{brush::Brush, LiveMapNodeId, MapNodeId},
    },
    editor::{
        cursor::{CursorMode, SpatialAxis, SpatialCursor},
        freelook::FreelookState,
        EditorSystems,
    },
    util::Facing3d,
};

/// Exists when a position is selected via the spatial cursor.
#[derive(Resource, Deref, Clone, Copy)]
pub struct SelectedPos(pub Vec3);

pub trait SelectedPosOrDefault {
    fn or_default(&self) -> Vec3;
}

impl SelectedPosOrDefault for Option<Res<'_, SelectedPos>> {
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
/// or maybe.. cursor fires an event with Vec3, which is picked up in here,
/// and turned into a SelectedPos..? decoupling
#[derive(Event)]
pub struct SelectionChanged;

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
                    Quat::from_rotation_arc(Vec3::NEG_Z, *closest_side.facing.as_dir()),
                ),
                closest_side.size,
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

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelAxisGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelTargetGizmos {}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelHighlightGizmos {}

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

pub fn plugin(app: &mut App) {
    app.insert_gizmo_config(
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
    );
    app.add_event::<SelectionChanged>()
        .add_systems(
            PreUpdate,
            (
                // Selected targets
                (
                    scroll_intersecting(1).run_if(input_just_pressed(Binding::SelNext)),
                    scroll_intersecting(-1).run_if(input_just_pressed(Binding::SelPrev)),
                )
                    .run_if(resource_exists::<SelectionTargets>),
            )
                .after(InputBindingSystem)
                .run_if(in_state(FreelookState::Unlocked))
                .in_set(EditorSystems),
        )
        .add_systems(
            Update,
            (
                find_targets_at_selection.run_if(on_event::<SelectionChanged>),
                sel_brush_test.run_if(resource_exists::<SelectedPos>),
            )
                .chain()
                .in_set(EditorSystems),
        )
        .add_systems(
            PostUpdate,
            (
                draw_axis_line_gizmos.run_if(resource_exists::<SelectedPos>),
                draw_sel_target_gizmos.run_if(resource_exists::<SelectionTargets>),
            )
                .after(TransformSystem::TransformPropagate)
                .in_set(EditorSystems),
        );
}
