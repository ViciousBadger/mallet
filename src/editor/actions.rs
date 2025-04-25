use bevy::{
    color::palettes::css,
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
};

use crate::{
    core::{
        binds::InputBindingSystem,
        map::{
            brush::{Brush, BrushBounds},
            light::{Light, LightType},
            DeployMapNode, Map, MapChange, MapContext, MapNode,
        },
    },
    editor::selection::{SelMode, SpaceCursor},
    util::Facing3d,
};

use super::{
    selection::{SelTargetBrushSide, SelectionTargets},
    EditorSystems,
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
    pub brush_entity_id: Entity,
    pub side: Facing3d,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ActionGizmos {}

fn cancel_action(mut next_editor_action: ResMut<NextState<EditorAction>>) {
    next_editor_action.set(EditorAction::None);
}

// Action: Building a new brush

fn start_building_brush_here(
    sel: Res<SpaceCursor>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut commands: Commands,
) {
    next_editor_action.set(EditorAction::BuildBrush);
    commands.insert_resource(BuildBrushProcess {
        start: sel.position,
    });
}

fn end_building_brush_here(
    process: Res<BuildBrushProcess>,
    sel: Res<SpaceCursor>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut map_changes: EventWriter<MapChange>,
) {
    let start = process.start;
    let end = sel.position;
    let bounds = BrushBounds::new(start, end);

    if bounds.is_valid() {
        map_changes.send(MapChange::Add(MapNode::Brush(Brush { bounds })));
        next_editor_action.set(EditorAction::None);
    }
}

fn build_brush_draw_gizmos(
    process: Res<BuildBrushProcess>,
    sel: Res<SpaceCursor>,
    mut gizmos: Gizmos<ActionGizmos>,
) {
    let start = process.start;
    let end = sel.position;
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
        brush_entity_id: sel_target.focused.unwrap(),
        side: sel_target_brush_side.0,
    });
    info!("resize brush!");
}

fn live_brush_resize(
    sel: Res<SpaceCursor>,
    process: Res<ResizeBrushProcess>,
    q_brushes: Query<&Brush>,
    mut deploy_events: EventWriter<DeployMapNode>,
) {
    if let Ok(brush) = q_brushes.get(process.brush_entity_id) {
        let resized_brush = Brush {
            bounds: brush.bounds.resized(process.side, sel.position),
        };
        deploy_events.send(DeployMapNode {
            entity_id: process.brush_entity_id,
            node: MapNode::Brush(resized_brush),
        });
    } else {
        warn!(
            "trying to resize brush with entity that is not a brush: {}",
            process.brush_entity_id
        );
    }
}

fn end_resizing_brush_here(
    sel: Res<SpaceCursor>,
    process: Res<ResizeBrushProcess>,
    q_brushes: Query<&Brush>,
    map: Res<Map>,
    map_context: Res<MapContext>,
    mut mod_events: EventWriter<MapChange>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
) {
    if let Ok(brush) = q_brushes.get(process.brush_entity_id) {
        let resized_bounds = brush.bounds.resized(process.side, sel.position);
        let node_id = map_context
            .entity_to_node(&process.brush_entity_id)
            .unwrap();
        let MapNode::Brush(mut brush) = map.get_node(node_id).unwrap().clone() else {
            panic!("notabrush")
        };
        brush.bounds = resized_bounds;
        mod_events.send(MapChange::Modify(*node_id, MapNode::Brush(brush)));
        next_editor_action.set(EditorAction::None);
    } else {
        warn!(
            "trying to resize brush with entity that is not a brush: {}",
            process.brush_entity_id
        );
    }
}

fn resize_brush_cleanup(mut commands: Commands) {
    commands.remove_resource::<ResizeBrushProcess>();
}

fn any_action_cleanup(mut next_sel_mode: ResMut<NextState<SelMode>>) {
    // next_sel_mode.set(SelMode::Normal);
}

fn remove_node(
    sel_target: Res<SelectionTargets>,
    map_context: Res<MapContext>,
    mut mod_events: EventWriter<MapChange>,
) {
    if let Some(entity) = sel_target.focused {
        let node_id = map_context.entity_to_node(&entity).unwrap();
        mod_events.send(MapChange::Remove(*node_id));
    }
}

fn add_light(
    sel: Res<SpaceCursor>,
    map_context: Res<MapContext>,
    mut mod_events: EventWriter<MapChange>,
) {
    let light = Light {
        position: sel.position,
        light_type: LightType::Point,
        color: Color::Srgba(css::WHITE),
        intensity: 30000.0,
        range: 20.0,
    };
    mod_events.send(MapChange::Add(MapNode::Light(light)));
}

pub fn plugin(app: &mut App) {
    app.init_state::<EditorAction>()
        .insert_gizmo_config(
            ActionGizmos {},
            GizmoConfig {
                line_width: 3.0,
                line_style: GizmoLineStyle::Dotted,
                depth_bias: -0.015,
                ..default()
            },
        )
        .add_systems(
            PreUpdate,
            (
                (
                    start_building_brush_here.run_if(
                        not(resource_exists::<SelTargetBrushSide>)
                            .and(input_just_pressed(MouseButton::Left)),
                    ),
                    start_resizing_brush.run_if(
                        resource_exists::<SelTargetBrushSide>
                            .and(input_just_pressed(MouseButton::Left)),
                    ),
                    remove_node.run_if(input_just_pressed(KeyCode::Delete)),
                    add_light.run_if(input_just_pressed(KeyCode::KeyI)),
                )
                    .run_if(in_state(EditorAction::None)),
                (
                    build_brush_draw_gizmos,
                    end_building_brush_here.run_if(input_just_pressed(MouseButton::Left)),
                )
                    .run_if(in_state(EditorAction::BuildBrush)),
                (
                    live_brush_resize,
                    end_resizing_brush_here.run_if(input_just_released(MouseButton::Left)),
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
        .add_systems(OnExit(EditorAction::ResizeBrush), resize_brush_cleanup)
        .add_systems(OnEnter(EditorAction::None), any_action_cleanup);
}
