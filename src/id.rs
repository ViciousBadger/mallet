use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use ulid::{serde::ulid_as_u128, Ulid};

/// Persistent identifier.
#[derive(
    Deref,
    Debug,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Component,
)]
pub struct Id(#[serde(with = "ulid_as_u128")] pub Ulid);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Resource)]
pub struct IdGen(ulid::Generator);

impl Default for IdGen {
    fn default() -> Self {
        IdGen(ulid::Generator::new())
    }
}

impl IdGen {
    pub fn generate(&mut self) -> Id {
        Id(self.0.generate().unwrap())
    }
}
