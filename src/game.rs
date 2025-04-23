use bevy::prelude::*;

use crate::{
    core::{AppState, AppStateSwitchConf},
    util::{grab_mouse, release_mouse},
};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::InGame), (init_game, grab_mouse))
        .add_systems(OnExit(AppState::InGame), (teardown_game, release_mouse));
}

fn init_game(mut commands: Commands, init_conf: Option<Res<AppStateSwitchConf>>) {
    let default_conf = AppStateSwitchConf::default();
    let conf = init_conf
        .map(|res| res.into_inner())
        .unwrap_or(&default_conf);

    // player
    let player = commands
        .spawn((
            StateScoped(AppState::InGame),
            Transform::from_translation(conf.pos),
            Visibility::Visible,
        ))
        .id();

    // player head
    commands
        .spawn((
            conf.look.clone(),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 72.0_f32.to_radians(),
                ..default()
            }),
        ))
        .set_parent(player);

    // test light
    commands.spawn((
        StateScoped(AppState::InGame),
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_x(-0.8)),
    ));
}

fn teardown_game(_: Commands) {
    //Remove resources etc...
}
