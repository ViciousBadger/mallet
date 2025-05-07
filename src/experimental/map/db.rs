use std::marker::PhantomData;

use bevy::prelude::*;
use redb::{Database, TableDefinition, TypeName};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use ulid::Ulid;

use crate::{
    core::map::{brush::Brush, light::Light},
    id::Id,
};

#[derive(Resource, Deref)]
pub struct Db {
    backing: redb::Database,
}

pub const CONTENT_TABLE_BRUSH: TableDefinition<Id, Postcard<Brush>> =
    TableDefinition::new("content_brush");
pub const CONTENT_TABLE_LIGHT: TableDefinition<Id, Postcard<Light>> =
    TableDefinition::new("content_light");

impl Db {
    pub fn new_temp() -> Db {
        Db {
            backing: Database::builder().create("test.db").unwrap(),
        }
    }
}

pub const META_TABLE: TableDefinition<(), Postcard<Meta>> = TableDefinition::new("meta");

#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    pub name: String,
    pub hist_key: Id,
}

pub const HIST_TABLE: TableDefinition<Id, Postcard<HistNode>> = TableDefinition::new("history");

#[derive(Serialize, Deserialize, Debug)]
pub enum HistNode {
    MapInit {
        timestamp: i64,
    },
    Node {
        parent_key: Id,
        timestamp: i64,
        action: Action,
    },
}

pub fn new_timestamp() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    StateSnapshot(Id),
    Delta { element_id: Id, delta: Delta },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Delta {
    Create { element: Element, content_key: Id },
    Modify { then: Element, now: Element },
    Remove { element: Element, content_key: Id },
}

pub const MAIN_STATE_TABLE: TableDefinition<Id, Postcard<Element>> =
    TableDefinition::new("main_state");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Element {
    pub name: String,
    pub role: ElementRole,
    pub content_key: Id,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum ElementRole {
    Brush = 0,
    Light = 1,
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
pub struct Postcard<T> {
    marker: PhantomData<T>,
}
impl<T> redb::Value for Postcard<T>
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
