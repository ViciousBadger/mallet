use bevy::{
    asset::{self, AssetPath, AssetServer},
    prelude::*,
};
use color_eyre::eyre;

#[derive(Resource, Default)]
pub struct AssetStorer {
    pub pending: Vec<PendingStoreTask>,
}

pub struct PendingStoreTask {
    pub path: AssetPath<'static>,
    pub handle: UntypedHandle,
}

#[derive(Event)]
pub struct AssetStoreResult(eyre::Result<()>);

impl AssetStorer {
    pub fn store<A>(&mut self, handle: Handle<A>, path: AssetPath)
    where
        A: Asset,
    {
        self.pending.push(PendingStoreTask {
            path: path.into_owned(),
            handle: handle.untyped(),
        });
    }
}

fn start_pending(asset_server: Res<AssetServer>, mut asset_storer: ResMut<AssetStorer>) {
    for pending_task in asset_storer.pending.drain(..) {
        let source_id = pending_task.path.source();
        let source = asset_server.get_source(source_id).unwrap();
        let writer = source.writer();
    }
}

fn poll_tasks(mut asset_storer: ResMut<AssetStorer>) {}

fn test(mut asset_storer: ResMut<AssetStorer>) {}

pub fn plugin(app: &mut App) {
    app.init_resource::<AssetStorer>();
    app.add_systems(Startup, test);
    app.add_systems(Update, (start_pending, poll_tasks));
}
