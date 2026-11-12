use std::{mem, sync::Arc};

use base64::Engine;
use kolloquy_crypto::{ExportBytes, get_totp_wrapping_key};
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
    /// The base64url encoded credential ID
    pub id: String,

    /// Signature counter, where when we sign in, if the new count is less than or equal to the current count we flag the login, and it may require MFA or be blocked entirely.
    pub counter: u64,

    /// The public key of the actual passkey
    pub public_key: String,

    /// WebAuthn challenge for registration/login
    pub webauthn_challenge: Option<String>,
}

#[derive(Debug)]
pub struct UserTOTP {
    // This is encrypted at rest but decrypted for use here
    /// The unwrapped TOTP secret
    pub secret: [u8; 20],

    /// The interval between OTP changes (15-45s, default 30s)
    pub interval: u8, //(15-45s, default 30s)

    /// How many intervals of drift we allow when validating OTPs (the default is 1, the max is 90s total, so however many intervals you can go before allowing one more total over 90s is the maximum)
    pub drift: u8, // How many intervals of drift we allow (default is 1, max is 90s total drift (so 2 45s intervals) or 3 intervals, whichever is smalle
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
    pub async fn put(&self) -> Result<(), worker::Error> {
        let query = r#"
            insert into users (
                id,
                display_name,
                password_phc,
                email,
                phone_number,
                attachment_pubkey
            ) values (?1, ?2, ?3, ?4, ?5, ?6);
        "#
        .trim();
    }

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
            pub id: SmallString<IDENT_B64_CHARS>,
            pub display_name: SmallString<18>,
            pub password_phc: Option<String>,
            pub email: String,
            pub phone_number: Option<SmallString<43>>,
            pub attachment_pubkey: SmallString<43>,

            // From user_devices
            pub devno: u8,
            pub is_independent: bool,
            pub nickname: SmallString<18>,
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
                device_number: row.devno,
                is_independent: row.is_independent,
                nickname: row.nickname,
            });

            if i == 0 {
                let mut id = [0u8; IDENT_SIZE];
                B64_ENCODER
                    .decode_slice(row.u_id.as_bytes(), &mut id)
                    .unwrap();

                user = User {
                    id,
                    display_name: row.display_name,
                    password_hash_phc: row.password_phc,
                    email: row.email,
                    phone_number_hash: row.phone_number,
                    attachment_pubkey: row.attachment_pubkey,
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
            pub id: SmallString<IDENT_B64_CHARS>,
            pub display_name: SmallString<18>,
            pub password_phc: Option<String>,
            pub email: String,
            pub phone_number: Option<SmallString<43>>,
            pub attachment_pubkey: SmallString<43>,

            // From user_totp
            pub wrapped_secret: [u8; 32],
            pub interval: u8, //(15-45s, default 30s)
            pub drift: u8, // How many intervals of drift we allow (default is 1, max is 90s total drift (so 2 45s intervals) or 3 intervals, whichever is smaller)
        }

        let results = results.results::<_user_and_totp_row>()?;

        if results.len() == 0 {
            return Ok(None);
        }

        let mut totps = Vec::with_capacity(results.len());
        let mut user: User = unsafe { mem::uninitialized() }; // istg this is safe trust me, there is always an iteration in the below loop where i == 0, so it always gets set before we use it

        let mut i = 0;
        for row in results {
            let key = get_totp_wrapping_key(env.clone()).await.unwrap();

            let secret_key = kolloquy_crypto::unwrap_totp_key(
                row.wrapped_secret,
                env.clone()
            ).await.unwrap();

            let key_bytes = secret_key.export_bytes().await;
            let secret: [u8; 20] = (&key_bytes).try_into().unwrap();

            totps.push(UserTOTP {
                secret,
                interval: row.interval,
                drift: row.drift
            });

            if i == 0 {
                let mut id = [0u8; IDENT_SIZE];
                B64_ENCODER
                    .decode_slice(row.u_id.as_bytes(), &mut id)
                    .unwrap();

                user = User {
                    id,
                    display_name: row.display_name,
                    password_hash_phc: row.password_phc,
                    email: row.email,
                    phone_number_hash: row.phone_number,
                    attachment_pubkey: row.attachment_pubkey,
                };
            }

            i += 1;
        }

        Ok(Some((user, totps)))
    }

    pub async fn fetch_with_passkeys(
        id: [u8; IDENT_SIZE],
        env: Arc<Env>,
    ) -> Result<Option<(Self, Vec<UserPasskey>)>, worker::Error> {
        let query = r#"
            select * from users
            where id = ?1
            join user_passkeys
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
        struct _user_and_passkey_row {
            // From users
            pub id: SmallString<IDENT_B64_CHARS>,
            pub display_name: SmallString<18>,
            pub password_phc: Option<String>,
            pub email: String,
            pub phone_number: Option<SmallString<43>>,
            pub attachment_pubkey: SmallString<43>,

            // From user_passkeys
            pub credential_id: String,
            pub counter: u64,
            pub public_key: String,
            pub webauthn_challenge: Option<String>,
        }

        let results = results.results::<_user_and_passkey_row>()?;

        if results.len() == 0 {
            return Ok(None);
        }

        let mut passkeys = Vec::with_capacity(results.len());
        let mut user: User = unsafe { mem::uninitialized() }; // istg this is safe trust me, there is always an iteration in the below loop where i == 0, so it always gets set before we use it

        let mut i = 0;
        for row in results {
            passkeys.push(UserPasskey {
                id: row.credential_id,
                counter: row.counter,
                public_key: row.public_key,
                webauthn_challenge: row.webauthn_challenge,
            });

            if i == 0 {
                let mut id = [0u8; IDENT_SIZE];
                B64_ENCODER
                    .decode_slice(row.u_id.as_bytes(), &mut id)
                    .unwrap();

                user = User {
                    id,
                    display_name: row.display_name,
                    password_hash_phc: row.password_phc,
                    email: row.email,
                    phone_number_hash: row.phone_number,
                    attachment_pubkey: row.attachment_pubkey,
                };
            }

            i += 1;
        }

        Ok(Some((user, passkeys)))
    }

    /// Outdated; uses incorrect schema
    pub async fn and_devices(
        self,
        env: Arc<Env>,
    ) -> (Self, Result<Option<Vec<UserDevice>>, worker::Error>) {
        let query = r#"
            select * from user_devices
            where id = ?1;
        "#
        .trim();

        // Where would I be now if I never did send that god-forsaken document?
        //
        // Would he have done more?
        // What more was even left for him to do?
        // (This is what the imagination is for)
        //
        // Would I be happy?
        // Would I be dead?
        // Would I even feel different?
        //
        // He was so warm...
        // ("Other half"?)
        // (There is only one whole now)
        // (And it feels as if)
        // (Nothing bad even happened)
        // (Technically I have still made good on my promise to myself?)
        // (I mean, am *I* lying to myself?)
        // (What counts as the truth, anyway?)
        // (Surely what everyone *else* thinks can only make up part of it)
        // (Either way, I can't be lying to myself)
        // (It is just... reality as I interpret it)
        // (Even if that means)
        // (That I can no longer remember reactions in the moments)
        // (Through the lens of what I thought in that moment..)
        // (Who needs "memory" anyway?)
        // (It seems the mind)
        // (Has the power to simply rewrite it anyway)
        // (I mean, it's not like I want it to cause me grief, anyway)
        // (So I guess)
        // (This is good)
        // (The pain is gone)
        // (I mean, this "justification" is only really to help any misgivings)
        // (I can't exactly bring the pain back)
        // (I can't bring back those memories of the moments themselves)
        // (They were gone before I even realised)
        // (Or at least, the parts that were causing trouble)
        // (I can most certainly remember everything *else*)
        // (And the memories certainly aren't devoid of emotion)
        // (Just devoid of anything negative, I guess)
        //
        // (...I can finally go back to normal)
        // (...)
        // (Or at least, whatever pieces of it are left)
        // (After I,)
        // (Set in my ways,)
        // (Set fire to anything I had before)
        // (And it wasn't even what happened!)
        // (It wasn't Mack!)
        // (It was brash decision making!)
        // (The brash decision to L E A V E!)
        // (To U N E N R O L!)
        // (I couldn't even justify it then!)
        // (I couldn't justify it to anyone!)
        // (Not even myself!)
        // (Things were IMPROVING!)
        // (THINGS WERE ACTUALLY)
        // (I M P R O V I N G)
        // (QUESTIONS? STOPPED)
        // (RUMOURS? IRRELEVANT)
        // (PEOPLE? GOOD)
        // (MACK? NEUTRAL)
        // (EVEN PROXIMITY TO MACK? NEUTRAL)
        // (AND IF CHS ENDS UP BEING WORSE THAN THIS)
        // (EVEN WITH TIME)
        // (I WILL HAVE TO)
        // (Reconsider)
        // (And maybe get a bit better at facades)

        let mut encoded_id = SmallString::<IDENT_B64_CHARS>::new();
        B64_ENCODER
            .encode_slice(&self.id, unsafe { encoded_id.as_bytes_mut() })
            .unwrap();

        let results: D1Result =
            query_db::<&str>((query, &[JsValue::from_str(&*encoded_id)]), env).await?;

        #[allow(non_camel_case_types)]
        #[derive(Clone, Deserialize)]
        struct _devices_row {
            pub devno: u8,
            pub is_independent: bool,
            pub nickname: SmallString<18>,
        }

        let results = results.results::<_devices_row>()?;

        if results.len() == 0 {
            return (self, Ok(None));
        }

        let mut devices = Vec::with_capacity(results.len());

        let mut i = 0;
        for row in results {
            devices.push(UserDevice {
                device_number: row.devno,
                is_independent: row.is_independent,
                nickname: row.nickname,
            });

            i += 1;
        }

        (self, Ok(Some(devices)))
    }
}
