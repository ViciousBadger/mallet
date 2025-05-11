use bevy::utils::default;
use serde::{Deserialize, Serialize};

use crate::{core::map::Map, editor::EditorContext};

#[derive(Deserialize)]
pub struct MapDe {
    pub map: Map,
    pub editor_context: EditorContext,
}

impl MapDe {
    pub fn from_bytes(bytes: &[u8]) -> Result<MapDe, ron::Error> {
        Ok(ron::de::from_bytes(bytes)?)
    }
}

#[derive(Serialize)]
pub struct MapSer<'a> {
    pub map: &'a Map,
    pub editor_context: &'a EditorContext,
}

impl MapSer<'_> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, ron::Error> {
        Ok(ron::ser::to_string_pretty(self, default())?.into())
    }
}
