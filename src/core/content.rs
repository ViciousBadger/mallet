use std::{
    any::{Any, TypeId},
    path::PathBuf,
};

use bevy::{
    ecs::define_label,
    prelude::*,
    utils::{HashMap, TypeIdMap},
};
use serde::{Deserialize, Serialize};

use crate::util::{Id, IdGen};

pub enum SourceKind {
    Base,
    Map,
}

#[derive(Resource, Default)]
pub struct ContentLib {
    sources: HashMap<SourceKind, ContentSource>,
    contents: HashMap<Id, UntypedContent>,
}

pub struct ContentSource {
    asset_source_name: Option<String>,
    subpath: PathBuf,
}

pub enum ContentGetError {
    Missing,
    WrongType,
}

// Caveat: since the actual content is behind a box pointer, it can't be densely packed (performance)
pub struct UntypedContent(Box<dyn Any + Send + Sync>);
impl UntypedContent {
    pub fn typed<C: 'static>(&self) -> Option<&Content<C>> {
        self.0.downcast_ref::<Content<C>>()
    }

    pub fn typed_mut<C: 'static>(&mut self) -> Option<&mut Content<C>> {
        self.0.downcast_mut::<Content<C>>()
    }
}

impl ContentLib {
    pub fn get<C: 'static>(&self, id: &Id) -> Option<&Content<C>> {
        self.contents
            .get(id)
            .and_then(|content| content.typed::<C>())
    }

    pub fn get_mut<C: 'static>(&mut self, id: &Id) -> Option<&mut Content<C>> {
        self.contents
            .get_mut(id)
            .and_then(|content| content.typed_mut::<C>())
    }

    pub fn insert<C: 'static + Send + Sync>(&mut self, id: Id, new_content: C) {
        self.contents
            .insert(id, UntypedContent(Box::new(new_content)));
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content<T> {
    pub path: PathBuf,
    pub hash: blake3::Hash,
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Material {
    pub roughness: f32,
    pub reflectance: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            roughness: 1.0,
            reflectance: 0.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sound;

fn sync_content(mut content_lib: ResMut<ContentLib>) {}

pub fn plugin(app: &mut App) {
    //app.add_systems(Startup, sync_content.in_set());
}

#[cfg(test)]
mod tests {
    use ulid::Ulid;

    use super::*;

    #[test]
    fn test_content_typecast() {
        let mut lib = ContentLib::default();

        let tex: Content<Material> = Content {
            path: "some_tex.png".into(),
            hash: blake3::hash(&[]),
            data: Material::default(),
        };

        let id = Id(Ulid::new());

        lib.insert(id, tex);

        let fetched = lib.get::<Material>(&id);
        assert!(fetched.is_some());
    }
}
