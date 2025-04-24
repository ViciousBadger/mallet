pub mod binds;
pub mod map;
pub mod view;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use binds::{Binding, InputBindingSystem};
use view::{Gimbal, GimbalPos};

use crate::{game::GameRules, util::IdGen};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    InEditor,
    InGame,
}

fn playtest(
    app_state: Res<State<AppState>>,
    q_existing_cam: Query<(&GlobalTransform, &Gimbal), With<Camera>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    if let Ok((transform, gimbal)) = q_existing_cam.get_single() {
        commands.insert_resource(GameRules {
            spawn: GimbalPos {
                pos: transform.translation(),
                rot: gimbal.clone(),
            },
        });
    }

    next_app_state.set(if app_state.get() == &AppState::InEditor {
        AppState::InGame
    } else {
        AppState::InEditor
    })
}

fn exit_app(mut exit_events: ResMut<Events<AppExit>>) {
    exit_events.send_default();
}

pub fn plugin(app: &mut App) {
    app.add_plugins((binds::plugin, view::plugin, map::plugin))
        .init_resource::<IdGen>()
        .insert_resource(ClearColor(Color::BLACK))
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .add_systems(
            PreUpdate,
            (
                exit_app.run_if(input_just_pressed(Binding::Quit)),
                playtest.run_if(input_just_pressed(Binding::Playtest)),
            )
                .after(InputBindingSystem),
        );
}
