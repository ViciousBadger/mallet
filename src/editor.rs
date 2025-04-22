pub mod actions;
pub mod freelook;
pub mod selection;

use crate::{AppState, AppStateSwitchConf};
use bevy::prelude::*;
use freelook::Freelook;

fn init_editor(mut commands: Commands, init_conf: Option<Res<AppStateSwitchConf>>) {
    let default_conf = AppStateSwitchConf::default();
    let conf = init_conf
        .map(|res| res.into_inner())
        .unwrap_or(&default_conf);

    commands.spawn((
        StateScoped(AppState::InEditor),
        Transform::from_translation(conf.pos),
        conf.look.clone(),
        Freelook::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0_f32.to_radians(),
            ..default()
        }),
    ));
    commands.spawn((
        StateScoped(AppState::InEditor),
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_x(-0.8)),
    ));
}

fn teardown_editor(_: Commands) {
    //Remove resources etc...
}

pub fn plugin(app: &mut App) {
    app.add_plugins((freelook::plugin, selection::plugin, actions::plugin))
        .add_systems(OnEnter(AppState::InEditor), init_editor)
        .add_systems(OnExit(AppState::InEditor), teardown_editor);
}
