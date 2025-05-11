use bevy::prelude::*;

use crate::app_data;

pub fn run_playground() {
    App::new()
        .add_plugins((app_data::plugin, DefaultPlugins, plugin))
        .run();
}

fn plugin(app: &mut App) {
    // app.add_plugins(map::plugin);
}
