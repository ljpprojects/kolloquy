use std::{mem, sync::Arc};

use base64::Engine;
use serde::Deserialize;
use stack_string::SmallString;
use web_sys::CryptoKey;
use worker::{D1Result, Env, wasm_bindgen::JsValue};

use crate::{B64_ENCODER, query::query_db};

pub const IDENT_SIZE: usize = 30;
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
    pub id: [u8; IDENT_SIZE],
    pub display_name: SmallString<18>,
    pub password_hash_phc: Option<String>,
    pub email: String,
    pub phone_number_hash: Option<SmallString<43>>,
    pub attachment_pubkey: SmallString<43>,
}

impl User {
    pub async fn fetch(id: [u8; IDENT_SIZE], env: Arc<Env>) -> Result<Option<Self>, worker::Error> {
        let query = r#"
            select * from users
            where id = ?1
        "#
        .trim();

        let mut encoded_id = SmallString::<IDENT_B64_CHARS>::new();
        B64_ENCODER
            .encode_slice(&id, unsafe { encoded_id.as_bytes_mut() })
            .unwrap();

        let results: D1Result =
            query_db::<&str>((query, &[JsValue::from_str(&*encoded_id)]), env).await?;

        #[allow(non_camel_case_types)]
        #[derive(Deserialize)]
        struct _user_row {
            pub id: SmallString<IDENT_B64_CHARS>,
            pub display_name: SmallString<18>,
            pub password_phc: Option<String>,
            pub email: String,
            pub phone_number: Option<SmallString<43>>,
            pub attachment_pubkey: SmallString<43>,
        }

        let results = results.results::<_user_row>()?;
        let Some(result) = results.into_iter().next() else {
            return Ok(None);
        };

        let mut id = [0u8; IDENT_SIZE];
        B64_ENCODER
            .decode_slice(result.id.as_bytes(), &mut id)
            .unwrap();

        Ok(Some(Self {
            id,
            display_name: result.display_name,
            password_hash_phc: result.password_phc,
            email: result.email,
            phone_number_hash: result.phone_number,
            attachment_pubkey: result.attachment_pubkey,
        }))
    }

    pub async fn fetch_with_devices(
        id: [u8; IDENT_SIZE],
        env: Arc<Env>,
    ) -> Result<Option<(Self, Vec<UserDevice>)>, worker::Error> {
        let query = r#"
            select * from users
            where id = ?1
            join user_devices
            using (id);
        "#
        .trim();

        let mut encoded_id = SmallString::<IDENT_B64_CHARS>::new();
        B64_ENCODER
            .encode_slice(&id, unsafe { encoded_id.as_bytes_mut() })
            .unwrap();

        let results: D1Result =
            query_db::<&str>((query, &[JsValue::from_str(&*encoded_id)]), env).await?;

        #[allow(non_camel_case_types)]
        #[derive(Clone, Deserialize)]
        struct _user_and_devices_row {
            // From users
            pub u_id: SmallString<IDENT_B64_CHARS>,
            pub u_display_name: SmallString<18>,
            pub u_password_phc: Option<String>,
            pub u_email: String,
            pub u_phone_number: Option<SmallString<43>>,
            pub u_attachment_pubkey: SmallString<43>,

            // From user_devices
            pub d_devno: u8,
            pub d_is_independent: bool,
            pub d_nickname: SmallString<18>,
        }

        let results = results.results::<_user_and_devices_row>()?;

        if results.len() == 0 {
            return Ok(None);
        }

        let mut devices = Vec::with_capacity(results.len());
        let mut user: User = unsafe { mem::uninitialized() }; // istg this is safe trust me

        let mut i = 0;
        for row in results {
            devices.push(UserDevice {
                device_number: row.d_devno,
                is_independent: row.d_is_independent,
                nickname: row.d_nickname,
            });

            if i == 0 {
                let mut id = [0u8; IDENT_SIZE];
                B64_ENCODER
                    .decode_slice(row.u_id.as_bytes(), &mut id)
                    .unwrap();

                user = User {
                    id,
                    display_name: row.u_display_name,
                    password_hash_phc: row.u_password_phc,
                    email: row.u_email,
                    phone_number_hash: row.u_phone_number,
                    attachment_pubkey: row.u_attachment_pubkey,
                };
            }

            i += 1;
        }

        Ok(Some((user, devices)))
    }

    pub async fn fetch_with_totp(
        id: [u8; IDENT_SIZE],
        env: Arc<Env>,
    ) -> Result<Option<(Self, Vec<UserTOTP>)>, worker::Error> {
        let query = r#"
            select * from users
            where id = ?1
            join users_totp
            using (id);
        "#
        .trim();

        let mut encoded_id = SmallString::<IDENT_B64_CHARS>::new();
        B64_ENCODER
            .encode_slice(&id, unsafe { encoded_id.as_bytes_mut() })
            .unwrap();

        let results: D1Result =
            query_db::<&str>((query, &[JsValue::from_str(&*encoded_id)]), env).await?;

        #[allow(non_camel_case_types)]
        #[derive(Clone, Deserialize)]
        struct _user_and_totp_row {
            // From users
            pub u_id: SmallString<IDENT_B64_CHARS>,
            pub u_display_name: SmallString<18>,
            pub u_password_phc: Option<String>,
            pub u_email: String,
            pub u_phone_number: Option<SmallString<43>>,
            pub u_attachment_pubkey: SmallString<43>,

            // From user_totp
            pub t_wrapped_secret: [u8; 32],
            pub t_interval: u8, //(15-45s, default 30s)
            pub t_drift: u8, // How many intervals of drift we allow (default is 1, max is 90s total drift (so 2 45s intervals) or 3 intervals, whichever is smaller)
        }

        let results = results.results::<_user_and_devices_row>()?;

        if results.len() == 0 {
            return Ok(None);
        }

        let mut devices = Vec::with_capacity(results.len());
        let mut user: User = unsafe { mem::uninitialized() }; // istg this is safe trust me

        let mut i = 0;
        for row in results {
            devices.push(UserDevice {
                device_number: row.d_devno,
                is_independent: row.d_is_independent,
                nickname: row.d_nickname,
            });

            if i == 0 {
                let mut id = [0u8; IDENT_SIZE];
                B64_ENCODER
                    .decode_slice(row.u_id.as_bytes(), &mut id)
                    .unwrap();

                user = User {
                    id,
                    display_name: row.u_display_name,
                    password_hash_phc: row.u_password_phc,
                    email: row.u_email,
                    phone_number_hash: row.u_phone_number,
                    attachment_pubkey: row.u_attachment_pubkey,
                };
            }

            i += 1;
        }

        Ok(Some((user, devices)))
    }
}
