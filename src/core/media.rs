mod dto;
pub mod surface;
mod watch;

use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{io::AssetSourceId, AssetPath},
    image::{
        ImageAddressMode, ImageLoader, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
    },
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{
    core::media::{
        dto::{MediaLibDe, MediaLibSer},
        surface::{Surface, SurfaceHandles},
    },
    id::{Id, IdGen},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct MediaSources(HashMap<MediaSrc, MediaSrcConf>);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct MediaMeta {
    pub path: PathBuf,
    pub hash: blake3::Hash,
    pub dupe_idx: Option<u32>,
}

impl MediaMeta {
    pub fn diff(&self, other: &MediaMeta) -> MediaDiff {
        let same_path = self.path == other.path;
        let same_hash = self.hash == other.hash;

        // if same_hash && self.dupe_idx != other.dupe_idx {
        //     info!("ignoring a dupe {:?}", self.dupe_idx);
        //     return MediaDiff::Unrelated;
        // }

        //let same_dupe_idx = self.dupe_idx == other.dupe_idx;
        match (same_path, same_hash) {
            (true, true) => MediaDiff::Identical,
            (true, false) => MediaDiff::SamePath,
            (false, true) => MediaDiff::SameContent,
            (false, false) => MediaDiff::Unrelated,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum MediaDiff {
    Identical,
    SamePath,
    SameContent,
    Unrelated,
}

pub struct Media<T> {
    pub source: MediaSrc,
    pub meta: MediaMeta,
    pub content: T,
}

#[derive(Resource)]
pub struct MediaCollection<T>(HashMap<Id, Media<T>>);

// This is nice, but since the collections are separate types there is some boilerplate when e.g. purging.
impl<T> Default for MediaCollection<T> {
    fn default() -> Self {
        MediaCollection(HashMap::new())
    }
}

impl<T> MediaCollection<T> {
    pub fn get(&self, id: &Id) -> Option<&Media<T>> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &Id) -> Option<&mut Media<T>> {
        self.0.get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &Media<T>)> {
        self.0.iter()
    }

    pub fn insert(&mut self, id: Id, source: MediaSrc, meta: MediaMeta, media: T) {
        self.0.insert(
            id,
            Media {
                source,
                meta,
                content: media,
            },
        );
    }

    pub fn remove(&mut self, id: &Id) {
        self.0.remove(id);
    }

    pub fn purge_source(&mut self, source: &MediaSrc) {
        self.0.retain(|_, sm| &sm.source != source);
    }
}

fn init_base_media(mut sources: ResMut<MediaSources>, mut commands: Commands) {
    sources.insert(
        MediaSrc::Base,
        MediaSrcConf::new(None, "base_content", "assets/base_content"),
    );
    commands.trigger(MediaLoad(MediaSrc::Base));
}

#[derive(Event, Deref, DerefMut)]
pub struct MediaLoad(MediaSrc);

fn media_load(
    trigger: Trigger<MediaLoad>,
    media_sources: Res<MediaSources>,
    mut surfaces: ResMut<MediaCollection<Surface>>,
    mut commands: Commands,
) {
    let src: MediaSrc = **trigger;
    let src_conf = media_sources
        .get(&src)
        .expect("Synced source should be configured");

    let path = src_conf.fs_base_path.parent().unwrap().join("media.db");
    if path.exists() {
        let bytes = std::fs::read(&path).expect("File read fail");
        let dto = MediaLibDe::from_bytes(&bytes).expect("Deserialization fail");

        surfaces.insert_from_dto_vec(&src, dto.surfaces);

        info!("loaded media collection from {:?}", path);
    } else {
        info!("no media collection at {:?}", path);
    }
    commands.trigger(MediaSync(src));
}

fn surface_image_settings(settings: &mut ImageLoaderSettings) {
    *settings = ImageLoaderSettings {
        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            // mag_filter: ImageFilterMode::Linear,
            // min_filter: ImageFilterMode::Linear,
            ..default()
        }),
        ..default()
    }
}

#[derive(Event, Deref, DerefMut)]
pub struct MediaSync(MediaSrc);

fn media_sync(
    trigger: Trigger<MediaSync>,
    asset_server: Res<AssetServer>,
    media_sources: Res<MediaSources>,
    mut id_gen: ResMut<IdGen>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut surfaces: ResMut<MediaCollection<Surface>>,
    mut commands: Commands,
) {
    let src: MediaSrc = **trigger;
    let src_conf = media_sources
        .get(&src)
        .expect("Synced source should be configured");

    let base_path = src_conf.fs_base_path.as_path();
    let mut scanned: HashSet<PathBuf> = HashSet::new();
    for entry in WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| entry.ok().filter(|entry| entry.file_type().is_file()))
    {
        let full_path = entry.path();
        let media_path = entry.path().strip_prefix(base_path).unwrap();
        let meta = MediaMeta {
            path: media_path.to_path_buf(),
            hash: blake3::hash(&std::fs::read(full_path).unwrap()),
            dupe_idx: None,
        };
        if let Some(ext) = full_path.extension().and_then(|osstr| osstr.to_str()) {
            if ImageLoader::SUPPORTED_FILE_EXTENSIONS.contains(&ext) {
                let diffs = surfaces
                    .0
                    .iter()
                    .filter(|(_, media)| media.source == src)
                    //.filter(|(_, media)| !scanned.contains(&media.meta.path))
                    .map(|(id, media)| (*id, meta.diff(&media.meta)))
                    .collect_vec();

                #[derive(PartialEq)]
                enum Act {
                    CreateFresh,
                    CreateDupe(u32),
                    NoOp,
                }

                info!("-------- scanning: {:?}", meta.path);

                let act = if diffs.iter().any(|(_, diff)| diff == &MediaDiff::Identical) {
                    info!("already registered and no change: {:?}", meta.path);
                    Act::NoOp
                } else if diffs.iter().all(|(_, diff)| diff == &MediaDiff::Unrelated) {
                    info!("new media: {:?}", meta.path);
                    Act::CreateFresh
                } else if let Some(modified) =
                    diffs.iter().find(|(_, diff)| diff == &MediaDiff::SamePath)
                {
                    //let mut surf = surfaces.get_mut(&modified.0).unwrap();
                    asset_server.reload(src_conf.asset_path(media_path));
                    info!("modified: {}", modified.0);
                    Act::NoOp
                } else {
                    // let clones = diffs
                    //     .iter()
                    //     .filter(|(id, diff)| diff == &MediaDiff::SameContent);
                    Act::NoOp
                };

                if act != Act::NoOp {
                    let meta = if let Act::CreateDupe(dupe_idx) = act {
                        MediaMeta {
                            path: meta.path.clone(),
                            hash: meta.hash,
                            dupe_idx: Some(dupe_idx),
                        }
                    } else {
                        meta
                    };

                    let id = id_gen.generate();
                    let mut surf = Surface::default();

                    let img = asset_server.load_with_settings(
                        src_conf.asset_path(media_path),
                        surface_image_settings,
                    );

                    let std_mat = standard_materials.add(StandardMaterial {
                        perceptual_roughness: surf.roughness,
                        reflectance: surf.reflectance,
                        base_color_texture: Some(img.clone()),
                        ..default()
                    });
                    surf.handles = SurfaceHandles {
                        albedo_texture: img,
                        std_material: std_mat,
                    };
                    surfaces.insert(id, src, meta, surf);
                }
            }
        } else {
            info!("{:?} has no file extension. Ignoring", full_path);
        }
        scanned.insert(media_path.to_path_buf());
    }

    let to_remove = surfaces
        .iter()
        .filter_map(|(id, media)| {
            (!src_conf.file_path(&media.meta.path).exists())
                .then_some((media.meta.path.to_path_buf(), *id))
        })
        .collect_vec();

    for (path, id) in to_remove {
        info!("remove {:?}, path missing", path);
        surfaces.remove(&id);
    }

    commands.trigger(MediaSave(src));
}

#[derive(Event, Deref, DerefMut)]
pub struct MediaSave(MediaSrc);

fn media_save(
    trigger: Trigger<MediaSave>,
    media_sources: Res<MediaSources>,
    surfaces: Res<MediaCollection<Surface>>,
) {
    let src: MediaSrc = **trigger;
    let src_conf = media_sources
        .get(&src)
        .expect("Synced source should be configured");

    // NOTE: can't be async atm because of the bororw.
    // Fortunately, serialization is fast.
    let dto = MediaLibSer {
        surfaces: surfaces.collect_dto_vec(&src),
    };
    let bytes = dto.to_bytes().expect("Serialization fail");

    let path = src_conf.fs_base_path.parent().unwrap().join("media.db");
    info!("saving media collection to {:?}", path);
    let mut file = File::create(path).expect("File creation fail");
    //postcard::to_io(&lib, file).unwrap();
    file.write_all(&bytes).expect("File write fail");
}

pub fn plugin(app: &mut App) {
    app.add_plugins(watch::plugin);
    app.init_resource::<MediaSources>();
    app.init_resource::<MediaCollection<Surface>>();
    app.add_observer(media_load);
    app.add_observer(media_sync);
    app.add_observer(media_save);
    app.add_systems(Startup, init_base_media);
}
