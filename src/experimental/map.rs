pub mod changes;
pub mod db;
pub mod elements;
pub mod history;
pub mod states;

use bevy::{
    ecs::schedule::ScheduleLabel, input::common_conditions::*, platform::collections::HashMap,
    prelude::*,
};
use thiserror::Error;

use crate::{
    core::map::brush::{Brush, BrushBounds},
    experimental::map::{
        changes::{
            Change, ChangeSet, CreateElem,
            NewElemId::{self, Generated},
            PendingChanges, UpdateElemInfo, UpdateElemParams,
        },
        db::{Db, Meta, Object, TBL_META, TBL_OBJECTS},
        elements::{AppRoles, ChangeBuilder, ElementId, ElementRoleRegistry, Info},
        history::{HistNode, TBL_HIST_NODES},
        states::TBL_STATES,
    },
    id::{Id, IdGen},
};

#[derive(Event)]
struct RestoreState {
    pub id: Id,
}

fn new_test_map(mut commands: Commands, mut id_gen: ResMut<IdGen>) -> Result {
    let db = Db::new();

    let mut restore: Option<Id> = None;

    if let Some(meta) = db
        .begin_read()?
        .open_table(TBL_META)
        .map(|table| table.get(()).unwrap_or(None).map(|guard| guard.value()))
        .unwrap_or(None)
    {
        info!("load map !!!");
        // Load the map
        let reader = db.begin_read()?;

        let hist_node = reader
            .open_table(TBL_HIST_NODES)?
            .get(meta.hist_node_id)?
            .unwrap()
            .value();
        restore = Some(hist_node.state_id);
    } else {
        // Write initial stuff
        let writer = db.begin_write()?;
        {
            let mut tbl_states = writer.open_table(states::TBL_STATES)?;
            let initial_state_id = id_gen.generate();
            tbl_states.insert(initial_state_id, states::State::default())?;

            // Initial history node
            let mut tbl_hist = writer.open_table(history::TBL_HIST_NODES)?;
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

            let mut tbl_meta = writer.open_table(TBL_META)?;
            tbl_meta.insert(
                (),
                Meta {
                    name: "test map".to_string(),
                    hist_node_id: initial_hist_id,
                },
            )?;
        }
        writer.commit()?;
    }

    commands.insert_resource(db);

    if let Some(id) = restore {
        commands.trigger(RestoreState { id });
    }
    Ok(())
}

fn new_thing(mut changes: ResMut<PendingChanges>) {
    changes.push_many(vec![
        CreateElem {
            id: Generated,
            info: Info {
                name: "first brush".to_string(),
            },
            params: Brush {
                bounds: BrushBounds {
                    start: Vec3::ZERO,
                    end: Vec3::ONE,
                },
            },
        },
        CreateElem {
            id: Generated,
            info: Info {
                name: "second brush".to_string(),
            },
            params: Brush {
                bounds: BrushBounds {
                    start: Vec3::ZERO,
                    end: Vec3::ONE,
                },
            },
        },
    ]);
    changes.push_single(CreateElem {
        id: Generated,
        info: Info {
            name: "third brush (in its own change set)".to_string(),
        },
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
    q_added_ids: Query<(&ElementId, Entity), Added<ElementId>>,
    mut q_removed_ids: RemovedComponents<ElementId>,
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
    let reader = world.resource::<Db>().begin_read()?;
    let meta = reader.open_table(TBL_META)?.get(())?.unwrap().value();
    let mut cur_hist = reader
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
    let new_state = world.remove_resource::<states::State>().unwrap();

    let in_scene = world.query::<&ElementId>().iter(world).len();
    info!(
        "total elements in state: {}, in scene: {}",
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
        cur_hist.child_ids.push(new_hist_id);
        tbl_hist.insert(meta.hist_node_id, cur_hist)?;
        tbl_hist.insert(
            new_hist_id,
            HistNode {
                timestamp: history::new_timestamp(),
                parent_id: Some(meta.hist_node_id),
                child_ids: Vec::new(),
                state_id: new_state_id,
            },
        )?;
    }
    writer.open_table(TBL_META)?.insert(
        (),
        Meta {
            hist_node_id: new_hist_id,
            ..meta
        },
    )?;
    writer.commit()?;
    info!(
        "created a new hist node {} for new state {}",
        new_hist_id, new_state_id
    );

    // TODO: update children of the ole node

    Ok(())
}

fn restore_state(trigger: Trigger<RestoreState>, world: &mut World) -> Result {
    info!("restoring state {}", trigger.id);

    let reader = world.resource::<Db>().begin_read()?;
    let state = reader
        .open_table(TBL_STATES)?
        .get(trigger.id)?
        .unwrap()
        .value();

    let objs = reader.open_table(TBL_OBJECTS)?;
    for (elem_id, elem) in state.elements {
        let info = objs.get(elem.info)?.unwrap().value().cast::<Info>();
        let params = objs.get(elem.params)?.unwrap().value();

        world.resource_scope(|world: &mut World, registry: Mut<ElementRoleRegistry>| {
            let builder = registry.roles.get(&elem.role.unwrap()).unwrap();

            if world.resource::<ElementLookup>().find(&elem_id).is_ok() {
                // Exists, update
                UpdateElemInfo {
                    elem_id,
                    new_info: info,
                }
                .apply_to_world(world);
                builder.build_update(elem_id, params).apply_to_world(world);
            } else {
                // Create
                builder
                    .build_create(NewElemId::Loaded(elem_id), info, params)
                    .apply_to_world(world);
            }
        });
    }
    //     RestoreElem {
    //         id: elem_id,
    //         meta,
    //         params: elem.params,
    //     }
    //     .apply_to_world(world);

    // TODO: Remove elems not in the state

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
    app.add_observer(restore_state);
    // app.register_map_element_role::<Brush>(BrushBuilder);
    app.init_resource::<ElementRoleRegistry>();
    app.register_map_element_role::<Brush>();
}

// pub struct BrushBuilder;
// impl RoleChangeBuilder for BrushBuilder {
//     fn build_create(&self, id: NewElemId, info: Info, raw_params: Object) -> Box<dyn Change> {
//         let params = raw_params.cast::<Brush>();
//         Box::new(CreateElem { id, info, params })
//     }
//
//     fn build_update(&self, elem_id: Id, raw_params: Object) -> Box<dyn Change> {
//         let new_params = raw_params.cast::<Brush>();
//         Box::new(UpdateElemParams {
//             elem_id,
//             new_params,
//         })
//     }
// }
