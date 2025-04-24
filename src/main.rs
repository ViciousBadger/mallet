mod app_data;
mod core;
mod editor;
mod game;
mod util;

use std::time::Duration;

use avian3d::PhysicsPlugins;
use bevy::{prelude::*, time::common_conditions::on_timer};
use color_eyre::eyre::Result;

pub const APP_NAME: &str = "Mallet";

fn main() -> Result<()> {
    color_eyre::install()?;

    App::new()
        .add_plugins((
            app_data::plugin,
            DefaultPlugins,
            PhysicsPlugins::default(),
            core::plugin,
            editor::plugin,
            game::plugin,
        ))
        // Only update when there is user input. Should be disabled when in-game
        //.insert_resource(WinitSettings::desktop_app())
        .add_systems(PreUpdate, file_drop)
        //.add_systems(Update, debuggy.run_if(on_timer(Duration::from_secs(1))))
        .run();
    Ok(())
}

fn debuggy(q_all_entities: Query<Entity>) {
    let entities = q_all_entities.iter().len();
    info!("{} entities", entities);
}

fn file_drop(mut evr_dnd: EventReader<FileDragAndDrop>) {
    for ev in evr_dnd.read() {
        info!("dnd event: {:?}", ev);
        if let FileDragAndDrop::DroppedFile { window, path_buf } = ev {
            info!(
                "Dropped file with path: {:?}, in window id: {:?}",
                path_buf, window
            );
        }
    }
}
