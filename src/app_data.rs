use bevy::{asset::io::AssetSourceBuilder, prelude::*};
use color_eyre::eyre::{self, Context, OptionExt};

pub fn plugin(app: &mut App) {
    let data_dir = determine_app_data_path().unwrap();
    app.register_asset_source(
        "data",
        AssetSourceBuilder::platform_default(&data_dir, None),
    )
    .insert_resource(AppDataPath(data_dir));
}

fn determine_app_data_path() -> eyre::Result<String> {
    let proj_dirs = directories::ProjectDirs::from("com", "badgerson", "mallet")
        .ok_or_eyre("Failed to determine OS data directories.")?;
    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir).wrap_err("Failed to create data directory for Mallet.")?;
    Ok(data_dir.to_str().ok_or_eyre("Empty data dir")?.to_string())
}

#[derive(Resource)]
pub struct AppDataPath(String);

impl AppDataPath {
    pub fn get(&self) -> &str {
        &self.0
    }
}
