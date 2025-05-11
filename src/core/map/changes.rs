use bevy::prelude::*;

use crate::{
    core::map::{
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

#[derive(Debug)]
pub struct CreateElem<R> {
    pub id: CreateId,
    pub info: Info,
    pub params: R,
}

#[derive(Debug)]
pub enum CreateId {
    Loaded(Id),
    Generated,
}

impl CreateId {
    pub fn loaded_id_or_none(&self) -> Option<Id> {
        match self {
            CreateId::Loaded(id) => Some(*id),
            CreateId::Generated => None,
        }
    }
}

impl<R> Change for CreateElem<R>
where
    R: Role,
    UpdateElemParams<R>: Change,
{
    fn apply_to_world(&self, world: &mut World) {
        let id = self
            .id
            .loaded_id_or_none()
            .unwrap_or_else(|| world.resource_mut::<IdGen>().generate());
        let entity_id = world.spawn((ElementId::new(id), self.info.clone())).id();
        world.resource_mut::<ElementLookup>().insert(id, entity_id);

        // Ok, now update info and params, to re-use the code.
        UpdateElemInfo {
            elem_id: id,
            new_info: self.info.clone(),
        }
        .apply_to_world(world);
        UpdateElemParams {
            elem_id: id,
            params: self.params.clone(),
        }
        .apply_to_world(world);
        info!("applied create for a generic elem role :)");
    }
}

#[derive(Debug)]
pub struct UpdateElemInfo {
    pub elem_id: Id,
    pub new_info: Info,
}

impl Change for UpdateElemInfo {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
        entity.insert(self.new_info.clone());
        info!("applied updateeleminfo");
    }
}

#[derive(Debug, Clone)]
pub struct UpdateElemParams<R> {
    pub elem_id: Id,
    pub params: R,
}

#[derive(Debug)]
pub struct RemoveElement {
    pub elem_id: Id,
}

impl Change for RemoveElement {
    fn apply_to_world(&self, world: &mut World) {
        get_elem_entity(world, &self.elem_id).unwrap().despawn();
    }
}
