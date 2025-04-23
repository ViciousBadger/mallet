mod core;
mod editor;
mod game;
mod util;

use avian3d::PhysicsPlugins;
use bevy::prelude::*;
use color_eyre::eyre::Result;

pub const APP_NAME: &str = "Mallet";

fn main() -> Result<()> {
    color_eyre::install()?;

    App::new()
        .add_plugins((
            core::plugin,
            DefaultPlugins,
            PhysicsPlugins::default(),
            editor::plugin,
            game::plugin,
        ))
        // Only update when there is user input. Should be disabled when in-game
        //.insert_resource(WinitSettings::desktop_app())
        .add_systems(PreUpdate, file_drop)
        .run();
    Ok(())
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
