use bevy::math::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Serialize, Deserialize)]
pub struct Map {
    pub nodes: Vec<MapNode>,
}

#[derive(Serialize, Deserialize)]
pub struct MapNode {
    pub id: Ulid,
    pub name: String,
    pub transform: Mat4,
    pub kind: MapNodeKind,
}

impl MapNode {
    pub fn new(kind: MapNodeKind) -> Self {
        let id = Ulid::new();
        let name = format!("{}-{}", id, kind.name());
        let transform = Mat4::IDENTITY;
        Self {
            id,
            name,
            transform,
            kind,
        }
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
