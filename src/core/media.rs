pub mod surface;
mod watch;

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{io::AssetSourceId, AssetPath},
    image::{
        ImageAddressMode, ImageLoader, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
    },
    prelude::*,
    utils::HashMap,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{core::media::surface::Surface, util::{Id, IdGen}};

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

// TODO: Like with mapnodes, media can be in generic form
// when saved in a resource and in an enum form when stored.
// (maybe dumb?)
//
pub trait Media {
    fn as_ref_kind<'a>(&'a self) -> MediaRefKind<'a>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaMeta {
    pub path: PathBuf,
    pub hash: blake3::Hash,
}

impl MediaMeta {
    pub fn same_but_moved_or_modified(&self, other: &MediaMeta) -> bool {
        self.path == other.path || self.hash == other.hash
    }
}

pub struct LiveMedia<M: Media> {
    pub source: MediaSrc,
    pub meta: MediaMeta,
    pub media: M,
}

#[derive(Serialize)]
pub struct SavableMediaLib<'a>(Vec<MediaRef<'a>>);

#[derive(Debug, Serialize)]
pub struct MediaRef<'a> {
    id: &'a Id,
    meta: &'a MediaMeta,
    kind: MediaRefKind<'a>,
}

#[derive(Debug, Serialize)]
pub enum MediaRefKind<'a> {
    Surface(&'a Surface),
}


#[derive(Deserialize)]
pub struct LoadedMediaLib(Vec<LoadedMedia>);

#[derive(Debug, Deserialize)]
pub struct LoadedMedia {
    id: Id,
    meta: MediaMeta,
    kind: LoadedMediaKind,
}

#[derive(Debug, Deserialize)]
pub enum LoadedMediaKind {
    Surface(Surface),
}

#[derive(Resource)]
pub struct MediaCollection<M: Media>(HashMap<Id, LiveMedia<M>>);

// This is nice, but since the collections are separate types there is some boilerplate when e.g. purging.
impl<M: Media> Default for MediaCollection<M> {
    fn default() -> Self {
        MediaCollection(HashMap::new())
    }
}

impl<M: Media> MediaCollection<M> {
    pub fn get(&self, id: &Id) -> Option<&M> {
        self.0.get(id).map(|sourced| &sourced.media)
    }

    pub fn get_mut(&mut self, id: &Id) -> Option<&mut M> {
        self.0.get_mut(id).map(|sourced| &mut sourced.media)
    }

    pub fn insert(&mut self, id: Id, source: MediaSrc, meta: MediaMeta, media: M) {
        self.0.insert(id, LiveMedia { source, meta, media });
    }

    pub fn remove(&mut self, id: &Id) {
        self.0.remove(id);
    }

    pub fn purge_source(&mut self, source: &MediaSrc) {
        self.0.retain(|_, sm| &sm.source != source);
    }

    pub fn collect_for_storage<'a>(&'a self, source: &'a MediaSrc) -> impl Iterator<Item = MediaRef<'a>>   {
        self.0
            .iter()
            .filter(move |(_, sourced)| &sourced.source == source)
            .map(|(id, live)| MediaRef {
                id,
                meta: &live.meta,
                kind: live.media.as_ref_kind(),
            })
    }
}

// TODO: Good way of handling e.g. textures for normals.
// - Combine hash of diffuse and normal and have an optional asset handle..?
//  - handle should not be serialized..
// - Normals are a special kind of media tied to its Material (diffuse) by filename..?

impl From<MediaSrc> for MediaSync {
    fn from(value: MediaSrc) -> Self {
        Self(value)
    }
}

impl From<MediaSrc> for MediaSave {
    fn from(value: MediaSrc) -> Self {
        Self(value)
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
        let bytes = std::fs::read(&path).unwrap();
        let loaded: LoadedMediaLib = postcard::from_bytes(&bytes).unwrap();
        for LoadedMedia {id, meta, kind} in loaded.0 {
            match kind {
                LoadedMediaKind::Surface(surface) => surfaces.insert(id, src, meta, surface),
            }
        }
        // surfaces.insert_from_storage(src, loaded.surfaces);
        info!("loaded media collection from {:?}", path);
        //let loaded = postcard::from_bytes
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
    for entry in WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| entry.ok().filter(|entry| entry.file_type().is_file()))
    {
        let full_path = entry.path();
        let media_path = entry.path().strip_prefix(base_path).unwrap();
        let meta = MediaMeta {
            path: media_path.to_path_buf(),
            hash: blake3::hash(&std::fs::read(full_path).unwrap()),
        };
        let id = id_gen.generate();
        if let Some(ext) = full_path.extension().and_then(|osstr| osstr.to_str()) {
            if ImageLoader::SUPPORTED_FILE_EXTENSIONS.contains(&ext) {
                let mut surf = Surface::default();
                let std_mat = standard_materials.add(StandardMaterial {
                    perceptual_roughness: surf.roughness,
                    reflectance: surf.reflectance,
                    base_color_texture: Some(asset_server.load_with_settings(
                        src_conf.asset_path(media_path),
                        surface_image_settings,
                    )),
                    ..default()
                });
                surf.handle = std_mat;

                // let media = Media {
                //     meta,
                //     content: surf,
                // };
                // dbg!(&media);
                surfaces.insert(id, src, meta, surf);
            }
        } else {
            info!("{:?} has no file extension. Ignoring", full_path);
        }
    }
    commands.trigger(MediaSave(src));
}

// #[derive(Serialize)]
// pub struct StorableMedia<'a> {
//     pub surfaces: Vec<(&'a Id, &'a Media<Surface>)>,
// }

#[derive(Event, Deref, DerefMut)]
pub struct MediaSave(MediaSrc);

// NOTE: This only makes sense for base assets lol unless the map media should be its own file outside of map??? (maybe good idea)
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
    let lib = SavableMediaLib(
        surfaces.collect_for_storage(&src).collect_vec()
    );

    let path = src_conf.fs_base_path.parent().unwrap().join("media.db");
    info!("saving media collection to {:?}", path);
    let file = File::create(path).unwrap();
    postcard::to_io(&lib, file).unwrap();
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
