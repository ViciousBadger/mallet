use std::any::Any;

use bevy::prelude::*;
use redb::TableDefinition;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

// Playground below
// Dyn is weird.

pub trait ElementContent: Send + Sync {
    fn role(&self) -> ElementRole;
}

impl ElementContent for Brush {
    fn role(&self) -> ElementRole {
        ElementRole::Brush
    }
}

impl ElementContent for Light {
    fn role(&self) -> ElementRole {
        ElementRole::Light
    }
}

#[derive(Error, Debug)]
#[error("asdf")]
pub struct ContentDowncastFail;

#[derive(Deref)]
pub struct ErasedContent {
    inner: Box<dyn ElementContent>,
}
impl ErasedContent {
    pub fn downcast_ref<T: 'static>(&self) -> Result<&T, ContentDowncastFail> {
        (&self.inner as &dyn Any)
            .downcast_ref()
            .ok_or(ContentDowncastFail)
    }
}
