pub mod binds;
pub mod content;
pub mod map;
pub mod media;
pub mod view;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use binds::{Binding, InputBindingSystem};
use view::{Gimbal, GimbalPos};

use crate::{editor::update_editor_context, game::GameRules, util::IdGen};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    InEditor,
    InGame,
}

fn init(mut commands: Commands) {
    // Key light
    commands.spawn((
        StudioLight,
        DirectionalLight {
            illuminance: 1_500.,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            0_f32.to_radians(),
            -45_f32.to_radians(),
            0_f32.to_radians(),
        )),
    ));
    // Fill light
    commands.spawn((
        StudioLight,
        DirectionalLight {
            illuminance: 1_000.,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            -85_f32.to_radians(),
            -25_f32.to_radians(),
            0_f32.to_radians(),
        )),
    ));
    // Back light
    commands.spawn((
        StudioLight,
        DirectionalLight {
            illuminance: 300.,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            115_f32.to_radians(),
            45_f32.to_radians(),
            0_f32.to_radians(),
        )),
    ));
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
                rot: *gimbal,
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

#[derive(Component)]
struct StudioLight;

fn toggle_studio_light(
    q_lights: Query<(Entity, &Visibility), With<StudioLight>>,
    mut commands: Commands,
) {
    for (light_entity, light_vis) in &q_lights {
        if light_vis == Visibility::Hidden {
            commands.entity(light_entity).insert(Visibility::Inherited);
        } else {
            commands.entity(light_entity).insert(Visibility::Hidden);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins((content::plugin, binds::plugin, view::plugin, map::plugin))
        .init_resource::<IdGen>()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight::NONE)
        .init_state::<AppState>()
        .enable_state_scoped_entities::<AppState>()
        .add_systems(Startup, init)
        .add_systems(
            PreUpdate,
            (
                exit_app.run_if(input_just_pressed(Binding::Quit)),
                (
                    update_editor_context.run_if(in_state(AppState::InEditor)),
                    playtest,
                )
                    .chain()
                    .run_if(input_just_pressed(Binding::Playtest)),
                toggle_studio_light.run_if(input_just_pressed(KeyCode::KeyL)),
            )
                .after(InputBindingSystem),
        );
}
