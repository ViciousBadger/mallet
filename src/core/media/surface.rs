use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
