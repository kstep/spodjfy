use itertools::Itertools;
use serde::{de::DeserializeOwned, Serialize};
use sled::{Batch, Db, IVec, Tree};
use std::{marker::PhantomData, path::Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("decode error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("database error: {0}")]
    Sled(#[from] sled::Error),
}

pub trait StorageModel: Serialize + DeserializeOwned + Sized {
    const TREE_NAME: &'static str;
    type Key: AsRef<[u8]> + ?Sized = str;

    fn key(&self) -> &Self::Key;
    fn encode(&self) -> Result<Vec<u8>, StorageError> { Ok(bincode::serialize(self)?) }
    fn decode(data: IVec) -> Result<Self, StorageError> { Ok(bincode::deserialize(&data)?) }
}

pub struct Storage {
    db: Db,
}

pub struct Collection<T> {
    tree: Tree,
    phantom: PhantomData<T>,
}

impl<T: StorageModel> Collection<T> {
    pub fn put(&self, model: T) -> Result<(), StorageError> {
        self.tree.insert(model.key(), model.encode()?)?;

        Ok(())
    }

    pub fn put_all<I: IntoIterator<Item = T>>(&self, models: I) -> Result<(), StorageError> {
        for chunk in &models.into_iter().chunks(100) {
            let mut batch = Batch::default();

            for model in chunk {
                batch.insert(model.key().as_ref(), model.encode()?);
            }

            self.tree.apply_batch(batch)?;
        }

        Ok(())
    }

    pub fn get(&self, key: &T::Key) -> Result<Option<T>, StorageError> {
        match self.tree.get(key)? {
            Some(model) => T::decode(model).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete(&self, key: &T::Key) -> Result<Option<T>, StorageError> {
        match self.tree.remove(key)? {
            Some(model) => T::decode(model).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_all<'a, I>(&self, keys: I) -> Result<(), StorageError>
    where
        I: IntoIterator<Item = &'a T::Key>,
        T::Key: 'a,
    {
        for chunk in &keys.into_iter().chunks(100) {
            let mut batch = Batch::default();

            for key in chunk {
                batch.remove(key.as_ref());
            }

            self.tree.apply_batch(batch)?;
        }

        Ok(())
    }

    pub async fn flush(&self) -> Result<usize, StorageError> { Ok(self.tree.flush_async().await?) }
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> { Ok(Self { db: sled::open(path)? }) }

    pub fn collection<T: StorageModel>(&self) -> Result<Collection<T>, StorageError> {
        Ok(Collection {
            tree: self.db.open_tree(T::TREE_NAME)?,
            phantom: PhantomData,
        })
    }
}
