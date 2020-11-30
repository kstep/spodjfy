use itertools::Itertools;
use serde::{de::DeserializeOwned, Serialize};
use sled::{Batch, Db, Error, IVec, Tree};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::mpsc::Receiver;
use relm::Channel;
use crate::servers::ResultSender;
use rspotify::model::{FullTrack, FullEpisode, FullPlaylist};

pub trait StoreModel: Sized {
    const TREE_NAME: &'static str;
    type Key: AsRef<[u8]> + ?Sized = str;
    fn key(&self) -> &Self::Key;

    fn encode(&self) -> Vec<u8>;
    fn decode(data: IVec) -> Option<Self>;
}

default impl<T: Serialize + DeserializeOwned> StoreModel for T {
    fn encode(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_else(|_| Vec::new())
    }
    fn decode(data: IVec) -> Option<Self> {
        bincode::deserialize(&data).ok()
    }
}

pub enum StorageCmd {
    PutTracks { items: Vec<FullTrack> },
    GetTrack { tx: ResultSender<Option<FullTrack>>, id: String },

    PutEpisodes { items: Vec<FullEpisode> },
    GetEpisode { tx: ResultSender<Option<FullEpisode>>, id: String },

    PutPlaylist { items: Vec<FullPlaylist> },
    GetPlaylist { tx: ResultSender<Option<FullPlaylist>>, id: String },
}

pub struct StorageServer {
    channel: Receiver<StorageCmd>,
    store: Storage,
    tracks_coll: Collection<FullTrack>,
    episodes_coll: Collection<FullEpisode>,
    playlists_coll: Collection<FullPlaylist>,
}

impl StorageServer {
    fn new(store: Storage, channel: Receiver<StorageCmd>) -> Self {
        StorageServer {
            channel,
            store,
            tracks_coll: store.collection().unwrap(),
            episodes_coll: store.collection().unwrap(),
        }
    }
}

pub struct Storage {
    db: Db,
}

pub struct Collection<T> {
    tree: Tree,
    phantom: PhantomData<T>,
}

impl<T: StoreModel> Collection<T> {
    pub fn put(&self, model: T) -> Result<(), Error> {
        self.tree.insert(model.key(), model.encode())?;
        Ok(())
    }

    pub fn put_all<I: IntoIterator<Item = T>>(&self, models: I) -> Result<(), Error> {
        for chunk in &models.into_iter().chunks(100) {
            let mut batch = Batch::default();
            for model in chunk {
                batch.insert(model.key().as_ref(), model.encode());
            }
            self.tree.apply_batch(batch)?;
        }

        Ok(())
    }

    pub fn get(&self, key: &T::Key) -> Result<Option<T>, Error> {
        Ok(self.tree.get(key)?.and_then(T::decode))
    }

    pub fn delete(&self, key: &T::Key) -> Result<Option<T>, Error> {
        Ok(self.tree.remove(key)?.and_then(T::decode))
    }

    pub fn delete_all<'a, I>(&self, keys: I) -> Result<(), Error>
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
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            db: sled::open(path)?,
        })
    }

    pub fn collection<T: StoreModel>(&self) -> Result<Collection<T>, Error> {
        Ok(Collection {
            tree: self.db.open_tree(T::TREE_NAME)?,
            phantom: PhantomData,
        })
    }
}
