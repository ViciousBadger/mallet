use std::marker::PhantomData;

use bevy::prelude::*;
use redb::{Database, TableDefinition, TransactionError, TypeName};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use ulid::Ulid;

use crate::id::Id;

#[derive(Resource, Deref)]
pub struct Db {
    backing: redb::Database,
}

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
    pub hist_node_id: Id,
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
