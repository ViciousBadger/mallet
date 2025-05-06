use bevy::{
    color::palettes::css,
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
};

use crate::{
    core::{
        binds::{Binding, InputBindingSystem},
        map::{
            brush::{Brush, BrushBounds},
            light::{Light, LightType},
            DeployMapNode, LiveMapNodeId, Map, MapChange, MapNode,
        },
    },
    editor::{
        selection::{SelTargetBrushSide, SelectedPos, SelectionTargets},
        EditorSystems,
    },
    util::Facing3d,
};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorAction {
    #[default]
    None,
    BuildBrush,
    ResizeBrush,
}

#[derive(Resource)]
pub struct BuildBrushProcess {
    pub start: Vec3,
}

#[derive(Resource)]
pub struct ResizeBrushProcess {
    pub brush: LiveMapNodeId,
    pub side: Facing3d,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ActionGizmos {}

fn cancel_action(mut next_editor_action: ResMut<NextState<EditorAction>>) {
    next_editor_action.set(EditorAction::None);
}

// Action: Building a new brush

fn start_building_brush_here(
    sel_pos: Res<SelectedPos>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut commands: Commands,
) {
    next_editor_action.set(EditorAction::BuildBrush);
    commands.insert_resource(BuildBrushProcess { start: **sel_pos });
}

fn end_building_brush_here(
    process: Res<BuildBrushProcess>,
    sel_pos: Res<SelectedPos>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut map_changes: EventWriter<MapChange>,
) {
    let start = process.start;
    let end = **sel_pos;
    let bounds = BrushBounds::new(start, end);

    if bounds.is_valid() {
        map_changes.write(MapChange::Add(MapNode::Brush(Brush { bounds })));
        next_editor_action.set(EditorAction::None);
    }
}

fn build_brush_draw_gizmos(
    process: Res<BuildBrushProcess>,
    sel_pos: Res<SelectedPos>,
    mut gizmos: Gizmos<ActionGizmos>,
) {
    let start = process.start;
    let end = **sel_pos;
    let bounds = BrushBounds::new(start, end);

    let transform = Transform::IDENTITY
        .with_translation(bounds.center())
        .with_scale(bounds.size());

    let color = if bounds.is_valid() {
        css::SPRING_GREEN
    } else {
        css::DARK_RED
    };

    gizmos.cuboid(transform, color);
}

fn build_brush_cleanup(mut commands: Commands) {
    commands.remove_resource::<BuildBrushProcess>();
}

// Action: Resizing an existing brush

fn start_resizing_brush(
    sel_target: Res<SelectionTargets>,
    sel_target_brush_side: Res<SelTargetBrushSide>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut commands: Commands,
) {
    next_editor_action.set(EditorAction::ResizeBrush);
    commands.insert_resource(ResizeBrushProcess {
        brush: sel_target.focused,
        side: sel_target_brush_side.0,
    });
}

fn live_brush_resize(
    sel_pos: Res<SelectedPos>,
    process: Res<ResizeBrushProcess>,
    q_brushes: Query<&Brush>,
    mut deploy_events: EventWriter<DeployMapNode>,
) {
    let brush = q_brushes.get(process.brush.entity).unwrap();
    let resized_brush = Brush {
        bounds: brush.bounds.resized(process.side, **sel_pos),
    };
    deploy_events.write(DeployMapNode {
        target_entity: process.brush.entity,
        node: MapNode::Brush(resized_brush),
    });
}

fn end_resizing_brush_here(
    sel_pos: Res<SelectedPos>,
    process: Res<ResizeBrushProcess>,
    q_brushes: Query<&Brush>,
    map: Res<Map>,
    mut mod_events: EventWriter<MapChange>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
) {
    let brush = q_brushes.get(process.brush.entity).unwrap();
    let resized_bounds = brush.bounds.resized(process.side, **sel_pos);
    let MapNode::Brush(mut brush) = map.get_node(&process.brush.node_id).unwrap().clone() else {
        panic!("notabrush")
    };
    brush.bounds = resized_bounds;
    mod_events.write(MapChange::Modify(
        process.brush.node_id,
        MapNode::Brush(brush),
    ));
    next_editor_action.set(EditorAction::None);
}

fn resize_brush_cleanup(mut commands: Commands) {
    commands.remove_resource::<ResizeBrushProcess>();
}

fn remove_node(sel_target: Res<SelectionTargets>, mut mod_events: EventWriter<MapChange>) {
    mod_events.write(MapChange::Remove(sel_target.focused.node_id));
}

fn add_light(sel_pos: Res<SelectedPos>, mut mod_events: EventWriter<MapChange>) {
    let light = Light {
        position: **sel_pos,
        light_type: LightType::Point,
        color: Color::Srgba(css::WHITE),
        intensity: 30000.0,
        range: 20.0,
    };
    mod_events.write(MapChange::Add(MapNode::Light(light)));
}

pub fn plugin(app: &mut App) {
    app.init_state::<EditorAction>()
        .insert_gizmo_config(
            ActionGizmos {},
            GizmoConfig {
                line: GizmoLineConfig {
                    width: 3.0,
                    style: GizmoLineStyle::Dotted,
                    ..default()
                },
                depth_bias: -0.015,
                ..default()
            },
        )
        .add_systems(
            PreUpdate,
            (
                (
                    start_building_brush_here.run_if(
                        resource_exists::<SelectedPos>
                            .and(not(resource_exists::<SelTargetBrushSide>))
                            .and(input_just_pressed(Binding::Primary)),
                    ),
                    start_resizing_brush.run_if(
                        resource_exists::<SelectionTargets>
                            .and(resource_exists::<SelTargetBrushSide>)
                            .and(input_just_pressed(Binding::Primary)),
                    ),
                    remove_node.run_if(
                        input_just_pressed(KeyCode::Delete)
                            .and(resource_exists::<SelectionTargets>),
                    ),
                    add_light.run_if(
                        resource_exists::<SelectedPos>.and(input_just_pressed(KeyCode::KeyI)),
                    ),
                )
                    .run_if(in_state(EditorAction::None)),
                (
                    build_brush_draw_gizmos.run_if(resource_exists::<SelectedPos>),
                    end_building_brush_here.run_if(
                        resource_exists::<SelectedPos>.and(input_just_pressed(Binding::Primary)),
                    ),
                )
                    .run_if(in_state(EditorAction::BuildBrush)),
                (
                    live_brush_resize.run_if(resource_exists::<SelectedPos>),
                    end_resizing_brush_here.run_if(
                        resource_exists::<SelectedPos>.and(input_just_released(Binding::Primary)),
                    ),
                )
                    .run_if(in_state(EditorAction::ResizeBrush)),
                cancel_action.run_if(
                    not(in_state(EditorAction::None)).and(input_just_pressed(KeyCode::Escape)),
                ),
            )
                .after(InputBindingSystem)
                .in_set(EditorSystems),
        )
        .add_systems(OnExit(EditorAction::BuildBrush), build_brush_cleanup)
        .add_systems(OnExit(EditorAction::ResizeBrush), resize_brush_cleanup);
}
