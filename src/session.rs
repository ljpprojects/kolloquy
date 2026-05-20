use alloc::{format, string::String, sync::Arc};
use axum_cookie::CookieManager;
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use stack_string::SmallString;
use worker::{Env, KvError};

use crate::{crypto::random_bytes_generic, d1::D1Interface, kv::KvInterface, user::{User, UserIdentifyingKey}};
use crate::kv::KvCore;
use crate::refresh::{RefreshToken, RFTK_SIZE};
use crate::util::ArrayB64EncodeSmallString;

pub const SESSION_ID_SIZE: usize = 18;

#[derive(Serialize, Deserialize)]
pub struct Session {
    session_id: [u8; SESSION_ID_SIZE],
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
        let id = random_bytes_generic::<SESSION_ID_SIZE>();

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

    pub fn session_id(&self) -> &[u8; SESSION_ID_SIZE] {
        &self.session_id
    }

    pub fn get_session_id_b64(&self) -> SmallString<{ SESSION_ID_SIZE * 8 / 6 }> {
        let mut dest = SmallString::<{ SESSION_ID_SIZE * 8 / 6 }>::new();

        BASE64_STANDARD.encode_slice(
            &self.session_id,
            unsafe { dest.as_bytes_mut() }, // Aliasing
        ).unwrap();

        dest
    }
}

impl KvCore for Session {
    async fn put_to_remote(&self, env: Arc<Env>) -> Result<(), KvError> {
        let data = SessionData {
            user_email: self.user_email.clone(),
            display_name: self.display_name.clone(),
        };

        let kv = env.kv("kSESSIONS").unwrap();
        let id_b64 = self.get_session_id_b64();

        kv.put(&*format!("session:{id_b64}"), data)
            .unwrap()
            .expiration_ttl(SESSION_MAX_AGE)
            .execute()
            .await
    }

    async fn delete_from_remote(self, env: Arc<Env>) -> Result<(), KvError> {
        let kv = env.kv("kSESSIONS").unwrap();
        let id_b64 = self.get_session_id_b64();

        kv.delete(&*format!("session:{id_b64}"))
            .await
    }

    async fn check(&self, env: Arc<Env>) -> Result<bool, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();
        let id_b64 = self.get_session_id_b64();

        let res = kv.get(&*format!("session:{id_b64}"))
            .cache_ttl(KV_CACHE_TTL)
            .json::<SessionData>()
            .await;

        res.map(|v| v.is_some())
    }
}

impl KvInterface<[u8; SESSION_ID_SIZE]> for Session {

    async fn fetch_from_remote(id: &[u8; SESSION_ID_SIZE], env: Arc<Env>) -> Result<Option<Self>, KvError> {
        let kv = env.kv("kSESSIONS").unwrap();

        let mut id_b64 = SmallString::<24>::new();
        BASE64_STANDARD.encode_slice(
            &id,
            unsafe { id_b64.as_bytes_mut() }, // Aliasing
        ).unwrap();

        let Some(data) = kv.get(&*format!("session:{id_b64}"))
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
}

pub enum GetSessionError {
    KvError(worker::KvError),
    D1Error(worker::D1Error),
    Unauthorised,
}

pub async fn get_session(
    cookie_jar: CookieManager,
    state: Arc<Env>
) -> Result<Session, GetSessionError> {

}

#[macro_export]
macro_rules! get_session_m {
    (
        with
            cookie_jar: $cookie:expr,
            state: $state:expr
    ) => {
        match (($cookie).get("ssid"), ($cookie).get("rftk")) {
            (Some(ssid), _) => {
                let mut decoded_ssid = [0u8; SESSION_ID_SIZE];
                BASE64_STANDARD.decode_slice(ssid.value(), &mut decoded_ssid).unwrap();

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
            (None, Some(rftk_cookie)) => {
                let mut rftk = [0u8; RFTK_SIZE];
                BASE64_STANDARD.decode_slice(rftk_cookie.value(), &mut rftk).unwrap();
                
                let rftk = match RefreshToken::owned_fetch_from_remote(rftk, ($state).env.clone()).await {
                    Ok(Some(rftk)) => rftk,
                    Ok(None) => return GenericResponse(
                        StatusCode::TEMPORARY_REDIRECT,
                        &[("Location", "/login")],
                        "Unauthorised (redirecting you to login)".to_string()
                    ),
                    Err(e) => return GenericResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "text/plain")],
                        format!("502 Bad Gateway (error fetching from KV): {e}")
                    )
                };

                // Quote of the year 2026:
                //     "God forbid gays socialize or have communities.
                //      I'm sorry you're incapable of interacting with anything but porn, anon.
                //      Go kill yourself, though. Ok? Bye, bitch."

                // AND A BONUS RESPONSE!!!!!!!!!!
                //     "this is literally a board for porn you retarded moron."

                // Fetch user

                let user = match User::fetch_from_remote(&UserIdentifyingKey::UserId(rftk.identifier().encode_b64()), ($state).env.clone()).await {
                    Ok(Some(user)) => user,
                    Ok(None) => return GenericResponse(
                        StatusCode::TEMPORARY_REDIRECT,
                        &[("Location", "/register")],
                        "No such user (redirecting you to register)".to_string()
                    ),
                    Err(e) => return GenericResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "text/plain")],
                        format!("502 Bad Gateway (error fetching from DB): {e}")
                    )
                };

                // Create new session

                let session = Session::new(user.email, user.display_name);

                // Push it to remote

                if let Err(e) = session.put_to_remote(($state).env.clone()).await {
                    return GenericResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "text/plain")],
                        format!("502 Bad Gateway (error putting to KV): {e}")
                    )
                };

                // Rotate refresh token
                let new_rftk = RefreshToken::new(*rftk.identifier());

                // Delete old token
                if let Err(e) = rftk.delete_from_remote(($state).env.clone()).await {
                    return GenericResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "text/plain")],
                        format!("502 Bad Gateway (error deleting from KV): {e}")
                    )
                };
                
                // Push new token
                if let Err(e) = new_rftk.put_to_remote(($state).env.clone()).await {
                    return GenericResponse(
                        StatusCode::BAD_GATEWAY,
                        &[("Content-Type", "text/plain")],
                        format!("502 Bad Gateway (error putting to KV): {e}")
                    )
                };
                
                let session_cookie = Cookie::new("ssid", session.get_session_id_b64())
                    .with_secure(true)
                    .with_http_only(true)
                    .with_max_age(Duration::from_secs(30 * 60))
                    .with_same_site(SameSite::Strict)
                    .with_path("/");

                // Set the session token cookie
                ($cookie).add(session_cookie);
                
                let refresh_token_cookie = Cookie::new("rftk", new_rftk.get_entropy_b64())
                    .with_secure(true)
                    .with_http_only(true)
                    .with_max_age(Duration::from_secs(45 * 24 * 60 * 60))
                    .with_same_site(SameSite::Strict)
                    .with_path("/");

                // Rotate the refresh token cookie
                ($cookie).add(refresh_token_cookie);

                session
            },
            (None, None) => return GenericResponse(
                StatusCode::TEMPORARY_REDIRECT,
                &[("Location", "/login")],
                "Unauthorised (redirecting you to login)".to_string()
            )
        }
    };
}
