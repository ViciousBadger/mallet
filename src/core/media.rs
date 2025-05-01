use std::{
    fs::File,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{io::AssetSourceId, AssetPath},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoader, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    prelude::*,
    utils::HashMap,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::util::{Id, IdGen};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Media<C> {
    pub meta: MediaMeta,
    pub content: C,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaMeta {
    pub path: PathBuf,
    pub hash: blake3::Hash,
}

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

pub struct SourcedMedia<C> {
    pub source: MediaSrc,
    pub media: Media<C>,
}

#[derive(Resource)]
pub struct MediaCollection<C>(HashMap<Id, SourcedMedia<C>>);

// This is nice, but since the collections are separate types there is some boilerplate when e.g. purging.
impl<C> Default for MediaCollection<C> {
    fn default() -> Self {
        MediaCollection(HashMap::new())
    }
}

impl<C> MediaCollection<C> {
    pub fn get(&self, id: &Id) -> Option<&Media<C>> {
        self.0.get(id).map(|sourced| &sourced.media)
    }

    pub fn get_mut(&mut self, id: &Id) -> Option<&mut Media<C>> {
        self.0.get_mut(id).map(|sourced| &mut sourced.media)
    }

    pub fn insert(&mut self, id: Id, source: MediaSrc, media: Media<C>) {
        self.0.insert(id, SourcedMedia { source, media });
    }

    pub fn remove(&mut self, id: &Id) {
        self.0.remove(id);
    }

    pub fn purge_source(&mut self, source: &MediaSrc) {
        self.0.retain(|_, sm| &sm.source != source);
    }

    pub fn collect_for_storage<'a>(&'a self, source: &MediaSrc) -> Vec<(&'a Id, &'a Media<C>)> {
        self.0
            .iter()
            .filter(|(_, sourced)| &sourced.source == source)
            .map(|(id, sourced_media)| (id, &sourced_media.media))
            .collect_vec()
    }

    pub fn insert_from_storage(&mut self, source: MediaSrc, to_insert: Vec<(Id, Media<C>)>) {
        for (id, media) in to_insert {
            self.insert(id, source, media);
        }
    }
}

// TODO: Good way of handling e.g. textures for normals.
// - Combine hash of diffuse and normal and have an optional asset handle..?
//  - handle should not be serialized..
// - Normals are a special kind of media tied to its Material (diffuse) by filename..?

#[derive(Event, Deref, DerefMut)]
pub struct MediaSync(MediaSrc);

#[derive(Event, Deref, DerefMut)]
pub struct MediaSave(MediaSrc);

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

fn init_base_media(mut sources: ResMut<MediaSources>, mut sync_events: EventWriter<MediaSync>) {
    sources.insert(
        MediaSrc::Base,
        MediaSrcConf::new(None, "base_content", "assets/base_content"),
    );
    sync_events.send(MediaSrc::Base.into());
}

#[derive(Deserialize)]
pub struct LoadedMedia {
    pub surfaces: Media<Surface>,
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

fn media_sync(
    asset_server: Res<AssetServer>,
    media_sources: Res<MediaSources>,
    mut id_gen: ResMut<IdGen>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut surfaces: ResMut<MediaCollection<Surface>>,
    mut sync_events: EventReader<MediaSync>,
    mut save_events: EventWriter<MediaSave>,
) {
    for src_to_sync in sync_events.read().map(|event| &event.0).unique() {
        let src_conf = media_sources
            .get(src_to_sync)
            .expect("Synced source should be configured");

        let base_path = src_conf.fs_base_path.as_path();
        for entry in WalkDir::new(base_path)
            .into_iter()
            .filter_map(|entry| entry.ok().filter(|entry| entry.file_type().is_file()))
        {
            let full_path = entry.path();
            dbg!(full_path);
            let media_path = entry.path().strip_prefix(base_path).unwrap();
            dbg!(media_path);
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

                    let media = Media {
                        meta,
                        content: surf,
                    };
                    dbg!(&media);
                    surfaces.insert(id, *src_to_sync, media);
                }
            } else {
                info!("{:?} has no file extension. Ignoring", full_path);
            }
        }
        save_events.send((*src_to_sync).into());
    }
}

#[derive(Serialize)]
pub struct StorableMedia<'a> {
    pub surfaces: Vec<(&'a Id, &'a Media<Surface>)>,
}

// NOTE: This only makes sense for base assets lol unless the map media should be its own file outside of map??? (maybe good idea)
fn media_save(
    media_sources: Res<MediaSources>,
    surfaces: Res<MediaCollection<Surface>>,
    mut save_events: EventReader<MediaSave>,
) {
    for src in save_events.read().map(|event| &event.0).unique() {
        let src_conf = media_sources
            .get(src)
            .expect("Synced source should be configured");

        let store = StorableMedia {
            surfaces: surfaces.collect_for_storage(src),
        };
        let path = src_conf.fs_base_path.join("media.db");
        info!("saving media collection to {:?}", path);
        let file = File::create(path).unwrap();
        postcard::to_io(&store, file).unwrap();
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MediaSources>();
    app.init_resource::<MediaCollection<Surface>>();
    app.add_event::<MediaSync>();
    app.add_event::<MediaSave>();
    app.add_systems(Startup, init_base_media);
    app.add_systems(PreUpdate, (media_sync, media_save.after(media_sync)));
}
