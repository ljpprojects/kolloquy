use alloc::{string::ToString, sync::Arc};
use core::future::Future;
use worker::{D1Error, Env};

pub trait D1Core: Sized {
    fn put_to_remote(&self, env: Arc<Env>) -> impl Future<Output = Result<(), worker::Error>>;
    fn delete_from_remote(self, env: Arc<Env>) -> impl Future<Output = Result<(), worker::Error>>;
}

pub trait D1Interface<K>: D1Core
where
    K: ?Sized,
{
    fn fetch_from_remote(key: &K, env: Arc<Env>) -> impl Future<Output = Result<Option<Self>, worker::Error>>;
}

pub trait D1InterfaceOwned<K>: D1Core
where
    K: Sized,
{
    fn owned_fetch_from_remote(key: K, env: Arc<Env>) -> impl Future<Output = Result<Option<Self>, worker::Error>>;
}

impl<T, K> D1InterfaceOwned<K> for T
where
    T: D1Interface<K>,
    K: Sized,
{
    async fn owned_fetch_from_remote(key: K, env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        <Self as D1Interface<K>>::fetch_from_remote(&key, env).await
    }
}