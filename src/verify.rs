use alloc::{format, string::String, sync::Arc, vec::Vec};
use serde::{Deserialize, Serialize};
use base64::{Engine, prelude::BASE64_STANDARD};
use worker::{Date, Env, KvError};

use crate::{crypto::random_bytes_generic, kv::KvInterface};

pub const MAX_RETRIES: u32 = 3;

#[derive(Serialize, Deserialize)]
pub struct VerificationSession {
    pub id: [u8; 18],

    pub password_hash: [u8; 32],
    pub password_salt: [u8; 32],

    pub display_name: String,
    pub user_email: String,

    pub code_digits: [char; 6],

    pub total_retries: u32,
    pub send_timeout_end_utc: u64,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationSessionData {
    /// Base64-encoded password hash
    password_hash: String,

    /// Base64-encoded password salt
    password_salt: String,

    display_name: String,
    user_email: String,

    code: String,

    total_retries: u32,

    send_timeout_end_utc: u64,
}

pub const KV_CACHE_TTL: u64 = 5 * 60;
pub const SESSION_MAX_AGE: u64 = 15 * 60;

impl VerificationSession {
    pub fn new(
        user_email: String,
        display_name: String,
        password_hash: [u8; 32],
        password_salt: [u8; 32],
        code: [char; 6],
    ) -> Self {
        let id = random_bytes_generic::<18>();

        Self {
            id,
            user_email,
            display_name,
            password_hash,
            password_salt,
            code_digits: code,
            total_retries: 0,
            send_timeout_end_utc: 0,
        }
    }

    pub fn id(&self) -> &[u8; 18] {
        &self.id
    }

    pub fn code_string(&self) -> String {
        self.code_digits.iter().collect()
    }

    /// Increments the retry count, but the changes must manually be pushed to the remote.
    ///
    /// Returns whether or not the retry count exceeds the maximum number of allowed retries
    pub fn inc_retries(&mut self) -> bool {
        self.total_retries += 1;

        self.total_retries > MAX_RETRIES
    }

    pub fn increase_send_timeout_end_utc(&mut self) {
        self.send_timeout_end_utc = Date::now().as_millis() + 2 * 60 * 1000;
    }

    pub fn can_send(&self) -> bool {
        Date::now().as_millis() as i64 - self.send_timeout_end_utc as i64 >= 0
    }
}

impl KvInterface for VerificationSession {
    type Key = [u8; 18];

    async fn fetch_from_remote(id: &Self::Key, env: Arc<Env>) -> Result<Option<Self>, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let Some(data) = kv.get(&*format!("verify:{}", BASE64_STANDARD.encode(id)))
            .cache_ttl(KV_CACHE_TTL)
            .json::<VerificationSessionData>()
            .await?
        else {
            return Ok(None)
        };

        Ok(Some(VerificationSession {
            id: *id,
            user_email: data.user_email,
            display_name: data.display_name,
            password_hash: BASE64_STANDARD.decode(data.password_hash).unwrap().try_into().unwrap(),
            password_salt: BASE64_STANDARD.decode(data.password_salt).unwrap().try_into().unwrap(),
            code_digits: data.code.chars().collect::<Vec<char>>()[0..6].try_into().unwrap(),
            total_retries: data.total_retries,
            send_timeout_end_utc: data.send_timeout_end_utc,
        }))
    }

    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError> {
        let data = VerificationSessionData {
            user_email: self.user_email.clone(),
            display_name: self.display_name.clone(),
            password_hash: BASE64_STANDARD.encode(self.password_hash),
            password_salt: BASE64_STANDARD.encode(self.password_salt),
            code: self.code_digits.into_iter().collect::<String>(),
            total_retries: self.total_retries,
            send_timeout_end_utc: self.send_timeout_end_utc,
        };

        let kv = env.kv("kSESSIONS").unwrap();

        kv.put(&*format!("verify:{}", BASE64_STANDARD.encode(self.id)), data)
            .unwrap() // If you look at the implementation for 'put' IT NEVER FAILS WTF CHANGE THE RETURN TYPE
            .expiration_ttl(SESSION_MAX_AGE)
            .execute()
            .await
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        kv.delete(&*format!("verify:{}", BASE64_STANDARD.encode(self.id)))
            .await
    }

    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let res = kv.get(&*format!("session:{}", BASE64_STANDARD.encode(self.id)))
            .cache_ttl(KV_CACHE_TTL)
            .json::<VerificationSessionData>()
            .await;

        res.map(|v| v.is_some())
    }
}
