mod app_data;
mod editor;
mod input_binding;
mod map;
mod util;

use avian3d::PhysicsPlugins;
use bevy::{app::AppExit, input::common_conditions::input_just_pressed, prelude::*};
use color_eyre::eyre::Result;
use input_binding::{Binding, InputBindingSystem};
use util::IdGen;

pub const APP_NAME: &'static str = "Mallet";

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    InEditor,
    InGame,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    App::new()
        .add_plugins((
            app_data::plugin,
            input_binding::plugin,
            DefaultPlugins,
            PhysicsPlugins::default(),
            map::plugin,
            editor::plugin,
        ))
        // Only update when there is user input. Should be disabled when in-game
        //.insert_resource(WinitSettings::desktop_app())
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .add_systems(
            PreUpdate,
            (exit_app.run_if(input_just_pressed(Binding::Quit)),).after(InputBindingSystem),
        )
        .add_systems(PreUpdate, file_drop)
        .init_resource::<IdGen>()
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

fn exit_app(mut exit_events: ResMut<Events<AppExit>>) {
    exit_events.send_default();
}
