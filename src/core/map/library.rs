use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize)]
pub struct MapAssetLib {}

#[derive(Serialize, Deserialize)]
pub struct MapMaterial {}
