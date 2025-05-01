use std::path::{Path, PathBuf};

use bevy::{
    asset::io::{AssetSource, AssetSourceId, MissingAssetSourceError},
    prelude::*,
    utils::HashMap,
};

pub enum MapAssetSrc {
    Base,
    Map,
}

pub struct MapAssetSrcConf {
    asset_source_name: Option<String>,
    fs_root_path: PathBuf,
    subpath: PathBuf,
}

impl MapAssetSrcConf {
    pub fn asset_source<'a>(
        &self,
        server: &'a AssetServer,
    ) -> Result<&'a AssetSource, MissingAssetSourceError> {
        server.get_source(AssetSourceId::new(self.asset_source_name.as_ref()))
    }

    pub fn asset_path<'a>(&self, server: &'a AssetServer, path: &Path) -> Option<&Path> {
        todo!()
    }

    // pub fn file_path_root(&self, server: &AssetServer) -> &Path {
    //     let src = self.asset_source(server);
    //     todo!()
    // }
}

#[derive(Resource)]
pub struct MapAssetSoures(HashMap<MapAssetSrc, MapAssetSrcConf>);

pub fn plugin(app: &mut App) {}
