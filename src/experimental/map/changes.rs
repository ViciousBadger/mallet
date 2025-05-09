use bevy::prelude::*;

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::{
        elements::{ElemId, ElemMeta, ElemParams},
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

pub fn get_elem_entity<'a>(world: &'a mut World, elem_id: &Id) -> EntityWorldMut<'a> {
    let entity_id = world.resource_mut::<ElementLookup>().find(elem_id).unwrap();
    world.entity_mut(entity_id)
}

#[derive(Debug)]
pub struct CreateElem<T> {
    pub name: String,
    pub params: T,
}

impl<T> CreateElem<T>
where
    T: ElemParams,
{
    pub fn spawn<'a>(&'a self, world: &'a mut World) -> EntityWorldMut<'a> {
        let new_id = world.resource_mut::<IdGen>().generate();
        world.spawn((
            ElemId::new(new_id),
            ElemMeta {
                name: self.name.clone(),
                role: self.params.role(),
            },
        ))
    }
}

impl Change for CreateElem<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        // NOTE: hey, maybe this code should be where the whole "deploy" thing happens. since we
        // already have the concrete type of element..
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

impl Change for UpdateElemMeta {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id);
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
        let mut entity = get_elem_entity(world, &self.elem_id);
        entity.insert(self.new_params.clone());
        info!("applied modparams for Brush");
    }
}

impl Change for UpdateElemParams<Light> {
    fn apply_to_world(&self, world: &mut World) {
        let mut entity = get_elem_entity(world, &self.elem_id);
        entity.insert(self.new_params.clone());
    }
}

#[derive(Debug)]
pub struct RemoveElem {
    pub elem_id: Id,
}

impl Change for RemoveElem {
    fn apply_to_world(&self, world: &mut World) {
        get_elem_entity(world, &self.elem_id).despawn();
    }
}
