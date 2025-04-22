mod app_data;
mod editor;
mod game;
mod input_binding;
mod map;
mod util;
mod view;

use avian3d::PhysicsPlugins;
use bevy::{app::AppExit, input::common_conditions::input_just_pressed, prelude::*};
use color_eyre::eyre::Result;
use input_binding::{Binding, InputBindingSystem};
use util::IdGen;
use view::Gimbal;

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
            DefaultPlugins,
            PhysicsPlugins::default(),
            input_binding::plugin,
            view::plugin,
            map::plugin,
            editor::plugin,
            game::plugin,
        ))
        // Only update when there is user input. Should be disabled when in-game
        //.insert_resource(WinitSettings::desktop_app())
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .add_systems(
            PreUpdate,
            (
                exit_app.run_if(input_just_pressed(Binding::Quit)),
                playtest.run_if(input_just_pressed(Binding::Playtest)),
            )
                .after(InputBindingSystem),
        )
        .add_systems(PreUpdate, file_drop)
        .init_resource::<IdGen>()
        .run();

    Ok(())
}

#[derive(Resource, Default)]
pub struct AppStateSwitchConf {
    pub pos: Vec3,
    pub look: Gimbal,
}

fn playtest(
    app_state: Res<State<AppState>>,
    q_existing_cam: Query<(&GlobalTransform, &Gimbal), With<Camera>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    if let Ok((transform, gimbal)) = q_existing_cam.get_single() {
        commands.insert_resource(AppStateSwitchConf {
            pos: transform.translation(),
            look: gimbal.clone(),
        });
    }

    next_app_state.set(if app_state.get() == &AppState::InEditor {
        AppState::InGame
    } else {
        AppState::InEditor
    })
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
