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
    #[error("missing indexed item with key {0}")]
    MissingItem(IVec),
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

pub struct Index<T> {
    index_tree: Tree,
    data_tree: Tree,
    phantom: PhantomData<T>,
}

impl<T: StorageModel> Index<T> {
    fn find<P: AsRef<[u8]>>(&self, prefix: P) -> impl Iterator<Item = Result<T, StorageError>> + '_ {
        let prefix_len = prefix.as_ref().len();
        self.index_tree.scan_prefix(prefix.as_ref()).map(move |item| {
            let (key, _) = item?;
            let (_, id) = key.as_ref().split_at(prefix_len);
            Ok(T::decode(
                self.data_tree.get(id)?.ok_or_else(|| StorageError::MissingItem(key))?,
            )?)
        })
    }

    fn update<F: Fn(&T) -> &[u8]>(&self, prefix: F) -> Result<(), StorageError> {
        for item in self.data_tree.iter() {
            let (key, data) = item?;
            let prefix = prefix(T::decode(data)?);
            let mut index_key = Vec::with_capacity(prefix.len() + key.len());
            index_key.extend_from_slice(prefix);
            index_key.extend_from_slice(&key);
            self.index_tree.insert(index_key, IVec::default())?;
        }
        Ok(())
    }
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
