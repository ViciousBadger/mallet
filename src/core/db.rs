use std::{marker::PhantomData, sync::Arc};

use bevy::prelude::*;
use redb::{Database, TableDefinition, TypeName};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

use crate::id::Id;

#[derive(Resource, Deref)]
pub struct Db {
    backing: Arc<redb::Database>,
}

impl Db {
    pub fn new() -> Db {
        Db {
            backing: Arc::new(Database::builder().create("test.mmap").unwrap()),
        }
    }
}

pub const TBL_META: TableDefinition<(), Typed<Meta>> = TableDefinition::new("meta");
pub const TBL_OBJECTS: TableDefinition<Checksum, Object> = TableDefinition::new("objects");

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

#[derive(Debug, Deref, DerefMut, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Checksum(blake3::Hash);

impl Checksum {
    pub fn nil() -> Self {
        Self(blake3::Hash::from_bytes([0; 32]))
    }
}

impl redb::Value for Checksum {
    type SelfType<'a>
        = Checksum
    where
        Self: 'a;

    type AsBytes<'a>
        = [u8; blake3::OUT_LEN]
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        Some(blake3::OUT_LEN)
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Checksum(blake3::Hash::from_bytes(data.try_into().unwrap()))
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        *value.as_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new("Checksum")
    }
}

impl redb::Key for Checksum {
    fn compare(sum1: &[u8], sum2: &[u8]) -> std::cmp::Ordering {
        // No reason to sort hash values.
        //std::cmp::Ordering::Equal
        sum1.cmp(sum2)
    }
}

/// Redb storage value for "objects" of any type, identified by their checksum.
#[derive(Debug)]
pub struct Object {
    pub data: Vec<u8>,
}

impl Object {
    /// New object from a serializable type.
    pub fn new_typed<T: Serialize>(input: &T) -> (Checksum, Object) {
        let bytes = Self::to_bytes(input);
        (Self::checksum(&bytes), Object { data: bytes })
    }

    pub fn checksum_typed<T: Serialize>(input: &T) -> Checksum {
        let bytes = Self::to_bytes(input);
        Checksum(blake3::hash(&bytes))
    }

    fn to_bytes<T: Serialize>(input: &T) -> Vec<u8> {
        postcard::to_stdvec(input).unwrap()
    }

    pub fn checksum(bytes: &[u8]) -> Checksum {
        Checksum(blake3::hash(bytes))
    }

    /// New object from raw data, e.g. a file.
    pub fn new_raw(bytes: Vec<u8>) -> (Checksum, Object) {
        let checksum = Self::checksum(&bytes);
        (checksum, Object { data: bytes })
    }

    /// Deserialize an object created from a type.
    pub fn cast<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        postcard::from_bytes(&self.data).unwrap()
    }
}

impl redb::Value for Object {
    type SelfType<'a>
        = Object
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
        Object {
            data: data.to_vec(),
        }
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        value.data.clone()
    }

    fn type_name() -> TypeName {
        TypeName::new("Object")
    }
}

#[derive(Debug)]
pub struct Typed<T> {
    marker: PhantomData<T>,
}

impl<T> redb::Value for Typed<T>
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
        TypeName::new(std::any::type_name::<T>())
    }
}

#[derive(Error, Debug)]
#[error("Invalid database key - not found")]
pub struct NotFound;

pub trait EnsureExists {
    type Output;
    fn ensure_exists(self) -> Result<Self::Output>;
}

impl<V> EnsureExists for Option<redb::AccessGuard<'static, V>>
where
    V: redb::Value + 'static,
{
    type Output = redb::AccessGuard<'static, V>;

    fn ensure_exists(self) -> Result<Self::Output> {
        Ok(self.ok_or(NotFound)?)
    }
}
