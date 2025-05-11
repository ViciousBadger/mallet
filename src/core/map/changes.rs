use bevy::prelude::*;

use crate::{
    core::{
        db::{Db, TBL_META},
        map::{
            elements::{ElementId, Info, Role},
            history::{new_timestamp, HistNode, UpdateCurrentHistNode, TBL_HIST_NODES},
            states::{MapState, StateSnapshot, TBL_STATES},
            ElementLookup,
        },
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
    pub id_mode: CreateId,
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
            .id_mode
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

pub fn apply_pending_changes(mut pending_changes: ResMut<PendingChanges>, mut commands: Commands) {
    let change_sets: Vec<ChangeSet> = pending_changes.drain(..).collect();

    for change_set in change_sets {
        commands.run_system_cached_with(try_apply_change_set, change_set);
    }
}

fn try_apply_change_set(change_set: In<ChangeSet>, world: &mut World) {
    if let Err(err) = world.run_system_cached_with(apply_change_set, change_set.0) {
        error!("Failed to apply change set: {}", err);
        world.remove_resource::<MapState>();
    }
}

fn apply_change_set(change_set: In<ChangeSet>, world: &mut World) -> Result {
    // TODO: Could likely be split into multiple event-triggered systems. Triggers cant return values tho

    let change_set = change_set.0;
    // Step 1: apply to world.
    for change in change_set.changes {
        // Quirk: apply_to_world could in theory take ownership over the "change" and prevent a
        // clone, but it's impossible while the change is a Box<dyn Change>
        change.apply_to_world(world);
    }

    // Step 2: create a state resource and run the snapshot schedule. other systems will fill out state.
    let reader = world.resource::<Db>().begin_read()?;
    let meta = reader.open_table(TBL_META)?.get(())?.unwrap().value();
    let cur_hist = reader
        .open_table(TBL_HIST_NODES)?
        .get(meta.hist_node_id)?
        .unwrap()
        .value();
    let cur_state = reader
        .open_table(TBL_STATES)?
        .get(cur_hist.state_id)?
        .unwrap()
        .value();
    drop(reader);

    world.insert_resource(cur_state);
    world.run_schedule(StateSnapshot);
    world.flush();

    // Step 3: insert new state into db and create a history node.
    let writer = world.resource::<Db>().begin_write()?;
    let new_state_id = world.resource_mut::<IdGen>().generate();
    let new_state = world.remove_resource::<MapState>().unwrap();

    let in_scene = world.query::<&ElementId>().iter(world).len();
    info!(
        "Total elements in state: {}, in scene: {}",
        new_state.elements.len(),
        in_scene
    );

    writer
        .open_table(TBL_STATES)?
        .insert(new_state_id, new_state)?;
    let new_hist_id = world.resource_mut::<IdGen>().generate();
    {
        // Update children on the current history node first
        let mut tbl_hist = writer.open_table(TBL_HIST_NODES)?;
        let updated_child_ids = cur_hist
            .child_ids
            .iter()
            .copied()
            .chain(std::iter::once(new_hist_id));
        tbl_hist.insert(
            meta.hist_node_id,
            HistNode {
                child_ids: updated_child_ids.collect(),
                ..cur_hist
            },
        )?;

        // New hist node as child of current
        tbl_hist.insert(
            new_hist_id,
            HistNode {
                timestamp: new_timestamp(),
                parent_id: Some(meta.hist_node_id),
                child_ids: Vec::new(),
                state_id: new_state_id,
            },
        )?;
    }
    writer.commit()?;
    world.trigger(UpdateCurrentHistNode(new_hist_id));

    // TODO: Here might be a good place to clean up the history, preventing file bloat.

    Ok(())
}

#[derive(Event)]
pub struct PushTempChange(Box<dyn Change>);
impl PushTempChange {
    pub fn new(change: impl Change + 'static) -> Self {
        Self(Box::new(change))
    }
}

fn push_temp_change(trigger: Trigger<PushTempChange>, world: &mut World) -> Result {
    trigger.0.apply_to_world(world);
    Ok(())
}

pub fn plugin(app: &mut App) {
    app.add_observer(push_temp_change);
    app.add_systems(Last, apply_pending_changes);
}
