use core::future::Future;

use alloc::{borrow::ToOwned, boxed::Box, format, string::{String, ToString}, sync::Arc, vec::Vec};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use stack_string::SmallString;
use web_sys::CryptoKey;
use worker::{D1Error, Env, crypto};

use crate::{crypto::random_bytes_generic, d1::D1Interface, kv::KvInterface, session::Session};
use crate::d1::D1Core;
use crate::session::SESSION_ID_SIZE;

pub const IDENT_SIZE: usize = 18;
pub const IDENT_B64_CHARS: usize = IDENT_SIZE * 8 / 6;

/// Devices associated with a user
#[derive(Debug)]
pub struct UserDevice {
    pub device_number: u8,
    pub is_independent: bool,
    pub nickname: SmallString<18>,
}

#[derive(Debug)]
pub struct UserPasskey {
    /// Base64url encoded
    pub credential_id: String,
    pub counter: i32,
    pub public_key: CryptoKey,
    pub webauthn_challenge: [u8; 32],
}

#[derive(Debug)]
pub struct UserTOTP {
    // This is encrypted at rest but decrypted for use here
    pub secret: [u8; 20],
}

/// A struct that holds information about a user
#[derive(Debug)]
pub struct User {
    /// Base64-encoded random 18 bytes
    pub id: SmallString<IDENT_B64_CHARS>,
    pub display_name: SmallString<18>,
    pub password_hash_phc: Option<String>,
    pub email: String,
    pub phone_number_hash: Option<SmallString<44>>,
    pub attachment_pubkey: SmallString<44>,
}

pub enum UserIdentifyingKey {
    Email(String),
    UserId(SmallString<24>),
    SessionId([u8; SESSION_ID_SIZE]),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DBRow {
    pub id: SmallString<24>,
    pub email: String,
    pub display_name: String,

    pub password_hash_phc: String,

    pub participations: String,
    pub pending_entrances: String,

    pub creation_timestamp: i64,
}

impl User {
    pub fn random_id() -> SmallString<24> {
        let mut b64 = SmallString::<24>::new();
        BASE64_STANDARD.encode_slice(
            random_bytes_generic::<IDENT_SIZE>(),
            unsafe { b64.as_bytes_mut() }, // Aliasing
        ).unwrap();

        b64
    }
}

impl D1Core for User {
    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("insert into users values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
            .bind(&[
                (&*self.id).into(),
                (&*self.email).into(),
                (&*self.display_name).into(),
                (&*self.password_hash_phc).into(),
                (&*self.participations.join(",")).into(),
                (&*self.pending_entrances.iter().map(|(i, r)| format!("{i}:{r}")).collect::<Vec<_>>().join(",")).into(),
                self.creation_time.timestamp_millis().into(),
            ])?;

        query.run().await.map(|_| ())
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = kdb.prepare("delete from users where id = ?1")
            .bind(&[(&*self.id).into()])?;

        query.run().await.map(|_| ())
    }
}

impl D1Interface<UserIdentifyingKey> for User {

    async fn fetch_from_remote(key: &UserIdentifyingKey, env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        let kdb = env.d1("kDB").unwrap();

        let query = match key {
            UserIdentifyingKey::Email(email) =>
                kdb.prepare("select * from users where email = ?1")
                    .bind(&[(&*email).into()])?,
            UserIdentifyingKey::UserId(id) =>
                kdb.prepare("select * from users where id = ?1")
                    .bind(&[id.as_str().into()])?,
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
            display_name: row.display_name,
            password_hash_phc: row.password_hash_phc,
            id: row.id,
            participations: row.participations.split(",").map(ToOwned::to_owned).collect(),
            pending_entrances:
            row.pending_entrances
                .split(",")
                .map(|i| i.split_once(":").unwrap())
                .map(|(i, r)| (i.to_owned(), r.to_owned()))
                .collect(),
            creation_time: DateTime::<Utc>::from_timestamp_millis(row.creation_timestamp).unwrap()
        }))
    }
}
