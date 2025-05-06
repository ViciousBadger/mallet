use std::str::from_utf8;

use bevy::prelude::*;
use sled::{Config, Db, Tree};

#[derive(Resource)]
struct MapDb {
    db: Db,
    meta: MapMeta,
}

const META_KEY: &[u8] = "meta".as_bytes();

struct MapMeta(Tree);

impl MapMeta {
    pub fn set_name(&self, name: &str) {
        self.0.insert("name", name).unwrap();
    }

    pub fn get_name(&self) -> String {
        String::from_utf8(self.0.get("name").unwrap().unwrap().to_vec()).unwrap()
    }
}

impl MapDb {
    pub fn new_temp() -> sled::Result<Self> {
        let db = Config::new().temporary(true).open()?;
        let meta = MapMeta(db.open_tree(META_KEY)?);
        meta.set_name("hello world");
        Ok(Self { db, meta })
        //Self(sled::confi)
    }

    // pub fn new(map_file: Path) -> Self {
    //     Self(
    //     )
    //
    // }
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
