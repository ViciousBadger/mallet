use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use serde::{Deserialize, Serialize};
use sled::{Config, Db};
use thiserror::Error;

use crate::{
    core::map::brush::{Brush, BrushBounds},
    id::{Id, IdGen},
};

type DbResult<T> = Result<T, DbError>;

#[derive(Error, Debug)]
enum DbError {
    #[error("serialization/deserialization error")]
    Serialization(#[from] postcard::Error),
    #[error("database error")]
    Database(#[from] sled::Error),
}

type TypedTree<T> = typed_sled::Tree<Id, T>;

// struct TypedTree<T> {
//     inner: Tree,
//     phantom_data: PhantomData<T>,
// }
//
// impl<T> TypedTree<T> {
//     pub fn new(inner: Tree) -> Self {
//         Self {
//             inner,
//             phantom_data: PhantomData,
//         }
//     }
// }

// impl<T> TypedTree<T>
// where
//     T: Serialize + DeserializeOwned,
// {
//     pub fn insert(&self, key: &Id, value: &T) -> DbResult<()> {
//         let serialized: Vec<u8> = postcard::to_stdvec::<T>(value)?;
//         self.inner.insert(key.to_bytes(), serialized)?;
//         Ok(())
//     }
//
//     pub fn get(&self, key: &Id) -> DbResult<Option<T>> {
//         if let Some(result) = self.inner.get(key.to_bytes())? {
//             Ok(postcard::from_bytes(&result)?)
//         } else {
//             Ok(None)
//         }
//     }
//
//     pub fn delete(&self, key: &Id) -> DbResult<bool> {
//         Ok(self.inner.remove(key.to_bytes())?.is_some())
//     }
// }

#[derive(Resource)]
struct MapDb {
    backing: Db,
}

const HIST_KEY: &str = "hist";
const STATE_KEY: &str = "node_state";
const CONTENT_KEY: &str = "node_content";

impl MapDb {
    pub fn new_temp() -> sled::Result<MapDb> {
        // let db = Config::new().temporary(true).open()?;
        let db = Config::new()
            .path("test.db")
            .use_compression(false)
            .open()?;
        // let db = sled::open("test.db")?;
        info!("opened");
        Ok(MapDb { backing: db })
    }

    fn history(&self) -> TypedTree<MapHistoryNode> {
        TypedTree::open(&self.backing, HIST_KEY)
    }

    fn main_state(&self) -> TypedTree<MapStateNode> {
        TypedTree::open(&self.backing, STATE_KEY)
    }

    fn snapshot_state(&self, id: &Id) -> TypedTree<MapStateNode> {
        let full_key = [STATE_KEY, &id.to_string()].concat();
        TypedTree::open(&self.backing, full_key)
    }

    /// Uses the same tree for all content types, it's up to the caller to ensure the correct type is being deserialized.
    fn node_content<T>(&self) -> TypedTree<T> {
        TypedTree::open(&self.backing, CONTENT_KEY)
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

fn new_test_map(mut commands: Commands) {
    let map = MapDb::new_temp().unwrap();
    commands.insert_resource(map);
}

fn flush_db(map_db: Res<MapDb>) {
    // Flushes the main Tree which is always open afaik.
    // Other trees are flushed after they are dropped in each system :)
    map_db.backing.flush().unwrap();
}

fn push_test(map_db: Res<MapDb>, mut id_gen: ResMut<IdGen>) -> Result {
    let hist = map_db.history();

    let new_node_id = id_gen.generate();
    let new_node_kind = NodeKind::Brush;

    // Insert the node content
    let brush_content = map_db.node_content::<Brush>();
    let new_node_content_id = id_gen.generate();

    let new_brush = Brush {
        bounds: BrushBounds {
            start: Vec3::NEG_ONE,
            end: Vec3::ONE,
        },
    };

    brush_content.insert(&new_node_content_id, &new_brush)?;

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
    let state = map_db.main_state();
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
    let tree_names: Vec<String> = map_db
        .backing
        .tree_names()
        .into_iter()
        .map(|bytes| String::from_utf8(bytes.to_vec()).unwrap_or("invalid".to_string()))
        .collect();
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
