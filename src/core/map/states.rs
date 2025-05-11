use bevy::{platform::collections::HashMap, prelude::*};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        db::{Checksum, Db, Object, Typed, TBL_OBJECTS},
        map::{
            elements::{brush::Brush, ElementId, Info, Role},
            StateSnapshot,
        },
    },
    id::Id,
};

pub const TBL_STATES: TableDefinition<Id, Typed<State>> = TableDefinition::new("states");

#[derive(Serialize, Deserialize, Debug, Resource, Default)]
pub struct State {
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
enum StateChange {
    SetInfo { id: Id, info: Checksum },
    SetParams { id: Id, role: u64, params: Checksum },
    // Removed id
}

fn sync_elems(
    db: Res<Db>,
    state: Res<State>,
    q_elems: Query<(&ElementId, &Info)>,
    mut changes: EventWriter<StateChange>,
) -> Result {
    info!("sync elems running");
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
            info!("element info inserted {:?}", id);
        }
    }
    Ok(())
}

fn sync_brush(
    db: Res<Db>,
    state: Res<State>,
    q_brushes: Query<(&ElementId, &Brush)>,
    mut changes: EventWriter<StateChange>,
) -> Result {
    info!("sync brush running");
    for (id, brush) in q_brushes.iter() {
        let (new_checksum, new_brush) = Object::new_typed(brush);

        if state
            .elements
            .get(id.id_ref())
            .is_none_or(|existing| new_checksum != existing.params)
        {
            let writer = db.begin_write()?;
            writer
                .open_table(TBL_OBJECTS)?
                .insert(&new_checksum, &new_brush)?;
            writer.commit()?;
            changes.write(StateChange::SetParams {
                id: **id,
                role: Brush::id_hash(),
                params: new_checksum,
            });
            info!("brush inserted {:?}", id);
        }
    }
    Ok(())
}

fn apply_state_changes(mut changes: EventReader<StateChange>, mut state: ResMut<State>) {
    for change in changes.read() {
        info!("apply state change: {:?}", change);
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

pub fn plugin(app: &mut App) {
    app.add_event::<StateChange>();
    app.add_systems(
        StateSnapshot,
        ((sync_elems, sync_brush), apply_state_changes).chain(),
    );
}
