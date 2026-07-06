//! Module that interfaces with the password hashing system

use std::{ops::Mul, sync::Arc};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD, prelude::BASE64_STANDARD};
use kolloquy_consts::{KOLLOQUY_VERSION_STR, PASS1_DIGEST_SIZE, PASS1_PEPPER_SECRET_NAME, PASS1_PEPPER_SIZE, PASS1_SALT_SIZE, PASS2_SALT_SIZE, PHASH_MAGIC_NUM_010, PHASH_MTLS_BINDING_NAME};
use kolloquy_phash::api::res_body_to_result;
use stack_string::SmallString;
use worker::{CacheMode, CfProperties, Env, Fetcher, Headers, Method, RequestInit, js_sys::Uint8Array, wasm_bindgen_futures::JsFuture};

use crate::webcrypto;

pub const PHASH_URI: &str = "https://phash.kolloquy.com";

/// Run stage 1 of the password hashing algorithm.
pub async fn stage_1_phash(password: &[u8], salt: [u8; PASS1_SALT_SIZE], env: Arc<worker::Env>) -> Result<[u8; PASS1_DIGEST_SIZE], worker::Error> {
    let alg = const { match PASS1_DIGEST_SIZE {
        32 => "SHA-256",
        48 => "SHA-384",
        64 => "SHA-512",
        _ => panic!("The PASS1_DIGEST_SIZE should be either 32, 48, or 64.")
    } };

    let mut pepper = [0u8; PASS1_PEPPER_SIZE];
    let b64_pepper = env.secret(PASS1_PEPPER_SECRET_NAME)?.to_string();
    match BASE64_STANDARD.decode_slice(b64_pepper, &mut pepper) {
        Ok(n) if n == PASS1_PEPPER_SIZE => (),
        Ok(_) => return Err(worker::Error::RustError(format!(
            "The PASS1_PEPPER is smaller than {PASS1_PEPPER_SIZE} bytes."
        ))),

        Err(base64::DecodeSliceError::OutputSliceTooSmall) =>
            return Err(worker::Error::RustError(format!(
                "The PASS1_PEPPER is larger than {PASS1_PEPPER_SIZE} bytes."
            ))),

        Err(base64::DecodeSliceError::DecodeError(e)) =>
            return Err(worker::Error::RustError(
                format!("The PASS1_PEPPER is not valid base64: {e}.")
            )),
    };

    // [0..8] = magic number (BE)
    // [8..(8 + PASS1_SALT_SIZE)] = salt
    // [(8 + PASS1_SALT_SIZE)..(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)] = pepper
    // [(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)..] = password
    // size = 8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE +
    let data = Vec::with_capacity(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE + password.len());
    (&mut data[0..8]).copy_from_slice(&PHASH_MAGIC_NUM_010.to_be_bytes());
    (&mut data[8..(8 + PASS1_SALT_SIZE)]).copy_from_slice(&salt);
    (&mut data[(8 + PASS1_SALT_SIZE)..(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)])
        .copy_from_slice(&pepper);

    (&mut data[(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)..])
        .copy_from_slice(password);

    let hash = webcrypto()
        .subtle()
        .digest_with_str_and_u8_array(alg, &data)
        .unwrap();

    // Imagining [TOP] ing is good and all... to a point
    // Eventually the [[ BOTTOM]]ing comes out again

    let hash = JsFuture::from(hash).await.unwrap();
    let hash_arr = Uint8Array::new(&hash);
    let mut hash = [0u8; PASS1_DIGEST_SIZE];
    hash_arr.copy_to(&mut hash); // hash_arr.byte_length() is always PASS1_DIGEST_SIZE
    Ok(hash)
}

pub async fn compute_phash(password_stage_1: [u8; PASS1_DIGEST_SIZE], salt: [u8; PASS2_SALT_SIZE], env: Arc<Env>) -> Result<phc::PasswordHash, worker::Error> {
    let headers = Headers::from_iter([
        ("X-Kolloquy-Server-Version", KOLLOQUY_VERSION_STR), // The PHash server rejects requests from older (or newer) server versions
    ]); // "Ten feet twenty the flower man"

    let mut b64_digest = SmallString::<{PASS1_DIGEST_SIZE.mul(8).div_ceil(6)}>::new();
    URL_SAFE_NO_PAD.encode_slice(
        &password_stage_1,
        unsafe { b64_digest.as_bytes_mut() }
    ).unwrap();

    let mut b64_salt = SmallString::<{PASS2_SALT_SIZE.mul(8).div_ceil(6)}>::new();
    URL_SAFE_NO_PAD.encode_slice(
        &salt,
        unsafe { b64_salt.as_bytes_mut() }
    ).unwrap();

    let uri = format!("{PHASH_URI}/{b64_digest}/{b64_salt}");

    let mut props = CfProperties::new();

    // We can also cache via Cloudflare, hopefully avoiding ever reaching phash at all
    props.cache_key = Some(uri);

    let mut init = RequestInit::new();
    init
        .with_headers(headers)
        .with_cf_properties(props)
        .with_method(Method::Get)
        .with_body(None);

    let mtls = env.get_binding::<Fetcher>(PHASH_MTLS_BINDING_NAME).unwrap();
    let res = mtls.fetch(PHASH_URI, Some(init)).await?;
    let body = res.text().await?;

    // res_body_to_result does the conversion for us (i.e. I wrote it somplace else already)
    // 3 hours of sleep btw
    res_body_to_result(&body).map_err(|e| worker::Error::RustError(e.to_string()))
}