use avian3d::parry::utils::hashmap::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::Id;

#[derive(Default, Resource, Serialize, Deserialize)]
pub struct ContentLib {
    pub surfaces: HashMap<Id, Surface>,
    pub sounds: HashMap<Id, Sound>,
    // TODO: 3d models, ..??
}

#[derive(Serialize, Deserialize)]
pub struct Surface {
    pub asset_path: String,
    pub roughness: f32,
    pub reflectance: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Sound {
    pub asset_path: String,
}

impl Surface {
    pub fn new(texture_asset_path: String) -> Self {
        Self {
            asset_path: texture_asset_path,
            roughness: 1.0,
            reflectance: 0.0,
        }
    }
}

#[derive(Deref)]
pub struct BaseContentLib(pub ContentLib);

impl BaseContentLib {
    pub fn new() {
        todo!()
    }

    pub fn default_surface(&self) -> &Surface {
        self.surfaces.values().next().unwrap()
    }
}
