use alloc::{borrow::ToOwned, boxed::Box, format, string::{String, ToString}, sync::Arc, vec::Vec};
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use worker::{D1Error, Env};

use crate::{crypto::random_bytes_generic, d1::D1Interface, kv::KvInterface, session::Session};

/// A struct that holds information about a user
#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    /// Base64-encoded random 18 bytes
    pub id: String,

    pub email: String,
    pub email_verified: bool,

    pub display_name: String,

    // Decoded from the base64 stored in the db
    pub password_salt: [u8; 32],

    // Decoded from the base64 stored in the db
    pub password_hash: [u8; 32],

    pub participations: Vec<String>,
    pub pending_entrances: Vec<(String, String)>
}

/// A struct that holds non-sensitive information about a user
pub struct UserSanitised {
    pub display_name: String,
    pub email_verified: bool,
}

impl From<User> for UserSanitised {
    fn from(value: User) -> Self {
        Self {
            display_name: value.display_name,
            email_verified: value.email_verified,
        }
    }
}

pub enum UserIdentifyingKey {
    Email(String),
    UserId(String),
    SessionId([u8; 18]),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DBRow {
    pub id: String,

    pub email: String,
    pub email_verified: f64,

    pub display_name: String,

    pub password_salt: String,
    pub password_hash: String,

    pub participations: String,
    pub pending_entrances: String
}

impl User {
    pub fn random_id() -> String {
        // 144 random bits just because we can have more randomness with the
        // same size (once encoded) as a 16 byte random id
        BASE64_STANDARD.encode(random_bytes_generic::<18>())
    }
}


impl D1Interface for User {
    type Key = UserIdentifyingKey;

    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("insert into users values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
            .bind(&[
                (&*self.id).into(),
                (&*self.email).into(),
                (&*self.display_name).into(),
                (&*BASE64_STANDARD.encode(self.password_salt)).into(),
                (&*BASE64_STANDARD.encode(self.password_hash)).into(),
                (self.email_verified).into(),
                (&*self.participations.join(",")).into(),
                (&*self.pending_entrances.iter().map(|(i, r)| format!("{i}:{r}")).collect::<Vec<_>>().join(",")).into(),
            ])?;

        query.run().await.map(|r| r.results::<Vec<u8>>()).flatten().map(|_| ())
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("delete from users where id = ?1")
            .bind(&[(&*self.id).into()])?;

        query.run().await.map(|r| r.results::<Vec<u8>>()).flatten().map(|_| ())
    }

    async fn fetch_from_remote(key: &Self::Key, env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = match key {
            UserIdentifyingKey::Email(email) =>
                kdb.prepare("select * from users where email = ?1")
                    .bind(&[(&*email).into()])?,
            UserIdentifyingKey::UserId(id) =>
                kdb.prepare("select * from users where id = ?1")
                    .bind(&[(&*id).into()])?,
            UserIdentifyingKey::SessionId(ssid) => {
                let Some(session) = Session::fetch_from_remote(&ssid, env.clone()).await? else {
                    return Err(worker::Error::RustError("Cannot fetch user by session id when no such session exists.".to_string()))
                };

                let res = Box::pin(User::fetch_from_remote(&UserIdentifyingKey::Email(session.user_email().to_owned()), env)).await;

                return res
            }
        };

        let Some(row) = query.first::<DBRow>(None).await? else {
            return Ok(None)
        };

        Ok(Some(Self {
            email: row.email,
            email_verified: (row.email_verified as u64) != 0,
            display_name: row.display_name,
            password_salt: BASE64_STANDARD.decode(row.password_salt).unwrap().try_into().unwrap(),
            password_hash: BASE64_STANDARD.decode(row.password_hash).unwrap().try_into().unwrap(),
            id: row.id,
            participations: row.participations.split(",").map(ToOwned::to_owned).collect(),
            pending_entrances:
                row.pending_entrances
                    .split(",")
                    .map(|i| i.split_once(":").unwrap())
                    .map(|(i, r)| (i.to_owned(), r.to_owned()))
                    .collect()
        }))
    }
}
