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

fn sync_elems(
    db: Res<Db>,
    mut state: ResMut<State>,
    q_elems: Query<(&ElemId, &ElemMeta)>,
) -> Result {
    info!("sync elems running");
    for (id, meta) in q_elems.iter() {
        if let Some(existing) = state.elements.get_mut(id.id_ref()) {
            // Compare and update
            // Can potentially optimize by caching checksum for every new/changed meta.
            let (new_checksum, new_meta) = Object::new_typed(meta);
            if new_checksum != existing.meta {
                let write_tx = db.begin_write()?;
                write_tx
                    .open_table(TBL_OBJECTS)?
                    .insert(&new_checksum, new_meta)?;
                // Point to new meta !
                existing.meta = new_checksum;
                info!("meta changed {:?}", id);
            }
        } else {
            // Insert new
            info!("insert {:?}", id);
            let write_tx = db.begin_write()?;

            let (meta_check, meta_obj) = Object::new_typed(meta);
            write_tx
                .open_table(TBL_OBJECTS)?
                .insert(&meta_check, meta_obj)?;
            write_tx.commit()?;

            state.elements.insert(
                **id,
                ElemState {
                    meta: meta_check,
                    params: Checksum::nil(),
                },
            );
        }
    }
    Ok(())
}

fn sync_brush(
    db: Res<Db>,
    mut state: ResMut<State>,
    q_brushes: Query<(&ElemId, &Brush)>,
) -> Result {
    info!("sync brush running");
    for (id, brush) in q_brushes.iter() {
        if let Some(existing) = state.elements.get_mut(id.id_ref()) {
            let (new_checksum, new_brush) = Object::new_typed(brush);
            if new_checksum != existing.params {
                let write_tx = db.begin_write()?;
                write_tx
                    .open_table(TBL_OBJECTS)?
                    .insert(&new_checksum, new_brush)?;
                existing.params = new_checksum;
                info!("brush changed {:?}", id);
            }
        }
    }
    Ok(())
}

pub fn plugin(app: &mut App) {
    app.add_systems(StateSnapshot, (sync_elems, sync_brush).chain());
}
