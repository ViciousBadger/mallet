use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::map::{nodes::MapNodeMeta, MapNodeDeploy};

#[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Light {
    pub position: Vec3,
    pub light_type: LightType,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LightType {
    Point,
    Spot,
}

pub fn deploy_lights(
    q_lights: Query<(Entity, &MapNodeMeta, &Light)>,
    mut deploy_events: EventReader<MapNodeDeploy>,
    mut commands: Commands,
) {
    for event in deploy_events.read() {
        if let Ok((entity, _meta, light)) = q_lights.get(event.target) {
            let mut entity_commands = commands.entity(entity);

            entity_commands.insert((
                Transform::from_translation(light.position),
                match light.light_type {
                    LightType::Point => PointLight {
                        color: light.color,
                        intensity: light.intensity,
                        range: light.range,
                        ..default()
                    },
                    LightType::Spot => {
                        unimplemented!("u would have to rotate it n shit")
                    }
                },
            ));
        }
    }
}
pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, deploy_lights);
}
