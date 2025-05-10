use bevy::prelude::*;
use serde::de::DeserializeOwned;

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::{
        db::{Checksum, Db, TBL_OBJECTS},
        elements::{ElementId, Info, Role},
        ElementLookup,
    },
    id::{Id, IdGen},
};

pub trait Change: std::fmt::Debug + Send + Sync {
    fn apply_to_world(&self, world: &mut World);
}

#[derive(Debug)]
pub struct ChangeSet {
    pub changes: Vec<Box<dyn Change>>,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PendingChanges(Vec<ChangeSet>);

impl PendingChanges {
    pub fn push_set(&mut self, set: ChangeSet) {
        self.0.push(set);
    }

    pub fn push_single<C>(&mut self, change: C)
    where
        C: Change + 'static,
    {
        self.0.push(ChangeSet {
            changes: vec![Box::new(change)],
        });
    }

    pub fn push_many<C>(&mut self, changes: Vec<C>)
    where
        C: Change + 'static,
    {
        let mut boxed_changes: Vec<Box<dyn Change>> = Vec::new();
        for c in changes {
            boxed_changes.push(Box::new(c));
        }
        self.0.push(ChangeSet {
            changes: boxed_changes,
        });
    }
}

pub fn get_elem_entity<'a>(world: &'a mut World, elem_id: &Id) -> Option<EntityWorldMut<'a>> {
    let entity_id = world.resource_mut::<ElementLookup>().find(elem_id).ok()?;
    Some(world.entity_mut(entity_id))
}

pub fn elem_has_entity(world: &mut World, elem_id: &Id) -> bool {
    world.resource_mut::<ElementLookup>().find(elem_id).is_ok()
}

#[derive(Debug)]
pub struct CreateElem<R> {
    pub id: NewElemId,
    pub info: Info,
    pub params: R,
}

#[derive(Debug)]
pub enum NewElemId {
    Loaded(Id),
    Generated,
}

impl NewElemId {
    pub fn loaded_id_or_none(&self) -> Option<Id> {
        match self {
            NewElemId::Loaded(id) => Some(*id),
            NewElemId::Generated => None,
        }
    }
}

impl<R> CreateElem<R>
where
    R: Role,
{
    pub fn spawn<'a>(&'a self, world: &'a mut World) -> EntityWorldMut<'a> {
        let id = self
            .id
            .loaded_id_or_none()
            .unwrap_or_else(|| world.resource_mut::<IdGen>().generate());
        world.spawn((ElementId::new(id), self.info.clone()))
    }
}

impl Change for CreateElem<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = self.spawn(world);
        entity.insert(self.params.clone());
        info!("applied create for Brush");
    }
}

impl Change for CreateElem<Light> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = self.spawn(world);
        entity.insert(self.params.clone());
        info!("applied create for Light");
    }
}

#[derive(Debug)]
pub struct UpdateElemInfo {
    pub elem_id: Id,
    pub new_info: Info,
}

// TODO: use CreateOrUpdate? create if nto exist, update if exist.
impl Change for UpdateElemInfo {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        entity.insert(self.new_info.clone());
        info!("applied updateeleminfo");
    }
}

#[derive(Debug)]
pub struct UpdateElemParams<R> {
    pub elem_id: Id,
    pub new_params: R,
}

impl Change for UpdateElemParams<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        entity.insert(self.new_params.clone());
        info!("applied updateelemparams for Brush");
    }
}

impl Change for UpdateElemParams<Light> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        entity.insert(self.new_params.clone());
    }
}

#[derive(Debug)]
pub struct RemoveElem {
    pub elem_id: Id,
}

impl Change for RemoveElem {
    fn apply_to_world(&self, world: &mut World) {
        get_elem_entity(world, &self.elem_id).unwrap().despawn();
    }
}
