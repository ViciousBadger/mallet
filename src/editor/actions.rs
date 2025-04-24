use bevy::{color::palettes::css, input::common_conditions::input_just_pressed, prelude::*};

use crate::{
    core::{
        input_binding::InputBindingSystem,
        map::{
            brush::{Brush, BrushBounds},
            CreateNewMapNode, MMapNodeKind,
        },
    },
    editor::selection::{Sel, SelMode},
};

use super::EditorSystems;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorAction {
    #[default]
    None,
    BuildBrush,
}

#[derive(Resource)]
pub struct BuildBrushProcess {
    pub start: Vec3,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ActionGizmos {}

fn cancel_action(mut next_editor_action: ResMut<NextState<EditorAction>>) {
    next_editor_action.set(EditorAction::None);
}

fn start_building_brush_here(
    sel: Res<Sel>,
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
    sel: Res<Sel>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut new_map_node_events: EventWriter<CreateNewMapNode>,
) {
    let start = process.start;
    let end = sel.position;
    let bounds = BrushBounds::new(start, end);

    if bounds.is_valid() {
        // do the thing
        new_map_node_events.send(CreateNewMapNode(MMapNodeKind::Brush(Brush { bounds })));
        next_editor_action.set(EditorAction::None);
    }
}

fn build_brush_draw_gizmos(
    process: Res<BuildBrushProcess>,
    sel: Res<Sel>,
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

fn any_action_cleanup(mut next_sel_mode: ResMut<NextState<SelMode>>) {
    // next_sel_mode.set(SelMode::Normal);
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
                (start_building_brush_here.run_if(input_just_pressed(MouseButton::Left)),)
                    .run_if(in_state(EditorAction::None)),
                (
                    build_brush_draw_gizmos,
                    end_building_brush_here.run_if(input_just_pressed(MouseButton::Left)),
                )
                    .run_if(in_state(EditorAction::BuildBrush)),
                cancel_action.run_if(
                    not(in_state(EditorAction::None)).and(input_just_pressed(KeyCode::Escape)),
                ),
            )
                .after(InputBindingSystem)
                .in_set(EditorSystems),
        )
        .add_systems(OnExit(EditorAction::BuildBrush), build_brush_cleanup)
        .add_systems(OnEnter(EditorAction::None), any_action_cleanup);
}
