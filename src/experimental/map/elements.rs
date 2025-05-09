use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    core::map::{brush::Brush, light::Light},
    id::Id,
};

#[derive(Component, Deref, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ElemId(Id);

impl ElemId {
    pub fn new(id: Id) -> Self {
        Self(id)
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone)]
pub struct ElemMeta {
    pub name: String,
    pub role: ElemRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum ElemRole {
    Brush = 0,
    Light = 1,
}

pub trait ElemParams: Send + Sync {
    fn role(&self) -> ElemRole;
}

impl ElemParams for Brush {
    fn role(&self) -> ElemRole {
        ElemRole::Brush
    }
}

impl ElemParams for Light {
    fn role(&self) -> ElemRole {
        ElemRole::Light
    }
}
