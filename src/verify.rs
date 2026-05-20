use alloc::{format, string::String, sync::Arc, vec::Vec};
use serde::{Deserialize, Serialize};
use base64::{Engine, prelude::BASE64_STANDARD};
use stack_string::SmallString;
use worker::{Date, Env, KvError};

use crate::{crypto::random_bytes_generic, kv::KvInterface};
use crate::kv::{KvCore, KvInterfaceOwned};

pub const MAX_RETRIES: u32 = 3;

#[derive(Serialize, Deserialize)]
pub struct VerificationSession {
    pub id: SmallString<24>,
    pub display_name: String,
    pub user_email: String,

    pub password_hash_phc: String,

    pub code_digits: [char; 6],

    pub total_retries: u32,
    pub send_timeout_end_utc: u64,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationSessionData {
    /// Password hash PHC string
    password_hash_phc: String,

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
        password_hash_phc: String,
        code: [char; 6],
    ) -> Self {
        let mut id_str = SmallString::<24>::new();
        BASE64_STANDARD.encode_slice(
            random_bytes_generic::<18>(),
            unsafe { id_str.as_bytes_mut() }, // Aliasing
        ).unwrap();

        Self {
            id: id_str,
            user_email,
            display_name,
            password_hash_phc,
            code_digits: code,
            total_retries: 0,
            send_timeout_end_utc: 0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn code_string(&self) -> SmallString<6> {
        let mut string = SmallString::<6>::new();
        let string_bytes = unsafe { string.as_bytes_mut() }; // ALIASING?!?@!?!

        let mut i = 0usize;
        for c in self.code_digits {
            string_bytes[i] = c as u8;
            i += 1;
        }

        string
    }

    /// Increments the retry count, but the changes must manually be pushed to the remote.
    ///
    /// Returns whether or not the **new** retry count exceeds the maximum number of allowed retries
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

impl KvCore for VerificationSession {
    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError> {
        let data = VerificationSessionData {
            user_email: self.user_email.clone(),
            display_name: self.display_name.clone(),
            password_hash_phc: self.password_hash_phc.clone(),
            code: self.code_digits.into_iter().collect::<String>(),
            total_retries: self.total_retries,
            send_timeout_end_utc: self.send_timeout_end_utc,
        };

        let kv = env.kv("kSESSIONS").unwrap();

        kv.put(&*format!("verify:{}", self.id), data)
            .unwrap() // If you look at the implementation for 'put' IT NEVER FAILS WTF CHANGE THE RETURN TYPE
            .expiration_ttl(SESSION_MAX_AGE)
            .execute()
            .await
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        kv.delete(&*format!("verify:{}", self.id))
            .await
    }

    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let res = kv.get(&*format!("session:{}", self.id))
            .cache_ttl(KV_CACHE_TTL)
            .json::<VerificationSessionData>()
            .await;

        res.map(|v| v.is_some())
    }
}

impl KvInterfaceOwned<SmallString<24>> for VerificationSession {
    async fn owned_fetch_from_remote(id: SmallString<24>, env: Arc<Env>) -> Result<Option<Self>, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let Some(data) = kv.get(&*format!("verify:{id}"))
            .cache_ttl(KV_CACHE_TTL)
            .json::<VerificationSessionData>()
            .await?
        else {
            return Ok(None)
        };

        Ok(Some(VerificationSession {
            id,
            user_email: data.user_email,
            display_name: data.display_name,
            password_hash_phc: data.password_hash_phc,
            code_digits: data.code.chars().collect::<Vec<char>>()[0..6].try_into().unwrap(),
            total_retries: data.total_retries,
            send_timeout_end_utc: data.send_timeout_end_utc,
        }))
    }
}
