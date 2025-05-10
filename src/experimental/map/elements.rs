use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::{platform::collections::HashMap, prelude::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::db::Object,
    id::Id,
};

#[derive(Component, Deref, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ElemId(Id);

impl ElemId {
    pub fn new(id: Id) -> Self {
        Self(id)
    }

    pub fn id_ref(&self) -> &Id {
        &self.0
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElemMeta {
    pub name: String,
    pub role: ElemRole,
}

#[derive(Debug, Clone)]
pub struct NewMeta {
    pub name: String,
}

impl From<ElemMeta> for NewMeta {
    fn from(ElemMeta { name, .. }: ElemMeta) -> Self {
        Self { name }
    }
}

impl ElemMeta {
    pub fn from_new(value: &NewMeta, role: ElemRole) -> Self {
        let NewMeta { name } = value.clone();
        Self { role, name }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ElemRole {
    Brush = 0,
    Light = 1,
}

pub trait ElemParams: Send + Sync + std::fmt::Debug {
    fn role(&self) -> ElemRole;

    fn identifier() -> &'static str;
    fn id_hash() -> u64 {
        let mut s = DefaultHasher::new();
        Self::identifier().hash(&mut s);
        s.finish()
    }
}

impl ElemParams for Brush {
    fn role(&self) -> ElemRole {
        ElemRole::Brush
    }
    fn identifier() -> &'static str {
        "brush"
    }
}

impl ElemParams for Light {
    fn role(&self) -> ElemRole {
        ElemRole::Light
    }
    fn identifier() -> &'static str {
        "light"
    }
}
