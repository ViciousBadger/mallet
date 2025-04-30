use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use avian3d::parry::utils::hashmap::HashMap;
use bevy::{asset::io::AssetSourceId, prelude::*};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::util::{Id, IdGen};

#[derive(Default, Serialize, Deserialize)]
pub struct ContentLib {
    asset_source_name: Option<String>,
    surfaces: HashMap<Id, Content<Surface>>,
    sounds: HashMap<Id, Content<Sound>>,
    // TODO: 3d models, ..??
}

pub struct ContentSource {
    asset_source_name: Option<String>,
    subpath: String,
}

impl ContentLib {
    pub fn get_surface(&self, id: &Id) -> Option<&Content<Surface>> {
        self.surfaces.get(id)
    }

    pub fn get_sound(&self, id: &Id) -> Option<&Content<Sound>> {
        self.sounds.get(id)
    }

    pub fn sync(&mut self, path: &Path, id_gen: &mut IdGen) {
        // Check for new files.
        info!("walk {:?}!", path);

        info!(
            "supported formats {:?}",
            bevy::image::ImageLoader::SUPPORTED_FORMATS
        );
        info!(
            "supported extensions {:?}",
            bevy::image::ImageLoader::SUPPORTED_FILE_EXTENSIONS
        );

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok().filter(|entry| entry.file_type().is_file()))
        {
            //let path = entry.path().strip_prefix(Path::new("assets/")).unwrap();
            let path = entry.path();
            match path.extension() {
                Some(val) if val == "png" => {
                    let hash = blake3::hash(&fs::read(path).unwrap());
                    let surf: Content<Surface> = Content {
                        path: path.to_owned(),
                        hash,
                        data: default(),
                    };
                    info!("new surface: {:?}", surf);
                    self.surfaces.insert(id_gen.generate(), surf);
                }
                _ => (),
            }
        }

        // TODO: Check for modified files.

        // TODO: Check for moved files.
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content<T> {
    pub path: PathBuf,
    pub hash: blake3::Hash,
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Surface {
    pub roughness: f32,
    pub reflectance: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sound;

impl Default for Surface {
    fn default() -> Self {
        Self {
            roughness: 1.0,
            reflectance: 0.0,
        }
    }
}

/// A content library for base_content
#[derive(Resource)]
pub struct BaseContent {
    pub lib: ContentLib,
    default_surface_id: Id,
}

impl BaseContent {
    pub fn default_surface(&self) -> &Content<Surface> {
        self.lib.surfaces.get(&self.default_surface_id).unwrap()
    }
}

fn init_base_content_lib(mut id_gen: ResMut<IdGen>, mut commands: Commands) {
    let lib_path = Path::new("assets/base_content.lib"); // save lib here to keep id's consistent
    let content_path = Path::new("assets/base_content/");

    let mut lib = if lib_path.is_file() {
        postcard::from_bytes(&std::fs::read(lib_path).unwrap()).unwrap()
        // ron::from_str(&std::fs::read_to_string(lib_path).unwrap()).unwrap()
    } else {
        ContentLib::default()
    };

    lib.sync(content_path, &mut id_gen);

    let file = File::create(lib_path).unwrap();
    postcard::to_io(&lib, file).unwrap();
    // std::fs::write(lib_path, ron::to_string(&lib).unwrap()).unwrap();

    commands.insert_resource(BaseContent {
        default_surface_id: *lib.surfaces.keys().next().unwrap(),
        lib,
    });
    //
    // TODO: save
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, init_base_content_lib);
}
