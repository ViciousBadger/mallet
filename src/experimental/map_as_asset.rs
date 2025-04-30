use bevy::{
    asset::{
        io::{Reader, Writer},
        saver::{AssetSaver, SavedAsset},
        AssetLoader, AsyncWriteExt, LoadContext,
    },
    prelude::*,
};
use postcard::to_stdvec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
struct MapFile {
    pub asdf: i32,
    pub dependency: String,
}

#[derive(Asset, TypePath, Debug)]
struct Map {
    pub asdf: i32,
    pub dependency: Handle<Image>,
}

#[derive(Default)]
struct MapLoader;

#[derive(Debug, Error)]
enum MapLoadError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Postcard(#[from] postcard::Error),
}

impl AssetLoader for MapLoader {
    type Asset = Map;

    type Settings = ();

    type Error = MapLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Map, MapLoadError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let file: MapFile = postcard::from_bytes(&bytes)?;
        Ok(Map {
            asdf: file.asdf,
            dependency: load_context.loader().load(file.dependency),
        })
    }
}

#[derive(Default)]
struct MapSaver;

#[derive(Debug, Error)]
enum MapSaveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Postcard(#[from] postcard::Error),
}

impl AssetSaver for MapSaver {
    type Asset = Map;

    type Settings = ();

    type OutputLoader = MapLoader;

    type Error = MapSaveError;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, Self::Asset>,
        _settings: &Self::Settings,
    ) -> Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error> {
        let map = asset.get();
        let mapfile = MapFile {
            asdf: map.asdf,
            dependency: "".to_string(),
        };
        let bytes = to_stdvec(&mapfile)?;
        writer.write_all(&bytes).await?;
        Ok(())
    }
}
