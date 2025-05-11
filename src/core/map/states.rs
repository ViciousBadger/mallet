use bevy::{ecs::schedule::ScheduleLabel, platform::collections::HashMap, prelude::*};
use color_eyre::eyre::eyre;
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        db::{Checksum, Db, EnsureExists, Object, Typed, TBL_META, TBL_OBJECTS},
        map::{
            changes::{Change, CreateId, UpdateElemInfo},
            elements::{ElementId, ElementRoleRegistry, Info, Role},
            get_current_hist_node, get_current_state,
            history::TBL_HIST_NODES,
            ElementLookup,
        },
    },
    id::Id,
};

pub const TBL_STATES: TableDefinition<Id, Typed<MapState>> = TableDefinition::new("states");

#[derive(Serialize, Deserialize, Debug, Resource, Default)]
pub struct MapState {
    pub elements: HashMap<Id, ElementState>,
    // snapshot should also store all Media used in the map, to be able to undo/redo media
    // add/delete/rename.. BUT not the media contents nor the element contents. those should be
    // stored separately and only deleted when all referring history snapshots are gone.. i guess..
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElementState {
    // TODO: role shoudl probably not be optional, forced by the decoupling here.
    /// used to insert/update the correct kind of element when restoring a state
    pub role: Option<u64>,
    pub info: Checksum,
    pub params: Checksum,
}

// Multiple typed statechange variants to allow parallel insertion
// (Generalize by element role i guess)
#[derive(Event, Debug)]
pub enum StateChange {
    SetInfo { id: Id, info: Checksum },
    SetParams { id: Id, role: u64, params: Checksum },
    // Removed id
}

fn sync_elems(
    db: Res<Db>,
    state: Res<MapState>,
    q_elems: Query<(&ElementId, &Info)>,
    mut changes: EventWriter<StateChange>,
) -> Result {
    for (id, info) in q_elems.iter() {
        let (new_checksum, new_info) = Object::new_typed(info);

        if state
            .elements
            .get(id.id_ref())
            .is_none_or(|existing| new_checksum != existing.info)
        {
            let writer = db.begin_write()?;
            writer
                .open_table(TBL_OBJECTS)?
                .insert(&new_checksum, &new_info)?;
            writer.commit()?;
            changes.write(StateChange::SetInfo {
                id: **id,
                info: new_checksum,
            });
        }
    }
    Ok(())
}

pub fn sync_params<R>(
    db: Res<Db>,
    state: Res<MapState>,
    q_elem_params: Query<(&ElementId, &R)>,
    mut changes: EventWriter<StateChange>,
) -> Result
where
    R: Role,
{
    for (id, params) in q_elem_params.iter() {
        let (new_checksum, new_params_obj) = Object::new_typed(params);

        if state
            .elements
            .get(id.id_ref())
            .is_none_or(|existing| new_checksum != existing.params)
        {
            let writer = db.begin_write()?;
            writer
                .open_table(TBL_OBJECTS)?
                .insert(&new_checksum, &new_params_obj)?;
            writer.commit()?;
            changes.write(StateChange::SetParams {
                id: **id,
                role: R::id_hash(),
                params: new_checksum,
            });
        }
    }
    Ok(())
}

fn apply_state_changes(mut changes: EventReader<StateChange>, mut state: ResMut<MapState>) {
    for change in changes.read() {
        match change {
            StateChange::SetInfo { id, info: meta } => {
                state
                    .elements
                    .entry(*id)
                    .and_modify(|elem| elem.info = meta.clone())
                    .or_insert(ElementState {
                        role: None,
                        info: meta.clone(),
                        params: Checksum::nil(),
                    });
            }
            StateChange::SetParams { id, role, params } => {
                state
                    .elements
                    .entry(*id)
                    .and_modify(|elem| {
                        elem.role = Some(*role);
                        elem.params = params.clone();
                    })
                    .or_insert(ElementState {
                        role: Some(*role),
                        info: Checksum::nil(),
                        params: params.clone(),
                    });
            }
        }
    }
}

#[derive(Event)]
pub struct RestoreState {
    pub id: Id,
    pub fresh_map: bool,
}

fn restore_state(trigger: Trigger<RestoreState>, world: &mut World) -> Result {
    let reader = world.resource::<Db>().begin_read()?;

    // Need to grab current state from db for easier comparisons..
    let cur_hist_node = get_current_hist_node(&reader)?;
    let states = reader.open_table(TBL_STATES)?;
    let cur_state = states.get(cur_hist_node.state_id)?.unwrap().value();
    let state_to_restore = states.get(trigger.id)?.ensure_exists()?.value();

    let objs = reader.open_table(TBL_OBJECTS)?;
    for (elem_id, elem) in state_to_restore.elements.iter() {
        let info = objs.get(&elem.info)?.unwrap().value().cast::<Info>();
        let params = objs.get(&elem.params)?.unwrap().value();

        // TODO: shared code for handling a single element?
        // quirk: here it checks for checksum changes before changing, but in the "checkout" thing it should always run either update or create. perhaps a "force" bool?
        // also, db stuff should be handled separately for each call.. because getting state for
        // ever elem would be stupid slow
        world.resource_scope(|world: &mut World, registry: Mut<ElementRoleRegistry>| {
            let builder = registry.roles.get(&elem.role.unwrap()).unwrap();
            if let Some(cur_elem) = (!trigger.fresh_map)
                .then(|| cur_state.elements.get(elem_id))
                .flatten()
            {
                if elem.info != cur_elem.info {
                    UpdateElemInfo {
                        elem_id: *elem_id,
                        new_info: info,
                    }
                    .apply_to_world(world);
                }
                if elem.params != cur_elem.params {
                    builder.build_update(*elem_id, params).apply_to_world(world);
                }
            } else {
                // Create
                builder
                    .build_create(CreateId::Loaded(*elem_id), info, params)
                    .apply_to_world(world);
            }
        });
    }

    // Remove elems not in the state
    // NOTE: This could also use a query over ElementId, idk which is faster.
    let to_despawn: Vec<Entity> = world
        .resource::<ElementLookup>()
        .iter()
        .flat_map(|(id, entity)| (!state_to_restore.elements.contains_key(id)).then_some(*entity))
        .collect();

    for entity in to_despawn {
        world.despawn(entity);
    }

    Ok(())
}

#[derive(Event)]
/// Like a Git file checkout, updates an element in the world to match its current state in the db.
pub struct CheckoutElement {
    pub id: Id,
}

fn checkout_element(trigger: Trigger<CheckoutElement>, world: &mut World) -> Result {
    let reader = world.resource::<Db>().begin_read()?;
    let state = get_current_state(&reader)?;
    let objs = reader.open_table(TBL_OBJECTS)?;

    let elem = state
        .elements
        .get(&trigger.id)
        .ok_or(eyre!("Missing element!!"))?;
    let info = objs.get(&elem.info)?.unwrap().value().cast::<Info>();
    let params = objs.get(&elem.params)?.unwrap().value();

    world.resource_scope(|world: &mut World, registry: Mut<ElementRoleRegistry>| {
        let builder = registry.roles.get(&elem.role.unwrap()).unwrap();
        UpdateElemInfo {
            elem_id: trigger.id,
            new_info: info,
        }
        .apply_to_world(world);
        builder
            .build_update(trigger.id, params)
            .apply_to_world(world);
    });

    Ok(())
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateSnapshot;

pub fn plugin(app: &mut App) {
    app.add_schedule(Schedule::new(StateSnapshot));
    app.add_event::<StateChange>();
    app.add_observer(restore_state);
    app.add_observer(checkout_element);
    app.add_systems(
        StateSnapshot,
        (
            sync_elems.in_set(SyncState),
            apply_state_changes.after(SyncState),
        ),
    );
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyncState;
