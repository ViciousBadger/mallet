use std::str::from_utf8;

use bevy::prelude::*;
use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use sled::{Config, Db, Tree};

#[derive(Resource)]
struct MapDb {
    db: Db,
    meta: MapMeta,
}

const META_KEY: &[u8] = "meta".as_bytes();
const HIST_KEY: &[u8] = "history".as_bytes();

struct MapMeta(Tree);

pub fn try_block<T>(closure: impl FnOnce() -> eyre::Result<T>) -> Option<T> {
    match closure() {
        Ok(result) => Some(result),
        Err(err) => {
            error!("try block failed with error\n{}", err);
            None
        }
    }
}

impl MapMeta {
    pub fn set_name(&self, name: &str) {
        try_block(|| {
            self.0.insert("name", name)?;
            Ok(())
        });
    }

    pub fn get_name(&self) -> String {
        try_block(|| {
            Ok(String::from_utf8(
                self.0.get("name")?.unwrap_or_default().to_vec(),
            )?)
        })
        .unwrap_or_default()
    }
}

impl MapDb {
    pub fn new_temp() -> sled::Result<Self> {
        let db = Config::new().temporary(true).open()?;
        let meta = MapMeta(db.open_tree(META_KEY)?);
        meta.set_name("hello world");
        Ok(Self { db, meta })
    }
}

#[derive(Serialize, Deserialize)]
struct MapHistEntr {
    pub parent: u64,
    pub timestamp: i64,
    pub delta: MapDelta,
}

#[derive(Serialize, Deserialize)]
pub enum MapDelta {
    Init,
    AddNode {
        node_id: u64,
        content_id: u64,
    },
    ModifyNode {
        node_id: u64,
        content_id_before: u64,
        content_id_after: u64,
    },
    RemoveNode {
        node_id: u64,
        content_id_before: u64,
    },
}

fn test(mut commands: Commands) {
    let map = MapDb::new_temp().unwrap();
    let name = map.meta.get_name();
    info!("name is {}", name);
    commands.insert_resource(map);
}

fn flush_db(map_db: Res<MapDb>) {
    map_db.db.flush().unwrap();
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, test);
    app.add_systems(Last, flush_db.run_if(on_event::<AppExit>));
}
