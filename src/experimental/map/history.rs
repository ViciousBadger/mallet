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
    pub parent_key: Option<Id>,
    pub child_keys: Vec<Id>,
    pub change: Change,
}

pub fn new_timestamp() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Change {
    InitMap,
    Element { key: Id, delta: Delta },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Delta {
    Create { element: Element, content_key: Id },
    Modify { then: Element, now: Element },
    Remove { element: Element, content_key: Id },
}
