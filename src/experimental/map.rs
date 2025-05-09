pub mod changes;
pub mod db;
pub mod elements;
pub mod history;
pub mod states;

use bevy::{
    ecs::schedule::ScheduleLabel,
    input::common_conditions::{input_just_pressed, input_pressed},
    platform::collections::HashMap,
    prelude::*,
};
use thiserror::Error;

use crate::{
    core::map::brush::{Brush, BrushBounds},
    experimental::map::{
        changes::{ChangeSet, Create, PendingChanges},
        db::{Db, Meta, TBL_META},
        elements::ElemId,
        history::{HistNode, TBL_HIST_NODES},
        states::{State, TBL_STATES},
    },
    id::{Id, IdGen},
};

fn new_test_map(mut commands: Commands, mut id_gen: ResMut<IdGen>) -> Result {
    let map = Db::new_temp();
    let tx = map.begin_write()?;

    {
        // Initial empty state

        let mut tbl_states = tx.open_table(states::TBL_STATES)?;
        let initial_state_id = id_gen.generate();
        tbl_states.insert(initial_state_id, states::State::empty())?;

        // Initial history node
        let mut tbl_hist = tx.open_table(history::TBL_HIST_NODES)?;
        let initial_hist_id = id_gen.generate();
        tbl_hist.insert(
            initial_hist_id,
            HistNode {
                timestamp: history::new_timestamp(),
                parent_id: None,
                child_ids: Vec::default(),
                state_id: initial_state_id,
            },
        )?;

        let mut tbl_meta = tx.open_table(TBL_META)?;
        tbl_meta.insert(
            (),
            Meta {
                name: "a map".to_string(),
                hist_node_id: initial_hist_id,
            },
        )?;
    }
    tx.commit()?;

    commands.insert_resource(map);
    Ok(())
}

fn new_thing(mut changes: ResMut<PendingChanges>) {
    changes.push_many(vec![
        Create {
            name: "first brush".to_string(),
            params: Brush {
                bounds: BrushBounds {
                    start: Vec3::ZERO,
                    end: Vec3::ONE,
                },
            },
        },
        Create {
            name: "second brush".to_string(),
            params: Brush {
                bounds: BrushBounds {
                    start: Vec3::ZERO,
                    end: Vec3::ONE,
                },
            },
        },
    ]);
    changes.push_single(Create {
        name: "third brush (in its own change set)".to_string(),
        params: Brush {
            bounds: BrushBounds {
                start: Vec3::ZERO,
                end: Vec3::ONE,
            },
        },
    });

    info!("ok, pushed some changes.");
}
fn undo() {}
fn redo() {}

#[derive(Resource, Default)]
pub struct ElementLookup(HashMap<Id, Entity>);

#[derive(Error, Debug)]
#[error("No entity found for {}", self.0)]
pub struct ElementLookupError(Id);

impl ElementLookup {
    pub fn find(&self, element_id: &Id) -> Result<Entity, ElementLookupError> {
        self.0
            .get(element_id)
            .copied()
            .ok_or(ElementLookupError(*element_id))
    }
}

fn track_element_ids(
    q_added_ids: Query<(&ElemId, Entity), Added<ElemId>>,
    mut q_removed_ids: RemovedComponents<ElemId>,
    mut lookup: ResMut<ElementLookup>,
) {
    for (id, entity) in q_added_ids.iter() {
        lookup.0.insert(**id, entity);
    }

    for entity in q_removed_ids.read() {
        lookup.0.retain(|_, e| *e != entity);
    }
}

pub fn apply_pending_changes(mut pending_changes: ResMut<PendingChanges>, mut commands: Commands) {
    let change_sets: Vec<ChangeSet> = pending_changes.drain(..).collect();

    if !change_sets.is_empty() {
        info!("collected {} change sets", change_sets.len());
    }

    for change_set in change_sets {
        commands.run_system_cached_with(try_apply_change_set, change_set);
    }
}

fn try_apply_change_set(change_set: In<ChangeSet>, world: &mut World) {
    if let Err(err) = world.run_system_cached_with(apply_change_set, change_set.0) {
        error!("Failed to apply change set: {}", err);
        //maybe remove any residual State resource
    }
}

fn apply_change_set(change_set: In<ChangeSet>, world: &mut World) -> Result {
    info!("apply change set: {:?}", change_set);
    let change_set = change_set.0;
    // Step 1: apply to world.
    for change in change_set.changes {
        // Quirk: apply_to_world could in theory take ownership over the "change" and prevent a
        // clone, but it's impossible while the change is a Box<dyn Change>
        change.apply_to_world(world);
    }

    // Step 2: create a state resource and run the snapshot schedule. other systems will fill out state.
    let read_tx = world.resource::<Db>().begin_read()?;
    let meta = read_tx.open_table(TBL_META)?.get(())?.unwrap().value();
    let cur_hist = read_tx
        .open_table(TBL_HIST_NODES)?
        .get(meta.hist_node_id)?
        .unwrap()
        .value();
    let cur_state = read_tx
        .open_table(TBL_STATES)?
        .get(cur_hist.state_id)?
        .unwrap()
        .value();
    drop(read_tx);

    world.insert_resource(cur_state);
    world.run_schedule(StateSnapshot);
    world.flush();

    // Step 3: insert new state into db and create a history node.
    let write_tx = world.resource::<Db>().begin_write()?;
    let new_state_id = world.resource_mut::<IdGen>().generate();
    let new_state = world.remove_resource::<states::State>().unwrap();
    info!("total elements in state: {}", new_state.elements.len());
    write_tx
        .open_table(TBL_STATES)?
        .insert(new_state_id, new_state)?;
    let new_hist_id = world.resource_mut::<IdGen>().generate();
    write_tx.open_table(TBL_HIST_NODES)?.insert(
        new_hist_id,
        HistNode {
            timestamp: history::new_timestamp(),
            parent_id: Some(meta.hist_node_id),
            child_ids: Vec::new(),
            state_id: new_state_id,
        },
    )?;
    write_tx.open_table(TBL_META)?.insert(
        (),
        Meta {
            hist_node_id: new_hist_id,
            ..meta
        },
    )?;
    write_tx.commit()?;
    info!(
        "created a new hist node {} for new state {}",
        new_hist_id, new_state_id
    );
    Ok(())
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateSnapshot;

pub fn plugin(app: &mut App) {
    app.add_schedule(Schedule::new(StateSnapshot));
    app.add_plugins(states::plugin);
    app.init_resource::<IdGen>();
    app.init_resource::<ElementLookup>();
    app.init_resource::<PendingChanges>();
    app.add_systems(Startup, new_test_map);
    app.add_systems(
        Update,
        (
            new_thing.run_if(input_pressed(KeyCode::KeyF)),
            track_element_ids,
        ),
    );
    app.add_systems(Last, apply_pending_changes);
}
