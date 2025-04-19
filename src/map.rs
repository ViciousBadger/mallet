use bevy::math::Vec3;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
};
use ulid::Ulid;

use crate::util::IdGen;

#[derive(Serialize, Deserialize)]
pub struct Map {
    pub nodes: BTreeSet<MapNode>,
}

#[derive(Serialize, Deserialize)]
pub struct MapNode {
    pub id: Ulid,
    pub name: String,
    pub kind: MapNodeKind,
}

impl Hash for MapNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for MapNode {}
impl PartialEq for MapNode {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Ord for MapNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for MapNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl MapNode {
    pub fn new(id_gen: &mut IdGen, kind: MapNodeKind) -> Self {
        let id = id_gen.generate();
        let name = format!("{}-{}", id, kind.name());
        Self { id, name, kind }
    }
}

#[derive(Serialize, Deserialize)]
pub enum MapNodeKind {
    Brush(Brush),
}

impl MapNodeKind {
    pub fn name(&self) -> &'static str {
        match self {
            MapNodeKind::Brush(_) => "Brush",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Brush {
    pub start: Vec3,
    pub end: Vec3,
}
