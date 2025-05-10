use std::{
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
};

use bevy::{ecs::system::SystemId, platform::collections::HashMap, prelude::*};
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

pub trait Deployable: Send + Sync + DeserializeOwned + 'static {
    fn identifier() -> &'static str;
    fn id_hash() -> u64 {
        let mut s = DefaultHasher::new();
        Self::identifier().hash(&mut s);
        s.finish()
    }
}

impl Deployable for ElemMeta {
    fn identifier() -> &'static str {
        "meta"
    }
}

impl Deployable for Brush {
    fn identifier() -> &'static str {
        "brush"
    }
}

impl Deployable for Light {
    fn identifier() -> &'static str {
        "light"
    }
}

#[derive(Event)]
pub struct Deploy<T: Deployable> {
    input: T,
    target: Entity,
}

pub fn deploy_meta(trigger: Trigger<Deploy<ElemMeta>>, mut commands: Commands) {
    let meta = trigger.input.clone();
    info!("deploy meta!! {:?}", meta);
    commands.entity(trigger.target).insert(meta);
}

pub fn deploy_brush(trigger: Trigger<Deploy<Brush>>, mut commands: Commands) {
    // let brush = trigger.input.cast::<Brush>();
    let brush = trigger.input.clone();
    info!("deploy brush!! {:?}", brush);
    commands.entity(trigger.target).insert(brush);
}

trait AppRegisterDeployable {
    fn register_deployable<T: Deployable>(&mut self);
}

impl AppRegisterDeployable for App {
    fn register_deployable<T>(&mut self)
    where
        T: Deployable,
    {
        let deployer_fn = |input: Object, target: Entity, mut commands: Commands| {
            let typed_input = input.cast::<T>();
            commands.trigger(Deploy {
                input: typed_input,
                target,
            });
        };
        self.world_mut()
            .resource_mut::<DeployerRegistry>()
            .register::<T>(deployer_fn);

        // this is where we would grab the "deploy registry"
        // and insert a thingy that can somehow determine which concrete Deploy<T> to call
        // .. based on whataver actually needs to be deployed..

        // let sys = |input: In<(Object, Entity)>, mut commands: Commands| {
        //     let (obj, ent) = input.0;
        //     commands.trigger(Deploy {
        //         input: obj,
        //         entity: ent,
        //     });
        // };
    }
}

pub trait Deployer: Send + Sync + 'static {
    fn deploy(&self, input: Object, target: Entity, commands: Commands);
}

impl<F> Deployer for F
where
    F: Fn(Object, Entity, Commands) + Send + Sync + 'static,
{
    fn deploy(&self, input: Object, target: Entity, commands: Commands) {
        self(input, target, commands);
    }
}

#[derive(Resource)]
struct DeployerRegistry {
    pub deployers: HashMap<u64, Box<dyn Deployer>>,
}
impl DeployerRegistry {
    pub fn register<T>(&mut self, deployer: impl Deployer)
    where
        T: Deployable,
    {
        let id_hash = T::id_hash();
        self.deployers.insert(id_hash, Box::new(deployer));
    }
}
