use alloc::format;
use alloc::sync::Arc;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use http::Response;
use stack_string::SmallString;
use worker::{Env, KvError};
use crate::crypto::random_bytes_generic;
use crate::kv::{KvCore, KvInterface, KvInterfaceOwned};
use crate::session::{Session, SessionData, KV_CACHE_TTL, SESSION_MAX_AGE};
use crate::user::IDENT_SIZE;

pub const RFTK_SIZE: usize = 18;

pub struct RefreshToken {
    entropy: [u8; RFTK_SIZE],
    identifier: [u8; IDENT_SIZE],
}

impl RefreshToken {
    pub fn new(identifier: [u8; IDENT_SIZE]) -> Self {
        let entropy = random_bytes_generic::<RFTK_SIZE>();

        Self { entropy, identifier }
    }

    pub fn from_parts(entropy: [u8; RFTK_SIZE], identifier: [u8; IDENT_SIZE]) -> Self {
        Self { entropy, identifier }
    }

    pub fn into_parts(self) -> ([u8; RFTK_SIZE], [u8; IDENT_SIZE]) {
        (self.entropy, self.identifier)
    }

    pub fn parts(&self) -> (&[u8; RFTK_SIZE], &[u8; IDENT_SIZE]) {
        (&self.entropy, &self.identifier)
    }
    
    pub fn identifier(&self) -> &[u8; IDENT_SIZE] {
        &self.identifier
    }
    
    pub fn entropy(&self) -> &[u8; RFTK_SIZE] {
        &self.entropy
    }
    
    pub fn get_entropy_b64(&self) -> SmallString<{ RFTK_SIZE * 8 / 6 }> {
        let mut dest = SmallString::<{ RFTK_SIZE * 8 / 6 }>::new();
        BASE64_STANDARD.encode_slice(
            &self.entropy,
            unsafe { dest.as_bytes_mut() }, // Aliasing
        ).unwrap();
        
        dest
    }
}

impl KvCore for RefreshToken {
    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError> {
        let mut data = SmallString::<24>::new();
        BASE64_STANDARD.encode_slice(
            &self.identifier,
            unsafe { data.as_bytes_mut() }, // Aliasing
        ).unwrap();

        let entropy_b64 = self.get_entropy_b64();
        let kv = env.kv("kSESSIONS").unwrap();

        kv.put(&*format!("refresh:{entropy_b64}"), data)
            .unwrap()
            .expiration_ttl(SESSION_MAX_AGE)
            .execute()
            .await
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError> {
        let kv = env.kv("kSESSIONS").unwrap();
        let entropy_b64 = self.get_entropy_b64();

        kv.delete(&*format!("refresh:{entropy_b64}"))
            .await
    }

    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let entropy_b64 = self.get_entropy_b64();
        let res = kv.get(&*format!("refresh:{entropy_b64}"))
            .cache_ttl(KV_CACHE_TTL)
            .text()
            .await;

        res.map(|v| v.is_some())
    }
}

impl KvInterface<[u8; RFTK_SIZE]> for RefreshToken {
    async fn fetch_from_remote(token: &[u8; RFTK_SIZE], env: Arc<Env>) -> Result<Option<Self>, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let mut token_b64 = SmallString::<24>::new();
        BASE64_STANDARD.encode_slice(
            &token,
            unsafe { token_b64.as_bytes_mut() }, // Aliasing
        ).unwrap();

        let Some(identifier_b64) = kv.get(&*format!("refresh:{token_b64}"))
            .cache_ttl(KV_CACHE_TTL)
            .text()
            .await?
        else {
            return Ok(None)
        };

        let mut identifier = [0u8; IDENT_SIZE];
        
        BASE64_STANDARD.decode_slice(identifier_b64, &mut identifier).unwrap();

        Ok(Some(RefreshToken {
            entropy: *token,
            identifier,
        }))
    }
}