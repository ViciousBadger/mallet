use std::any::TypeId;

use bevy::{
    asset::{
        self,
        saver::{AssetSaver, ErasedAssetSaver},
        AssetPath, AssetServer,
    },
    prelude::*,
    utils::TypeIdMap,
};
use color_eyre::eyre;
use itertools::Itertools;

#[derive(Resource, Default)]
pub struct AssetStorer {
    pub savers: TypeIdMap<Box<dyn ErasedAssetSaver>>,
    pub pending: Vec<PendingStoreTask>,
}

pub struct PendingStoreTask {
    pub path: AssetPath<'static>,
    pub handle: UntypedHandle,
}

#[derive(Event)]
pub struct AssetStoreResult(eyre::Result<()>);

impl AssetStorer {
    pub fn register_saver<S: AssetSaver>(&mut self, saver: S) {
        let type_name = core::any::type_name::<S>();
        let saved_asset_type = TypeId::of::<S::Asset>();
        let saved_asset_type_name = core::any::type_name::<S::Asset>();

        self.savers.insert(saved_asset_type, Box::new(saver));
    }

    pub fn store<A: Asset>(&mut self, handle: Handle<A>, path: AssetPath) {
        self.pending.push(PendingStoreTask {
            path: path.into_owned(),
            handle: handle.untyped(),
        });
    }
}

fn start_pending(asset_server: Res<AssetServer>, mut asset_storer: ResMut<AssetStorer>) {
    for task in asset_storer.pending.drain(..).collect_vec() {
        let source_id = task.path.source();
        let source = asset_server.get_source(source_id).unwrap();
        let writer = source.writer();

        let asset_type = task.handle.type_id();
        let saver = asset_storer.savers.get(&asset_type).unwrap();

        let task = async move || {
            info!("save");
            // let result = saver.save(writer).await;
            // dbg!(result);
        };
    }
}

fn poll_tasks(mut asset_storer: ResMut<AssetStorer>) {}

fn test(mut asset_storer: ResMut<AssetStorer>) {}

pub trait AssetStoreApp {
    fn register_asset_saver<S: AssetSaver>(&mut self, saver: S) -> &mut Self;
}

impl AssetStoreApp for App {
    fn register_asset_saver<S: AssetSaver>(&mut self, saver: S) -> &mut Self {
        self
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<AssetStorer>();
    app.add_systems(Startup, test);
    app.add_systems(Update, (start_pending, poll_tasks));
}
