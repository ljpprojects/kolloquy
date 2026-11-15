//! Module that interfaces with the password hashing system

use kolloquy_consts::{
    KOLLOQUY_VERSION_STR, PASS1_DIGEST_SIZE, PASS1_PEPPER_SECRET_NAME, PASS1_PEPPER_SIZE, PASS1_SALT_SIZE, PASS2_SALT_SIZE, PHASH_MAGIC_NUM_010, PHASH_MTLS_CLIENT_NAME, base64_decode, base64_decode_slice, base64_decode_slice_safe, base64url_rev_alphabet,
};
use stack_string::SmallString;
use std::{ops::Mul, sync::Arc};

#[cfg(feature = "no-phash-serv")]
mod _no_phash_serv {
    use kolloquy_consts::{PASS1_DIGEST_SIZE, PASS2_SALT_SIZE};

    pub fn compute_stage_2_digest(
        digest_1: [u8; PASS1_DIGEST_SIZE],
        salt_2: [u8; PASS2_SALT_SIZE],
        state: Arc<ServerState>,
    ) -> PasswordHash {
        let assoc_data = AssociatedData::new(&PHASH_MAGIC_NUM_010.to_be_bytes()).unwrap();

        let alg = Algorithm::Argon2id;
        let ver = Version::V0x13;

        let params = ParamsBuilder::new() // We are only computing these like once ever for each user so it is fine to maybe go a bit overboard with parameters (like literally once ever hopefully, we can cache the hashes and nginx can serve those and the request hopefully never touches the server) but it might still be a bit hard on my poor 4GB-ram Pi 5...
            .data(assoc_data)
            .output_len(PASS2_DIGEST_SIZE)
            .m_cost(2u32.pow(17)) // 128MiB
            .t_cost(3)
            .p_cost(2)
            .build()
            .unwrap();

        let mut pepper = [0u8; PASS2_PEPPER_SIZE];
        STANDARD.decode_slice_unchecked(env::var(PASS2_PEPPER_VAR_NAME).unwrap(), &mut pepper).unwrap(); // envcheck ensures that the PASS2_PEPPER is valid base64 and the correct length. Also shouldn't decode_slice_unchecked be marked unsafe...? (Apparently it shouldn't, as it panics if the output buf is too small, rather than causing UB, but unchecked is misleading, because it is checked, it just fails differently)

        let mut full_secret = Zeroizing::new([0u8; PASS2_PEPPER_SIZE + PASS2_THYME_SIZE]);
        (&mut full_secret[..PASS2_THYME_SIZE]).copy_from_slice(&**state.thyme);
        (&mut full_secret[PASS2_THYME_SIZE..]).copy_from_slice(&pepper);

        let ctx = Argon2::new_with_secret(&*full_secret, alg, ver, params).unwrap();

        ctx.hash_password_with_salt(&digest_1, &salt_2).unwrap()
    }
}

#[cfg(feature = "use-phash-serv")]
mod _use_phash_serv {
    use kolloquy_consts::{KOLLOQUY_VERSION_STR, PASS1_DIGEST_SIZE, PASS2_SALT_SIZE, PHASH_MTLS_CLIENT_NAME, PHASH_MTLS_ROOT_NAME, PHASH_SERV_URL_NAME, SERVER_VERSION_HEADER, base64_decode_slice, base64_encode_slice, base64url_alphabet};
    use reqwest::{Method, Url, header::HeaderValue};
    use stack_string::SmallString;
    use kolloquy_phash::api::{res_body_to_result, PHashError};

    pub enum PhashStage2Error {
        ReqwestError(reqwest::Error),
        PhashError(PHashError)
    }

    pub async fn call_stage_2_phash(
        password_stage_1: [u8; PASS1_DIGEST_SIZE],
        salt: [u8; PASS2_SALT_SIZE],
    ) -> Result<phc::PasswordHash, PhashStage2Error> {
        let mut serv_url = Url::try_from(
            &*std::env::var(PHASH_SERV_URL_NAME).unwrap()
        ).unwrap();

        let mut b64_digest =
            SmallString::<{ (PASS1_DIGEST_SIZE * 8).div_ceil(6) }>::new();

        let mut b64_salt =
            SmallString::<{ (PASS2_SALT_SIZE * 8).div_ceil(6) }>::new();

        base64_encode_slice(
            &password_stage_1,
            unsafe { b64_digest.as_bytes_mut() },
            base64url_alphabet()
        );

        base64_encode_slice(
            &salt,
            unsafe { b64_salt.as_bytes_mut() },
            base64url_alphabet()
        );

        serv_url.path_segments_mut().unwrap().extend([&*b64_digest, &*b64_salt]);

        let mut req = reqwest::Request::new(Method::GET, serv_url);
        req.headers_mut().insert(
            SERVER_VERSION_HEADER,
            HeaderValue::from_static(KOLLOQUY_VERSION_STR)
        );

        let root_cert =
            tokio::fs::read(std::env::var(PHASH_MTLS_ROOT_NAME).unwrap())
                .await
                .unwrap();

        let root_cert = reqwest::Certificate::from_pem(&root_cert).unwrap();

        let identity = tokio::fs::read(std::env::var(PHASH_MTLS_CLIENT_NAME).unwrap())
            .await
            .unwrap();

        let identity = reqwest::Identity::from_pem(&identity).unwrap();

        let mtls_client = reqwest::Client::builder()
            .tls_backend_rustls()
            .tls_certs_merge([root_cert])
            .identity(identity)
            .https_only(true)
            .build()
            .map_err(|e| PhashStage2Error::ReqwestError(e))?;

        let res = mtls_client.execute(req)
            .await
            .map_err(|e| PhashStage2Error::ReqwestError(e))?;
        let body = res.text()
            .await
            .map_err(|e| PhashStage2Error::ReqwestError(e))?;

        res_body_to_result(&body).map_err(|e| PhashStage2Error::PhashError(e))

        // 23 + 29.90 + 19.93 + 26.62 + 18.24 + 6 + 9.58 + 9.75 + 8.12 = $151.14
        //
        // 23 + 29.90 + 19.93 + 17.56 + 18.24 + 6 + 9.58 + 16.49 + 8.12 = $148.82
        //
        // 23 + 59 + 17.56 + 18.24 + 6 + 16.49 + 8.12 = $148.41
        //
        // 79 + 28.68 = $107.68
        //
        // 6017B0585001
        //
        // 29.90 + 19.93 + 9.58 = $59.41
    }
}

pub enum PhashStage1Error {
    PepperTooSmall,
    PepperTooLarge
}

/// Run stage 1 of the password hashing algorithm.
pub async fn stage_1_phash(
    password: &[u8],
    salt: [u8; PASS1_SALT_SIZE],
) -> Result<[u8; PASS1_DIGEST_SIZE], PhashStage1Error> {
    let mut pepper = [0u8; PASS1_PEPPER_SIZE];
    let b64_pepper = std::env::var(PASS1_PEPPER_SECRET_NAME).unwrap();
    match base64_decode_slice_safe(
        b64_pepper.as_bytes(),
        &mut pepper,
        base64url_rev_alphabet()
    ) {
        Some(n) if n == PASS1_PEPPER_SIZE => {},
        Some(n) if n < PASS1_PEPPER_SIZE
            => return Err(PhashStage1Error::PepperTooSmall),
        None => return Err(PhashStage1Error::PepperTooLarge),
        _ => unreachable!("if decoding worked then n <= PASS1_PEPPER_SIZE")
    }

    // [0..8] = magic number (BE)
    // [8..(8 + PASS1_SALT_SIZE)] = salt
    // [(8 + PASS1_SALT_SIZE)..(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)] = pepper
    // [(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)..] = password
    // size = 8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE +
    let mut data = Vec::with_capacity(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE + password.len());
    (&mut data[0..8]).copy_from_slice(&PHASH_MAGIC_NUM_010.to_be_bytes());
    (&mut data[8..(8 + PASS1_SALT_SIZE)]).copy_from_slice(&salt);
    (&mut data[(8 + PASS1_SALT_SIZE)..(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)])
        .copy_from_slice(&pepper);

    (&mut data[(8 + PASS1_SALT_SIZE + PASS1_PEPPER_SIZE)..]).copy_from_slice(password);

    let mut digest = [0u8; PASS1_DIGEST_SIZE];
    match PASS1_DIGEST_SIZE {
        28 => digest.copy_from_slice(&openssl::sha::sha224(&data)),
        32 => digest.copy_from_slice(&openssl::sha::sha256(&data)),
        48 => digest.copy_from_slice(&openssl::sha::sha384(&data)),
        64 => digest.copy_from_slice(&openssl::sha::sha512(&data)),
        _ => unreachable!(), // There is a static check for this in kolloquy-consts already
    }

    Ok(digest)
}