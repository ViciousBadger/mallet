use bevy::prelude::*;
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::db::Postcard,
    id::Id,
};

pub const CONTENT_TABLE_BRUSH: TableDefinition<Id, Postcard<Brush>> =
    TableDefinition::new("content_brush");
pub const CONTENT_TABLE_LIGHT: TableDefinition<Id, Postcard<Light>> =
    TableDefinition::new("content_light");
pub const MAIN_STATE_TABLE: TableDefinition<Id, Postcard<Element>> =
    TableDefinition::new("main_state");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Element {
    pub name: String,
    pub role: ElementRole,
    pub content_key: Id,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum ElementRole {
    Brush = 0,
    Light = 1,
}
