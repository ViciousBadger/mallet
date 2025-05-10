use bevy::prelude::*;
use serde::de::DeserializeOwned;

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::{
        db::{Checksum, Db, TBL_OBJECTS},
        elements::{ElemId, ElemMeta, ElemParams, ElemRole, NewMeta},
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

    pub fn push_single<T>(&mut self, change: T)
    where
        T: Change + 'static,
    {
        self.0.push(ChangeSet {
            changes: vec![Box::new(change)],
        });
    }

    pub fn push_many<T>(&mut self, changes: Vec<T>)
    where
        T: Change + 'static,
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

pub fn get_elem_params<T>(world: &mut World, checksum: &Checksum) -> T
where
    T: ElemParams + DeserializeOwned,
{
    world
        .resource::<Db>()
        .begin_read()
        .unwrap()
        .open_table(TBL_OBJECTS)
        .unwrap()
        .get(checksum)
        .unwrap()
        .unwrap()
        .value()
        .cast::<T>()
}

#[derive(Debug)]
pub struct CreateElem<T> {
    pub id: NewElemId,
    pub meta: NewMeta,
    pub params: T,
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

impl<T> CreateElem<T>
where
    T: ElemParams,
{
    pub fn spawn<'a>(&'a self, world: &'a mut World) -> EntityWorldMut<'a> {
        let id = self
            .id
            .loaded_id_or_none()
            .unwrap_or_else(|| world.resource_mut::<IdGen>().generate());
        world.spawn((
            ElemId::new(id),
            ElemMeta::from_new(&self.meta, self.params.role()),
        ))
    }
}

impl Change for CreateElem<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        // NOTE: hey, maybe this code should be where the whole "deploy" thing happens. since we
        // already have the concrete type of element..
        // CAVEAT: restoring state does not invoke a CreateElem!
        // when restoring a state the entire ElemMeta is spawned, not just .name ..
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
pub struct UpdateElemMeta {
    pub elem_id: Id,
    pub new_meta: ElemMeta,
}

// TODO: use CreateOrUpdate? create if nto exist, update if exist.
impl Change for UpdateElemMeta {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        entity.insert(self.new_meta.clone());
        info!("applied modmeta");
    }
}

#[derive(Debug)]
pub struct UpdateElemParams<T> {
    pub elem_id: Id,
    pub new_params: T,
}

impl Change for UpdateElemParams<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        entity.insert(self.new_params.clone());
        info!("applied modparams for Brush");
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

#[derive(Debug)]
pub struct RestoreElem {
    pub id: Id,
    pub meta: ElemMeta,
    pub params: Checksum,
}

impl Change for RestoreElem {
    fn apply_to_world(&self, world: &mut World) {
        if elem_has_entity(world, &self.id) {
            UpdateElemMeta {
                elem_id: self.id,
                new_meta: self.meta.clone(),
            }
            .apply_to_world(world);
            match self.meta.role {
                ElemRole::Brush => {
                    UpdateElemParams {
                        elem_id: self.id,
                        new_params: get_elem_params::<Brush>(world, &self.params),
                    }
                    .apply_to_world(world);
                }
                ElemRole::Light => {
                    UpdateElemParams {
                        elem_id: self.id,
                        new_params: get_elem_params::<Light>(world, &self.params),
                    }
                    .apply_to_world(world);
                }
            }
        } else {
            match self.meta.role {
                ElemRole::Brush => {
                    CreateElem {
                        id: NewElemId::Loaded(self.id),
                        meta: self.meta.clone().into(),
                        params: get_elem_params::<Brush>(world, &self.params),
                    }
                    .apply_to_world(world);
                }
                ElemRole::Light => {
                    CreateElem {
                        id: NewElemId::Loaded(self.id),
                        meta: self.meta.clone().into(),
                        params: get_elem_params::<Light>(world, &self.params),
                    }
                    .apply_to_world(world);
                }
            }
        }
    }
}
