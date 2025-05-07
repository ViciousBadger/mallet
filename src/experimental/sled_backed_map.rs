use std::marker::PhantomData;

use bevy::{input::common_conditions::input_pressed, prelude::*};
use redb::{Database, TableDefinition, TypeName};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use ulid::Ulid;

use crate::{
    core::map::{
        brush::{Brush, BrushBounds},
        light::{Light, LightType},
    },
    id::{Id, IdGen},
};

#[derive(Resource, Deref)]
struct MapDb {
    backing: redb::Database,
}

const HIST_TABLE: TableDefinition<Id, Card<MapHistoryNode>> = TableDefinition::new("history");
const STATE_TABLE: TableDefinition<Id, Card<MapStateNode>> = TableDefinition::new("node_state");
const CONTENT_TABLE_BRUSH: TableDefinition<Id, Card<Brush>> = TableDefinition::new("content_brush");
const CONTENT_TABLE_LIGHT: TableDefinition<Id, Card<Light>> = TableDefinition::new("content_light");

impl MapDb {
    pub fn new_temp() -> MapDb {
        MapDb {
            backing: Database::builder().create("test.db").unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MapHistoryNode {
    pub parent: Option<Id>,
    pub timestamp: i64,
    pub action: MapAction,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MapAction {
    StateSnapshot(Id),
    Delta {
        node_id: Id,
        node_kind: NodeKind,
        delta: MapDelta,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MapDelta {
    Create { content_key: Id },
    Modify { before_key: Id, after_key: Id },
    Remove { content_key: Id },
}

#[derive(Serialize, Deserialize, Debug)]
struct MapStateNode {
    pub name: String,
    pub node_kind: NodeKind,
    pub content_key: Id,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum NodeKind {
    Brush = 0,
    Light = 1,
}

fn new_test_map(mut commands: Commands) {
    let map = MapDb::new_temp();
    commands.insert_resource(map);
}

impl redb::Value for Id {
    type SelfType<'a>
        = Id
    where
        Self: 'a;

    type AsBytes<'a>
        = [u8; 16]
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        u128::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Id(Ulid::from_bytes(data.try_into().unwrap()))
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        value.to_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new("Id")
    }
}

impl redb::Key for Id {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        data1.cmp(data2)
    }
}

#[derive(Debug)]
struct Card<T> {
    marker: PhantomData<T>,
}
impl<T> redb::Value for Card<T>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
{
    type SelfType<'a>
        = T
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        postcard::from_bytes(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        postcard::to_stdvec(value).unwrap()
    }

    fn type_name() -> TypeName {
        TypeName::new(&format!("Card<{}>", std::any::type_name::<T>()))
    }
}

fn push_test(map_db: Res<MapDb>, mut id_gen: ResMut<IdGen>) -> Result {
    let new_node_id = id_gen.generate();
    let new_node_kind = NodeKind::Brush;

    let txn = map_db.begin_write()?;
    {
        // Insert the node content
        let mut brush_content = txn.open_table(CONTENT_TABLE_BRUSH)?;

        let new_node_content_id = id_gen.generate();

        let new_brush = Brush {
            bounds: BrushBounds {
                start: Vec3::NEG_ONE,
                end: Vec3::ONE,
            },
        };

        brush_content.insert(&new_node_content_id, &new_brush)?;

        let mut light_content = txn.open_table(CONTENT_TABLE_LIGHT)?;
        let new_light_content_id = id_gen.generate();
        light_content.insert(
            &new_light_content_id,
            &Light {
                position: Vec3::ZERO,
                light_type: LightType::Point,
                color: Color::srgb(1.0, 0.0, 0.0),
                intensity: 1000.,
                range: 10.,
            },
        )?;

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

        let mut hist = txn.open_table(HIST_TABLE)?;
        hist.insert(&new_hist_id, &new_node)?;

        // Create state entry in main state
        // Should probably happen in a separate system that applies history entries.
        // also, needs to be propagated to the game world.
        let mut state = txn.open_table(STATE_TABLE)?;
        let new_state_node = MapStateNode {
            name: "a brush".to_string(),
            node_kind: new_node_kind,
            content_key: new_node_content_id,
        };
        state.insert(&new_node_id, &new_state_node)?;

        info!("yey done!");
    }

    txn.commit()?;

    Ok(())
}

pub fn plugin(app: &mut App) {
    app.init_resource::<IdGen>();
    app.add_systems(Startup, new_test_map);
    app.add_systems(Update, (push_test.run_if(input_pressed(KeyCode::KeyF)),));
}
