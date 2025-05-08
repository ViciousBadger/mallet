use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    experimental::map::{db::Postcard, elements::Element},
    id::Id,
};

pub const HIST_TABLE: TableDefinition<Id, Postcard<HistNode>> = TableDefinition::new("history");

#[derive(Serialize, Deserialize, Debug)]
pub struct HistNode {
    pub timestamp: i64,
    pub parent_id: Option<Id>,
    pub child_ids: Vec<Id>,
    pub snapshot_id: Id,
}

pub fn new_timestamp() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

pub const SNAPSHOT_TABLE: TableDefinition<Id, Postcard<Snapshot>> =
    TableDefinition::new("snapshots");

#[derive(Serialize, Deserialize, Debug)]
pub struct Snapshot {
    elements: Vec<Element>,
    // snapshot should also store all Media used in the map, to be able to undo/redo media
    // add/delete/rename.. BUT not the media contents nor the element contents. those should be
    // stored separately and only deleted when all referring history snapshots are gone.. i guess..
}

impl Snapshot {
    pub fn empty() -> Self {
        Snapshot {
            elements: Vec::new(),
        }
    }
}
