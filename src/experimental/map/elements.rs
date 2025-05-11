use std::{
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
};

use bevy::{asset::ErasedAssetLoader, platform::collections::HashMap, prelude::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::{
        changes::{Change, CreateElem, NewElemId, UpdateElemParams},
        db::Object,
    },
    id::Id,
};

#[derive(Component, Deref, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ElementId(Id);

impl ElementId {
    pub fn new(id: Id) -> Self {
        Self(id)
    }

    pub fn id_ref(&self) -> &Id {
        &self.0
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Info {
    pub name: String,
}

pub trait Role: Send + Sync + DeserializeOwned + std::fmt::Debug + Clone {
    fn id() -> &'static str;
    fn id_hash() -> u64 {
        let mut s = DefaultHasher::new();
        Self::id().hash(&mut s);
        s.finish()
    }
}

impl Role for Brush {
    fn id() -> &'static str {
        "brush"
    }
}

impl Role for Light {
    fn id() -> &'static str {
        "light"
    }
}

#[derive(Resource, Default)]
pub struct ElementRoleRegistry {
    pub roles: HashMap<u64, Box<dyn ChangeBuilder>>,
}

pub trait ChangeBuilder: Send + Sync + 'static {
    fn build_create(&self, id: NewElemId, info: Info, raw_params: Object) -> Box<dyn Change>;
    fn build_update(&self, id: Id, raw_params: Object) -> Box<dyn Change>;
}

struct RoleChangeBuilder<R>(PhantomData<R>);
impl<R> RoleChangeBuilder<R> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<R> ChangeBuilder for RoleChangeBuilder<R>
where
    R: Role + 'static,
    CreateElem<R>: Change,
    UpdateElemParams<R>: Change,
{
    fn build_create(&self, id: NewElemId, info: Info, raw_params: Object) -> Box<dyn Change> {
        let params = raw_params.cast::<R>();
        Box::new(CreateElem { id, info, params })
    }

    fn build_update(&self, elem_id: Id, raw_params: Object) -> Box<dyn Change> {
        let new_params = raw_params.cast::<R>();
        Box::new(UpdateElemParams {
            elem_id,
            new_params,
        })
    }
}

impl ElementRoleRegistry {
    pub fn register<R>(&mut self)
    where
        R: Role + 'static,
        CreateElem<R>: Change,
        UpdateElemParams<R>: Change,
    {
        let id_hash = R::id_hash();
        let builder: RoleChangeBuilder<R> = RoleChangeBuilder::new();
        self.roles.insert(id_hash, Box::new(builder));
    }
}

pub trait AppRoleRegistry {
    fn register_map_element_role<R>(&mut self)
    where
        R: Role + 'static,
        CreateElem<R>: Change,
        UpdateElemParams<R>: Change;
}

impl AppRoleRegistry for App {
    fn register_map_element_role<R>(&mut self)
    where
        R: Role + 'static,
        CreateElem<R>: Change,
        UpdateElemParams<R>: Change,
    {
        self.world_mut()
            .resource_mut::<ElementRoleRegistry>()
            .register::<R>();
    }
}
