use bevy::{color::palettes::css, input::common_conditions::input_just_pressed, prelude::*};

use crate::selection::{Sel, SelMode};

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

pub struct BrushBounds {
    start: Vec3,
    end: Vec3,
}

impl BrushBounds {
    pub fn new(point_a: Vec3, point_b: Vec3) -> Self {
        Self {
            start: Vec3::min(point_a, point_b),
            end: Vec3::max(point_a, point_b),
        }
    }

    pub fn is_valid(&self) -> bool {
        let size = self.size();
        size.x > 0.01 && size.y > 0.01 && size.z > 0.01
    }

    pub fn center(&self) -> Vec3 {
        (self.start + self.end) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.end - self.start
    }

    pub fn sides(&self) -> Vec<BrushSide> {
        let center = self.center();
        let size = self.size();
        let half_size = size / 2.0;
        vec![
            // X-
            BrushSide {
                pos: Vec3::new(self.start.x, center.y, center.z),
                plane: Plane3d::new(Vec3::NEG_X, Vec2::new(half_size.y, half_size.z)),
            },
            // X+
            BrushSide {
                pos: Vec3::new(self.end.x, center.y, center.z),
                plane: Plane3d::new(Vec3::X, Vec2::new(half_size.y, half_size.z)),
            },
            // Z-
            BrushSide {
                pos: Vec3::new(center.x, center.y, self.start.z),
                plane: Plane3d::new(Vec3::NEG_Z, Vec2::new(half_size.x, half_size.y)),
            },
            // Z+
            BrushSide {
                pos: Vec3::new(center.x, center.y, self.end.z),
                plane: Plane3d::new(Vec3::Z, Vec2::new(half_size.x, half_size.y)),
            },
            // Y-
            BrushSide {
                pos: Vec3::new(center.x, self.start.y, center.z),
                plane: Plane3d::new(Vec3::NEG_Y, Vec2::new(half_size.x, half_size.z)),
            },
            // Y+
            BrushSide {
                pos: Vec3::new(center.x, self.end.y, center.z),
                plane: Plane3d::new(Vec3::Y, Vec2::new(half_size.x, half_size.z)),
            },
        ]
    }
}

pub struct BrushSide {
    pos: Vec3,
    plane: Plane3d,
}

pub fn plugin(app: &mut App) {
    app.init_state::<EditorAction>()
        .add_systems(
            Update,
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
            ),
        )
        .add_systems(OnExit(EditorAction::BuildBrush), build_brush_cleanup)
        .add_systems(OnEnter(EditorAction::None), any_action_cleanup);
}

pub fn cancel_action(mut next_editor_action: ResMut<NextState<EditorAction>>) {
    next_editor_action.set(EditorAction::None);
}

pub fn start_building_brush_here(
    sel: Res<Sel>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut commands: Commands,
) {
    next_editor_action.set(EditorAction::BuildBrush);
    commands.insert_resource(BuildBrushProcess {
        start: sel.position,
    });
}

pub fn end_building_brush_here(
    process: Res<BuildBrushProcess>,
    sel: Res<Sel>,
    mut next_editor_action: ResMut<NextState<EditorAction>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let start = process.start;
    let end = sel.position;
    let bounds = BrushBounds::new(start, end);

    if bounds.is_valid() {
        // do the thing
        for side in bounds.sides() {
            commands.spawn((
                Transform::IDENTITY.with_translation(side.pos),
                Mesh3d(meshes.add(side.plane.mesh())),
                MeshMaterial3d(materials.add(Color::Srgba(css::PERU))),
            ));
        }
        next_editor_action.set(EditorAction::None);
    }
}

pub fn build_brush_draw_gizmos(process: Res<BuildBrushProcess>, sel: Res<Sel>, mut gizmos: Gizmos) {
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

pub fn build_brush_cleanup(mut commands: Commands) {
    commands.remove_resource::<BuildBrushProcess>();
}

pub fn any_action_cleanup(mut next_sel_mode: ResMut<NextState<SelMode>>) {
    next_sel_mode.set(SelMode::Normal);
}
