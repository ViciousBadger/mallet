use avian3d::parry::utils::hashmap::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::Id;

#[derive(Default, Resource, Serialize, Deserialize)]
pub struct ContentLib {
    pub surfaces: HashMap<Id, Content<Surface>>,
    pub sounds: HashMap<Id, Content<Sound>>,
    // TODO: 3d models, ..??
}

impl ContentLib {
    pub fn get_surface(&self, id: &Id) -> Option<&Content<Surface>> {
        self.surfaces.get(id)
    }

    pub fn get_sound(&self, id: &Id) -> Option<&Content<Sound>> {
        self.sounds.get(id)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Content<T> {
    pub path: String,
    pub hash: blake3::Hash,
    pub data: T,
}

#[derive(Serialize, Deserialize)]
pub struct Surface {
    pub roughness: f32,
    pub reflectance: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Sound {
    pub asset_path: String,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
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
        &self.surfaces.values().next().unwrap().data
    }
}

pub fn plugin(app: &mut App) {}
