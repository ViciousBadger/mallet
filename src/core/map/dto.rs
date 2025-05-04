use bevy::utils::default;
use serde::{Deserialize, Serialize};

use crate::{
    core::map::{brush::Brush, MapHistory, MapNodeMeta},
    editor::EditorContext,
};

#[derive(Deserialize)]
pub struct MapDe {
    pub history: MapHistory,
    pub editor_context: EditorContext,
    pub brushes: Vec<MapNodeDe<Brush>>,
}

#[derive(Deserialize)]
pub struct MapNodeDe<T> {
    pub meta: MapNodeMeta,
    pub node: T,
}

impl MapDe {
    pub fn from_bytes(bytes: &[u8]) -> Result<MapDe, ron::Error> {
        Ok(ron::de::from_bytes(bytes)?)
    }
}

#[derive(Serialize)]
pub struct MapSer<'a> {
    pub history: &'a MapHistory,
    pub editor_context: &'a EditorContext,

    pub brushes: Vec<MapNodeSer<'a, Brush>>,
}

#[derive(Serialize)]
pub struct MapNodeSer<'a, T> {
    pub meta: &'a MapNodeMeta,
    pub node: &'a T,
}

impl MapSer<'_> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, ron::Error> {
        Ok(ron::ser::to_string_pretty(self, default())?.into())
    }
}
