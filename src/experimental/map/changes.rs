use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::{
    core::map::{brush::Brush, light::Light},
    experimental::map::{
        elements::{ElementId, Info, Role},
        ElementLookup, MapAssets,
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
            new_params: self.params.clone(),
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
    pub new_params: R,
}

impl Change for UpdateElemParams<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        world
            .run_system_cached_with(
                |change: In<Self>,
                 lookup: Res<ElementLookup>,
                 map_assets: Res<MapAssets>,
                 mut meshes: ResMut<Assets<Mesh>>,
                 mut commands: Commands|
                 -> Result {
                    let entity_id = lookup.find(&change.elem_id)?;
                    let mut entity = commands.entity(entity_id);
                    //let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
                    let brush = change.new_params.clone();
                    // entity.insert(brush.clone());

                    // Brush will use base entity as a container for sides.
                    let center = brush.bounds.center();
                    let size = brush.bounds.size();

                    entity.insert((
                        brush.clone(),
                        Transform::IDENTITY.with_translation(center),
                        RigidBody::Static,
                        Collider::cuboid(size.x, size.y, size.z),
                    ));

                    for side in brush.bounds.sides_local() {
                        let mesh = meshes.add(side.mesh());
                        let material = map_assets.default_material.clone();
                        entity.with_child((
                            Transform::IDENTITY.with_translation(side.pos),
                            Mesh3d(mesh),
                            //MeshMaterial3d(materials.add(color)),
                            MeshMaterial3d(material),
                        ));
                    }

                    info!("applied updateelemparams for Brush :O it runs as a cached system");
                    Ok(())
                },
                self.clone(),
            )
            .expect("error running system")
            .expect("system returned an error");
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
