mod app_data;
mod core;
mod editor;
mod experimental;
mod game;
mod id;
mod util;

use avian3d::{prelude::PhysicsInterpolationPlugin, PhysicsPlugins};
use bevy::prelude::*;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

pub const APP_NAME: &str = "Mallet";

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Experiment,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match &cli.command {
        None => {
            App::new()
                .add_plugins((
                    app_data::plugin,
                    DefaultPlugins,
                    //PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()),
                    PhysicsPlugins::default(),
                    core::plugin,
                    editor::plugin,
                    game::plugin,
                ))
                // Only update when there is user input. Should be disabled when in-game
                //.insert_resource(WinitSettings::desktop_app())
                .add_systems(PreUpdate, file_drop)
                .run();
        }
        Some(Commands::Experiment) => {
            experimental::run_playground();
        }
    }
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
