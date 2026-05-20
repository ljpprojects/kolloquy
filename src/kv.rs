use alloc::sync::Arc;
use core::future::Future;
use worker::{Env, KvError};

pub trait KvCore: Sized {
    fn put_to_remote(&self, env: Arc<Env>) -> impl Future<Output = Result<(), KvError>>;
    fn delete_from_remote(self, env: Arc<Env>) -> impl Future<Output = Result<(), KvError>>;

    fn check(&self, env: Arc<Env>) -> impl Future<Output = Result<bool, KvError>>;
}

pub trait KvInterface<K>: KvCore
where
    K: ?Sized,
{

    fn fetch_from_remote(key: &K, env: Arc<Env>) -> impl Future<Output = Result<Option<Self>, KvError>>;
}

pub trait KvInterfaceOwned<K>: KvCore
where
    K: Sized,
{
    fn owned_fetch_from_remote(key: K, env: Arc<Env>) -> impl Future<Output = Result<Option<Self>, KvError>>;
}

impl<T, K> KvInterfaceOwned<K> for T
where
    T: KvInterface<K>,
    K: Sized,
{
    async fn owned_fetch_from_remote(key: K, env: Arc<Env>) -> Result<Option<Self>, KvError> {
        <Self as KvInterface<K>>::fetch_from_remote(&key, env).await
    }
}