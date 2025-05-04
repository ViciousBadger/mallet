use bevy::utils::default;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    core::media::{surface::Surface, MediaCollection, MediaMeta, MediaSrc},
    util::Id,
};

#[derive(Deserialize)]
pub struct MediaLibDe {
    pub surfaces: Vec<MediaDe<Surface>>,
}

impl MediaLibDe {
    // TODO: Read directly from file instead of intermediary byte array. (zero-copy)
    pub fn from_bytes(bytes: &[u8]) -> Result<MediaLibDe, ron::Error> {
        Ok(ron::de::from_bytes(bytes)?)
    }
}

#[derive(Deserialize)]
pub struct MediaDe<T> {
    pub id: Id,
    pub meta: MediaMeta,
    pub content: T,
}

#[derive(Serialize)]
pub struct MediaLibSer<'a> {
    pub surfaces: Vec<MediaSer<'a, Surface>>,
}

#[derive(Serialize)]
pub struct MediaSer<'a, T> {
    pub id: &'a Id,
    pub meta: &'a MediaMeta,
    pub content: &'a T,
}

impl MediaLibSer<'_> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, ron::Error> {
        Ok(ron::ser::to_string_pretty(self, default())?.into())
    }
}

impl<T> MediaCollection<T> {
    pub fn collect_dto_vec<'a>(&'a self, source: &'a MediaSrc) -> Vec<MediaSer<'a, T>> {
        self.0
            .iter()
            .filter(move |(_, sourced)| &sourced.source == source)
            .map(|(id, live)| MediaSer {
                id,
                meta: &live.meta,
                content: &live.content,
            })
            .collect_vec()
    }

    pub fn insert_from_dto_vec(&mut self, source: &MediaSrc, dto: Vec<MediaDe<T>>) {
        for de in dto {
            self.insert(de.id, *source, de.meta, de.content)
        }
    }
}
