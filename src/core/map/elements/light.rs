use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::map::changes::{get_elem_entity, Change, UpdateElemParams};

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

impl Change for UpdateElemParams<Light> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        //entity.insert(self.params.clone());
        let light = self.params.clone();
        entity.insert((
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
