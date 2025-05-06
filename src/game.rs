pub mod actor;

use actor::PlayerActor;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    core::{
        view::{GimbalPos, GimbalRotatesParent},
        AppState,
    },
    util::{grab_mouse, release_mouse},
};

#[derive(Resource)]
pub struct GameRules {
    pub spawn: GimbalPos,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            spawn: GimbalPos {
                pos: Vec3::Y * 5.0,
                rot: default(),
            },
        }
    }
}

fn init_game(mut commands: Commands, init_conf: Option<Res<GameRules>>) {
    let default_conf = GameRules::default();
    let conf = init_conf
        .map(|res| res.into_inner())
        .unwrap_or(&default_conf);

    let player_head_height = 1.0;
    let player_coll = Collider::capsule(0.4, 1.0);
    let mut caster_shape = player_coll.clone();
    caster_shape.set_scale(Vec3::ONE * 0.99, 10);

    commands.spawn((
        // Player avatar body
        StateScoped(AppState::InGame),
        PlayerActor,
        Transform::from_translation(conf.spawn.pos - Vec3::Y * player_head_height),
        Visibility::Visible,
        RigidBody::Kinematic,
        ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
            .with_max_distance(0.2),
        player_coll,
        children![(
            // Player head
            Transform::from_xyz(0.0, player_head_height, 0.0),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 72.0_f32.to_radians(),
                ..default()
            }),
            conf.spawn.rot,
            GimbalRotatesParent,
        )],
    ));
}

fn teardown_game(_: Commands) {
    //Remove resources etc...
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameSystems;

pub fn plugin(app: &mut App) {
    app.add_plugins(actor::plugin)
        .configure_sets(PreUpdate, GameSystems.run_if(in_state(AppState::InGame)))
        .configure_sets(Update, GameSystems.run_if(in_state(AppState::InGame)))
        .configure_sets(
            PhysicsSchedule,
            GameSystems.run_if(in_state(AppState::InGame)),
        )
        .add_systems(OnEnter(AppState::InGame), (init_game, grab_mouse))
        .add_systems(OnExit(AppState::InGame), (teardown_game, release_mouse));
}
