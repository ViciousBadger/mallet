use std::marker::PhantomData;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Config, Db, Tree};
use thiserror::Error;

use crate::{
    core::map::brush::{Brush, BrushBounds},
    id::{Id, IdGen},
};

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
const CONTENT_KEY: &[u8] = &[2];

impl MapDb {
    pub fn new_temp() -> sled::Result<MapDb> {
        // let db = Config::new().temporary(true).open()?;
        let db = Config::new().path("test.db").use_compression(true).open()?;
        // let db = sled::open("test.db")?;
        info!("opened");
        Ok(MapDb { backing: db })
    }

    fn history(&self) -> DbResult<TypedTree<MapHistoryNode>> {
        Ok(TypedTree::new(self.backing.open_tree(HIST_KEY)?))
    }

    fn main_state(&self) -> DbResult<TypedTree<MapStateNode>> {
        Ok(TypedTree::new(self.backing.open_tree(STATE_KEY)?))
    }

    fn snapshot_state(&self, id: &Id) -> DbResult<TypedTree<MapStateNode>> {
        let full_key = [STATE_KEY, &id.to_bytes()].concat();
        Ok(TypedTree::new(self.backing.open_tree(full_key)?))
    }

    fn node_content<T>(&self, kind: NodeKind) -> DbResult<NodeContent<T>> {
        Ok(NodeContent(TypedTree::new(
            self.backing.open_tree(kind.to_node_content_key())?,
        )))
    }
}

#[derive(Serialize, Deserialize)]
struct MapHistoryNode {
    pub parent: Option<Id>,
    pub timestamp: i64,
    pub action: MapAction,
}

#[derive(Serialize, Deserialize)]
pub enum MapAction {
    StateSnapshot(Id),
    Delta {
        node_id: Id,
        node_kind: NodeKind,
        delta: MapDelta,
    },
}

#[derive(Serialize, Deserialize)]
pub enum MapDelta {
    Create { content_key: Id },
    Modify { before_key: Id, after_key: Id },
    Remove { content_key: Id },
}

#[derive(Serialize, Deserialize)]
struct MapStateNode {
    pub name: String,
    pub node_kind: NodeKind,
    pub content_key: Id,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(u8)]
pub enum NodeKind {
    Brush = 0,
    Light = 1,
}

impl NodeKind {
    fn to_node_content_key(self) -> Vec<u8> {
        [CONTENT_KEY, &[self as u8]].concat()
    }
}

fn new_test_map(mut commands: Commands) {
    let map = MapDb::new_temp().unwrap();
    commands.insert_resource(map);
}

fn flush_db(map_db: Res<MapDb>) {
    // Flushes the main Tree which is always open afaik.
    // Other trees are flushed after they are dropped in each system :)
    map_db.backing.flush().unwrap();
}

struct NodeContent<T>(TypedTree<T>);

fn push_test(map_db: Res<MapDb>, mut id_gen: ResMut<IdGen>) -> Result {
    let hist = map_db.history()?;

    let new_node_id = id_gen.generate();
    let new_node_kind = NodeKind::Brush;

    // Insert the node content
    let brush_content = map_db.node_content::<Brush>(new_node_kind)?;
    let new_node_content_id = id_gen.generate();

    let new_brush = Brush {
        bounds: BrushBounds {
            start: Vec3::NEG_ONE,
            end: Vec3::ONE,
        },
    };

    brush_content.0.insert(&new_node_content_id, &new_brush)?;

    // Create history entry
    let new_hist_id = id_gen.generate();
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

    let new_node = MapHistoryNode {
        parent: None,
        timestamp,
        action: MapAction::Delta {
            node_id: new_node_id,
            node_kind: new_node_kind,
            delta: MapDelta::Create {
                content_key: new_node_content_id,
            },
        },
    };
    hist.insert(&new_hist_id, &new_node)?;

    // Create state entry in main state
    // Should probably happen in a separate system that applies history entries.
    // also, needs to be propagated to the game world.
    let state = map_db.main_state()?;
    let new_state_node = MapStateNode {
        name: "a brush".to_string(),
        node_kind: new_node_kind,
        content_key: new_node_content_id,
    };
    state.insert(&new_node_id, &new_state_node)?;

    info!("yey done!");

    Ok(())
}

fn debuggy(map_db: Res<MapDb>) -> Result {
    let tree_names = map_db.backing.tree_names();
    //let tree_names = tree_names.into_iter().map(|bytes| bytes);
    dbg!(&tree_names);
    Ok(())
}

pub fn plugin(app: &mut App) {
    app.init_resource::<IdGen>();
    app.add_systems(Startup, new_test_map);
    app.add_systems(
        Update,
        (
            push_test.run_if(input_just_pressed(KeyCode::KeyF)),
            debuggy.run_if(input_just_pressed(KeyCode::KeyD)),
        ),
    );
    app.add_systems(Last, flush_db.run_if(on_event::<AppExit>));
}
