use std::str::from_utf8;

use bevy::prelude::*;
use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use sled::{Config, Db, Tree};

#[derive(Resource)]
struct MapDb {
    db: Db,
}

const HIST_KEY: &[u8] = "history".as_bytes();

impl MapDb {}

impl MapDb {
    pub fn new_temp() -> sled::Result<MapDb> {
        let db = Config::new().temporary(true).open()?;
        Ok(MapDb { db })
    }
}

#[derive(Serialize, Deserialize)]
struct MapHistEntr {
    pub parent: u64,
    pub timestamp: i64,
    pub kind: MapHistEntrKind,
}

#[derive(Serialize, Deserialize)]
pub enum MapHistEntrKind {
    Init,
    NodeModify {
        id: u64,
        type_id: u64,
        state_before: Option<u64>,
        state_after: Option<u64>,
    },
}

fn test(mut commands: Commands) {
    let map = MapDb::new_temp().unwrap();
    commands.insert_resource(map);
}

fn flush_db(map_db: Res<MapDb>) {
    map_db.db.flush().unwrap();
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, test);
    app.add_systems(Last, flush_db.run_if(on_event::<AppExit>));
}
