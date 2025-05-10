use std::hash::{DefaultHasher, Hash, Hasher};

use bevy::{asset::ErasedAssetLoader, platform::collections::HashMap, prelude::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::{
        changes::{Change, CreateElem, NewElemId},
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

pub trait Role: Send + Sync + DeserializeOwned {
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

#[derive(Resource)]
pub struct ElementRoleRegistry {
    pub roles: HashMap<u64, RoleImpls>,
}

pub trait RoleCreateFn: Send + Sync {
    fn build(&self, id: NewElemId, info: Info, params: Object) -> Box<dyn Change>;
}

impl<F> RoleCreateFn for F
where
    F: Fn(NewElemId, Info, Object) -> Box<dyn Change> + Send + Sync + 'static,
{
    fn build(&self, id: NewElemId, info: Info, params: Object) -> Box<dyn Change> {
        self(id, info, params)
    }
}

pub trait RoleUpdateFn: Send + Sync {
    fn build(&self, id: Id, new_params: Object) -> Box<dyn Change>;
}

impl<F> RoleUpdateFn for F
where
    F: Fn(Id, Object) -> Box<dyn Change> + Send + Sync + 'static,
{
    fn build(&self, id: Id, params: Object) -> Box<dyn Change> {
        self(id, params)
    }
}

pub struct RoleImpls {
    pub build_create: Box<dyn RoleCreateFn + Send + Sync>,
    pub build_update: Box<dyn RoleUpdateFn + Send + Sync>,
}

impl ElementRoleRegistry {
    pub fn register<R>(&mut self, impls: RoleImpls)
    where
        R: Role,
    {
        let id_hash = R::id_hash();
        // let create = |id, info, params: Object| {
        //     let params = params.cast::<R>();
        //     todo!();
        //     Box::new(CreateElem { id, info, params })
        // };

        self.roles.insert(id_hash, impls);
    }
}

pub trait AppRoles {
    fn register_map_element_role<R: Role>(&mut self, impls: RoleImpls);
}

impl AppRoles for App {
    fn register_map_element_role<R: Role>(&mut self, impls: RoleImpls) {
        self.world_mut()
            .resource_mut::<ElementRoleRegistry>()
            .register::<R>(impls);
    }
}
