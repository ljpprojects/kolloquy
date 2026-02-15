use alloc::{string::ToString, sync::Arc};
use worker::{D1Error, Env};

// The lifetimes indicate to the co
pub trait D1Interface: Sized {
    type Key: ?Sized;

    async fn fetch_from_remote(key: &Self::Key, env: Arc<Env>) -> Result<Option<Self>, worker::Error>;

    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), worker::Error>;
    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), worker::Error>;
}

pub trait D1InterfaceExt: D1Interface {
    async fn owned_fetch_from_remote(key: Self::Key, env: Arc<Env>) -> Result<Option<Self>, worker::Error>;
}

impl<T> D1InterfaceExt for T
where
    T: D1Interface,
    T::Key: Sized,
{
    async fn owned_fetch_from_remote(key: Self::Key, env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        Self::fetch_from_remote(&key, env).await
    }
}
