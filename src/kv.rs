use alloc::sync::Arc;
use core::marker::PhantomData;
use serde::de::DeserializeOwned;
use worker::{Env, KvError, KvStore};

pub trait KvInterface: Sized {
    type Key: ?Sized;

    async fn fetch_from_remote(key: &Self::Key, env: Arc<Env>) -> Result<Option<Self>, KvError>;

    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError>;
    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError>;

    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError>;
}

pub trait KvInterfaceExt: KvInterface {
    async fn owned_fetch_from_remote(key: Self::Key, env: Arc<Env>) -> Result<Option<Self>, KvError>;
}

impl<T> KvInterfaceExt for T
where
    T: KvInterface,
    T::Key: Sized,
{
    async fn owned_fetch_from_remote(key: Self::Key, env: Arc<Env>) -> Result<Option<Self>, KvError> {
        Self::fetch_from_remote(&key, env).await
    }
}