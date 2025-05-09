use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    experimental::map::db::{Checksum, Typed},
    id::Id,
};

pub const TBL_STATES: TableDefinition<Id, Typed<State>> = TableDefinition::new("states");

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    elements: Vec<ElemState>,
    // snapshot should also store all Media used in the map, to be able to undo/redo media
    // add/delete/rename.. BUT not the media contents nor the element contents. those should be
    // stored separately and only deleted when all referring history snapshots are gone.. i guess..
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElemState {
    pub id: Id,
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
            elements: Vec::new(),
        }
    }
}
