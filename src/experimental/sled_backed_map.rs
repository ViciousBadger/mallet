use std::{io::Read, marker::PhantomData, str::from_utf8};

use bevy::prelude::*;
use color_eyre::eyre;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Config, Db, Tree};
use thiserror::Error;

use crate::id::{Id, IdGen};

#[derive(Error, Debug)]
enum DbError {
    #[error("serialization/deserialization error")]
    Serialization(#[from] postcard::Error),
    #[error("database error")]
    Database(#[from] sled::Error),
}

struct TypedTree<T> {
    inner: Tree,
    phantom_data: PhantomData<T>,
}

impl<T> TypedTree<T> {
    pub fn new(inner: Tree) -> Self {
        Self {
            inner,
            phantom_data: PhantomData,
        }
    }
}

type DbResult<T> = Result<T, DbError>;

impl<T> TypedTree<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn insert(&self, key: &Id, value: &T) -> DbResult<()> {
        let serialized: Vec<u8> = postcard::to_stdvec::<T>(value)?;
        self.inner.insert(key.to_bytes(), serialized)?;
        Ok(())
    }

    pub fn get(&self, key: &Id) -> DbResult<Option<T>> {
        if let Some(result) = self.inner.get(key.to_bytes())? {
            Ok(postcard::from_bytes(&result)?)
        } else {
            Ok(None)
        }
    }
    pub fn delete(&self, key: &Id) -> DbResult<bool> {
        Ok(self.inner.remove(key.to_bytes())?.is_some())
    }
}

#[derive(Resource)]
struct MapDb {
    backing: Db,
}

const HIST_KEY: &[u8] = &[0];
const STATE_KEY: &[u8] = &[1];

impl MapDb {
    pub fn new_temp() -> sled::Result<MapDb> {
        //let db = Config::new().temporary(true).open()?;
        let db = sled::open("test.db")?;
        info!("opened");
        Ok(MapDb { backing: db })
    }

    fn history(&self) -> DbResult<MapHistory> {
        Ok(MapHistory(TypedTree::new(
            self.backing.open_tree(HIST_KEY)?,
        )))
    }

    fn main_state(&self) -> DbResult<MapState> {
        Ok(MapState(TypedTree::new(self.backing.open_tree(STATE_KEY)?)))
    }

    fn snapshot_state(&self, id: &Id) -> DbResult<MapState> {
        let full_key = [STATE_KEY, &id.to_bytes()].concat();
        Ok(MapState(TypedTree::new(self.backing.open_tree(full_key)?)))
    }
}

struct MapHistory(TypedTree<MapHistoryNode>);

impl MapHistory {
    pub fn push(&self, id_gen: &mut IdGen, map_hist_entr: MapHistoryNode) {
        let id = id_gen.generate();
        self.0.insert(&id, &map_hist_entr).unwrap();
    }
}

#[derive(Serialize, Deserialize)]
struct MapHistoryNode {
    pub parent: u64,
    pub timestamp: i64,
    pub action: MapAction,
}

#[derive(Serialize, Deserialize)]
pub enum MapAction {
    StateSnapshot(u64),
    Delta {
        node_id: u64,
        node_kind: NodeKind,
        delta: MapDelta,
    },
}

#[derive(Serialize, Deserialize)]
pub enum MapDelta {
    Create { content_key: u64 },
    Modify { before_key: u64, after_key: u64 },
    Remove { content_key: u64 },
}

struct MapState(TypedTree<MapStateNode>);

#[derive(Serialize, Deserialize)]
struct MapStateNode {
    pub name: String,
    pub node_kind: NodeKind,
    pub content_key: u64,
}

#[derive(Serialize, Deserialize)]
pub enum NodeKind {
    Brush = 0,
    Light = 1,
}

fn test(mut commands: Commands) {
    let map = MapDb::new_temp().unwrap();
    commands.insert_resource(map);
}

fn flush_db(map_db: Res<MapDb>) {
    map_db.backing.flush().unwrap();
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, test);
    app.add_systems(Last, flush_db.run_if(on_event::<AppExit>));
}
