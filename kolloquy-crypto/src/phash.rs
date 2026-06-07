//! Module that interfaces with the password hashing system

use std::sync::Arc;
use base64::{Engine, prelude::BASE64_STANDARD};
use kolloquy_consts::{KOLLOQUY_VERSION_STR, PHASH_MTLS_SECRET_NAME};
use stack_string::SmallString;
use worker::{Env, Fetcher, Headers, Method, RequestInit};

pub const PHASH_URI: &str = "https://phash.kolloquy.com";

pub async fn compute_phash(password_stage_1: [u8; 32], salt: [u8; 32], env: Arc<Env>) -> Result<phc::PasswordHash, worker::Error> {
    let headers = Headers::from_iter([
        ("X-Kolloquy-Server-Version", KOLLOQUY_VERSION_STR), // The PHash server rejects requests from old server versions
        ("Content-Type", "application/json"),
    ]);

    let mut b64_digest = SmallString::<44>::new();
    BASE64_STANDARD.encode_slice(
        &password_stage_1,
        unsafe { b64_digest.as_bytes_mut() }
    ).unwrap();

    let mut b64_salt = SmallString::<44>::new();
    BASE64_STANDARD.encode_slice(
        &salt,
        unsafe { b64_salt.as_bytes_mut() }
    ).unwrap();

    let body = format!(r#"{{"stage_1_digest":"{b64_digest}",salt:"{b64_salt}"}}"#);

    let mut init = RequestInit::new();
    init
        .with_headers(headers)
        .with_method(Method::Post)
        .with_body(Some((&*body).into()));

    let mtls = env.get_binding::<Fetcher>(PHASH_MTLS_SECRET_NAME).unwrap();
    let res = mtls.fetch(PHASH_URI, Some(init)).await?;


    Ok()
}