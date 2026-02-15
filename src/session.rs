use alloc::{format, string::String, sync::Arc};
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use worker::{Env, KvError};

use crate::{crypto::random_bytes_generic, d1::D1Interface, kv::KvInterface, user::{User, UserIdentifyingKey}};

#[derive(Serialize, Deserialize)]
pub struct Session {
    session_id: [u8; 18],
    user_email: String,
    display_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    user_email: String,
    display_name: String,
}

pub const KV_CACHE_TTL: u64 = 10 * 60;
pub const SESSION_MAX_AGE: u64 = 30 * 60;

impl Session {
    pub fn new(user_email: String, display_name: String) -> Self {
        let id = random_bytes_generic::<18>();

        Self {
            session_id: id,
            user_email,
            display_name,
        }
    }

    pub fn user_email(&self) -> &str {
        &self.user_email
    }

    pub fn user_display_name(&self) -> &str {
        &self.display_name
    }

    pub fn session_id(&self) -> &[u8; 18] {
        &self.session_id
    }
}

impl KvInterface for Session {
    type Key = [u8; 18];

    async fn fetch_from_remote(id: &Self::Key, env: Arc<Env>) -> Result<Option<Self>, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let Some(data) = kv.get(&*format!("session:{}", BASE64_STANDARD.encode(id)))
            .cache_ttl(KV_CACHE_TTL)
            .json::<SessionData>()
            .await?
        else {
            return Ok(None)
        };

        Ok(Some(Session {
            session_id: *id,
            user_email: data.user_email,
            display_name: data.display_name,
        }))
    }

    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError> {
        let data = SessionData {
            user_email: self.user_email.clone(),
            display_name: self.display_name.clone(),
        };

        let kv = env.kv("kSESSIONS").unwrap();

        kv.put(&*format!("session:{}", BASE64_STANDARD.encode(self.session_id)), data)
            .unwrap()
            .expiration_ttl(SESSION_MAX_AGE)
            .execute()
            .await
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        kv.delete(&*format!("session:{}", BASE64_STANDARD.encode(self.session_id)))
            .await
    }

    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let res = kv.get(&*format!("session:{}", BASE64_STANDARD.encode(self.session_id)))
            .cache_ttl(KV_CACHE_TTL)
            .json::<SessionData>()
            .await;

        res.map(|v| v.is_some())
    }
}

#[macro_export]
macro_rules! get_session {
    (
        with
            cookie_jar: $cookie:expr,
            state: $state:expr
    ) => {
        todo!("FIX THE FUCKING REFRESH TOKEN SYSTEM HOLY FUCK SHIT FUCK IS IT BROKEN LIKE HOLY FUCKING GOD")

        match (($cookie).get("ssid"), ($cookie).get("rftk")) {
            (Some(ssid), _) => {
                let decoded_ssid: [u8; 18] = BASE64_STANDARD.decode(ssid.value()).unwrap().try_into().unwrap();

                // Ensure that a session exists for the user
                match Session::fetch_from_remote(&decoded_ssid, ($state).env.clone()).await {
                    Err(e) => {
                        return GenericResponse(
                            StatusCode::BAD_GATEWAY,
                            &[("Content-Type", "text/plain")],
                            format!("502 Bad Gateway (error fetching from KV): {e}")
                        )
                    },
                    Ok(Some(session)) => session,
                    Ok(None) => return GenericResponse(
                        StatusCode::TEMPORARY_REDIRECT,
                        &[("Location", "/login")],
                        "Unauthorised (redirecting you to login)".to_string()
                    )
                }
            },
            (None, Some(rftk)) => {
                let decoded_rftk: [u8; 32] = BASE64_STANDARD.decode(rftk.value()).unwrap().try_into().unwrap();
                let user_id: [u8; 18] = decoded_rftk[14..].try_into().unwrap();

                match Session::fetch_from_remote(&user_id, ($state).env.clone()).await {
                    Ok(Some(session)) => {
                        if let Err(e) = session.put_to_remote(($state).env.clone()).await {
                            return GenericResponse(
                                StatusCode::BAD_GATEWAY,
                                &[("Content-Type", "text/plain")],
                                format!("502 Bad Gateway (error putting to KV): {e}")
                            )
                        }

                        let session_cookie = Cookie::new("ssid", BASE64_STANDARD.encode(session.session_id()))
                            .with_secure(true)
                            .with_http_only(true)
                            .with_max_age(Duration::from_secs(30 * 60))
                            .with_same_site(SameSite::Strict)
                            .with_path("/");

                        // Set the session token cookie
                        ($cookie).add(session_cookie);

                        // Create a refresh token (112 random bits + 144 bit user id)
                        let refresh_token: [u8; 32] =
                            [random_bytes_generic::<14>().as_slice(), &*BASE64_STANDARD.decode(user_id).unwrap()]
                                .concat()
                                .try_into()
                                .unwrap();

                        let refresh_token_cookie = Cookie::new("rftk", BASE64_STANDARD.encode(refresh_token))
                            .with_secure(true)
                            .with_http_only(true)
                            .with_max_age(Duration::from_secs(45 * 24 * 60 * 60))
                            .with_same_site(SameSite::Strict)
                            .with_path("/");

                        // Rotate the refresh token cookie
                        ($cookie).add(refresh_token_cookie);

                        session
                    },
                    Ok(None) => return GenericResponse(
                        StatusCode::TEMPORARY_REDIRECT,
                        &[("Location", "/register")],
                        "No such user (redirecting you to register)".to_string()
                    ),
                    Err(e) => return GenericResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "text/plain")],
                        format!("502 Bad Gateway (error putting to KV): {e}")
                    )
                }
            },
            (None, None) => return GenericResponse(
                StatusCode::TEMPORARY_REDIRECT,
                &[("Location", "/login")],
                "Unauthorised (redirecting you to login)".to_string()
            )
        }
    };
}
