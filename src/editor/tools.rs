use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CurrentTool {
    #[default]
    Select,
    BuildBrush,
    ResizeBrush,
    SetSurface,
    AddLight,
}

pub fn plugin(app: &mut App) {
    app.init_state::<CurrentTool>();
}
