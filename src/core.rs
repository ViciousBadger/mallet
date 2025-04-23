pub mod app_data;
pub mod input_binding;
pub mod map;
pub mod view;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        app_data::plugin,
        input_binding::plugin,
        view::plugin,
        map::plugin,
    ));
}
