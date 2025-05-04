use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// TODO: Good way of handling e.g. textures for normals.
// - Combine hash of diffuse and normal and have an optional asset handle..?
//  - handle should not be serialized..
// - Normals are a special kind of media tied to its Material (diffuse) by filename..?

// Idea: put Surface stuff here and use dependency injection
// to have the Surfaces collection only exist when the plugin is loaded.
// Caveat: saving and loading the media lib has to be more complex to account
// for different sets of media types...

#[derive(Debug, Serialize, Deserialize)]
pub struct Surface {
    pub roughness: f32,
    pub reflectance: f32,
    #[serde(skip)]
    pub handle: Handle<StandardMaterial>,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            roughness: 1.0,
            reflectance: 0.0,
            handle: default(),
        }
    }
}
