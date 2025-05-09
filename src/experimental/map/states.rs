use bevy::{platform::collections::HashMap, prelude::*};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    core::map::brush::Brush,
    experimental::map::{
        db::{Checksum, Db, Object, Typed, TBL_META, TBL_OBJECTS},
        elements::{ElemId, ElemMeta},
        StateSnapshot,
    },
    id::Id,
};

pub const TBL_STATES: TableDefinition<Id, Typed<State>> = TableDefinition::new("states");

#[derive(Serialize, Deserialize, Debug, Resource)]
pub struct State {
    pub elements: HashMap<Id, ElemState>,
    // snapshot should also store all Media used in the map, to be able to undo/redo media
    // add/delete/rename.. BUT not the media contents nor the element contents. those should be
    // stored separately and only deleted when all referring history snapshots are gone.. i guess..
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElemState {
    pub meta: Checksum,
    pub params: Checksum,
}

impl ElemState {
    pub fn params(&self) -> &Checksum {
        &self.params
    }
}

impl State {
    pub fn empty() -> Self {
        Self {
            elements: HashMap::new(),
        }
    }
}

// Multiple typed statechange variants to allow parallel insertion
// (Generalize by element role i guess)
#[derive(Event, Debug)]
enum StateChange {
    SetMeta { id: Id, meta: Checksum },
    SetParams { id: Id, params: Checksum },
    // Removed id
}

fn sync_elems(
    db: Res<Db>,
    state: Res<State>,
    q_elems: Query<(&ElemId, &ElemMeta)>,
    mut changes: EventWriter<StateChange>,
) -> Result {
    info!("sync elems running");
    for (id, meta) in q_elems.iter() {
        let (new_checksum, new_meta) = Object::new_typed(meta);

        if state
            .elements
            .get(id.id_ref())
            .is_none_or(|existing| new_checksum != existing.meta)
        {
            db.begin_write()?
                .open_table(TBL_OBJECTS)?
                .insert(&new_checksum, &new_meta)?;
            changes.write(StateChange::SetMeta {
                id: **id,
                meta: new_checksum,
            });
            info!("element meta inserted {:?}", id);
        }
    }
    Ok(())
}

fn sync_brush(
    db: Res<Db>,
    state: Res<State>,
    q_brushes: Query<(&ElemId, &Brush)>,
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
            db.begin_write()?
                .open_table(TBL_OBJECTS)?
                .insert(&new_checksum, &new_brush)?;
            changes.write(StateChange::SetParams {
                id: **id,
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
            StateChange::SetMeta { id, meta } => {
                state
                    .elements
                    .entry(*id)
                    .and_modify(|elem| elem.meta = meta.clone())
                    .or_insert(ElemState {
                        meta: meta.clone(),
                        params: Checksum::nil(),
                    });
            }
            StateChange::SetParams { id, params } => {
                state
                    .elements
                    .entry(*id)
                    .and_modify(|elem| elem.params = params.clone())
                    .or_insert(ElemState {
                        meta: Checksum::nil(),
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
