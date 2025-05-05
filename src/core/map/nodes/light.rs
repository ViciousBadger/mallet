use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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

pub fn plugin(app: &mut App) {
    //app.add_systems(PostUpdate, deploy_brushes);
}
