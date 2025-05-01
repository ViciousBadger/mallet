use std::path::{Path, PathBuf};

use bevy::{
    asset::{
        io::{AssetSource, AssetSourceId, MissingAssetSourceError},
        AssetPath,
    },
    prelude::*,
    utils::HashMap,
};

use crate::app_data::AppDataPath;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum MediaSrc {
    Base,
    Map,
}

pub struct MediaSrcConf {
    asset_source_name: Option<String>,
    /// Base path to use within the asset source
    asset_base_path: PathBuf,
    /// Base path to use on real file system
    fs_base_path: PathBuf,
}

impl MediaSrcConf {
    // pub fn asset_source<'a>(
    //     &self,
    //     server: &'a AssetServer,
    // ) -> Result<&'a AssetSource, MissingAssetSourceError> {
    //     server.get_source(AssetSourceId::new(self.asset_source_name.as_ref()))
    // }

    pub fn new(
        asset_source_name: Option<String>,
        asset_base_path: impl Into<PathBuf>,
        fs_base_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            asset_source_name,
            asset_base_path: asset_base_path.into(),
            fs_base_path: fs_base_path.into(),
        }
    }

    pub fn asset_path(&self, media_path: &Path) -> AssetPath {
        AssetPath::from_path(&self.asset_base_path.join(media_path))
            .with_source(AssetSourceId::new(self.asset_source_name.as_ref()))
            .into_owned()
    }

    pub fn file_path(&self, media_path: &Path) -> PathBuf {
        self.fs_base_path.join(media_path)
    }

    // pub fn asset_path<'a>(&self, server: &'a AssetServer, media_path: &Path) -> &Path {
    //
    // }

    // pub fn file_path_root(&self, server: &AssetServer) -> &Path {
    //     let src = self.asset_source(server);
    //     todo!()
    // }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct MediaSources(HashMap<MediaSrc, MediaSrcConf>);

fn init_base_media(mut sources: ResMut<MediaSources>) {
    sources.insert(
        MediaSrc::Base,
        MediaSrcConf::new(None, "base_content", "assets/base_content"),
    );
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MediaSources>();
}
