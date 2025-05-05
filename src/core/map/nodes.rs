pub mod brush;
pub mod light;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoDiscriminant};

use crate::{
    core::map::nodes::{brush::Brush, light::Light},
    util::Id,
};

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MapNodeMeta {
    pub id: Id,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Component, EnumDiscriminants)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[strum_discriminants(name(MapNodeType))]
pub enum TypedMapNode {
    Brush(Brush),
    Light(Light),
}

impl TypedMapNode {
    pub fn insert_as_component(&self, mut entity_cmds: EntityCommands) {
        match self {
            TypedMapNode::Brush(brush) => entity_cmds.insert(brush.clone()),
            TypedMapNode::Light(light) => entity_cmds.insert(light.clone()),
        };
    }

    pub fn remove_as_component<'a>(&self, mut entity_cmds: EntityCommands) {
        match self.discriminant() {
            MapNodeType::Brush => entity_cmds.remove::<Brush>(),
            MapNodeType::Light => entity_cmds.remove::<Light>(),
        };
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins((brush::plugin, light::plugin));
}
