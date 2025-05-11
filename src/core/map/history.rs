use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{core::db::Typed, id::Id};

pub const TBL_HIST_NODES: TableDefinition<Id, Typed<HistNode>> = TableDefinition::new("hist_nodes");

#[derive(Serialize, Deserialize, Debug)]
pub struct HistNode {
    pub timestamp: i64,
    pub parent_id: Option<Id>,
    pub child_ids: Vec<Id>,
    pub state_id: Id,
}

pub fn new_timestamp() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
