use alloc::{format, string::ToString, sync::Arc, vec::Vec};
use alloc::string::String;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde_json::json;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKey, ReadableByteStreamController};
use worker::{Env, crypto::{DigestStream, DigestStreamAlgorithm}, js_sys::{self, Array, ArrayBuffer, Object, Reflect, Uint8Array, global}, Fetcher, RequestInit, Method, Request, Response};
use wasm_bindgen::{JsValue, convert::TryFromJsValue, prelude::Closure};


pub fn webcrypto() -> web_sys::Crypto {
    web_sys::Crypto::from(
        js_sys::Reflect::get(&global(), &"crypto".into())
            .unwrap()
    )
}

pub fn random_byte() -> u8 {
    let mut arr = [0u8];

    webcrypto().get_random_values_with_u8_array(&mut arr).unwrap();

    arr[0]
}

pub fn _random_bytes(count: u32) -> Vec<u8> {
    let mut arr = Vec::with_capacity(count as usize);

    webcrypto().get_random_values_with_u8_array(&mut arr).unwrap();

    arr
}

pub fn random_bytes_generic<const COUNT: usize>() -> [u8; COUNT] {
    let mut arr = [0u8; COUNT];

    webcrypto().get_random_values_with_u8_array(&mut arr).unwrap();

    arr
}

pub struct RandBytesIterator;

impl Iterator for RandBytesIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        Some(random_byte())
    }
}

pub async fn phash_stage_1<T>(
    password: T,
    salt: [u8; 32],
    env: Arc<Env>
) -> [u8; 32]
where
    T: AsRef<[u8]>,
{
    let store = env.secret_store("ARGON_SECRET").unwrap();

    // A "multitude" of secrets are used
    let pepper = BASE64_STANDARD.decode(env.secret("ARGON_PEPPER").unwrap().to_string()).unwrap();
    let mut secret = BASE64_STANDARD.decode(store.get().await.unwrap().unwrap()).unwrap();

    secret.extend(pepper);
    secret.extend(salt);

    // Secret is now full

    let data = [&*secret, password.as_ref()].concat();
    let digest = native_webcrypto_hash(DigestStreamAlgorithm::Sha256, data).await;

    digest.try_into().unwrap()
}

/// Calls stage 2 of the password hashing algorithm.
/// It returns the PHC string.
pub async fn call_phash_stage_2(
    stage_1_digest: [u8; 32],
    salt: [u8; 32],
    env: Arc<Env>,
) -> Result<String, worker::Error> {
    let mtls_fetcher = env.get_binding::<Fetcher>("PHASH_MTLS").unwrap();

    let body = json!({
        "stage_1_digest": stage_1_digest,
        "salt": salt,
    }).to_string();

    let mut req_init = RequestInit::new();
    req_init
        .with_method(Method::Post)
        .with_body(Some(JsValue::from_str(&*body.to_string())));

    let req =
        Request::new_with_init("https://phash.kolloquy.com", &req_init)
            .unwrap();

    let mut response =
        mtls_fetcher.fetch_request(req).await?;

    let mut response = Response::try_from(response)?;

    let phc = match response.status_code() {
        200 => response.text().await?,
        502 => return Err(worker::Error::RustError(
            "Bad gateway to offload server.".to_string()
        )),
        code => return Err(worker::Error::RustError(format!(
            "Unknown response from offload server (code {code}): {}",
            response.text().await?,
        )))
    };

    Ok(phc)
}

/// Compute the hash of a password using the 2 stage system.
pub async fn compute_password_hash<T>(
    password: T,
    salt: [u8; 32],
    env: Arc<Env>
) -> Result<String, worker::Error>
where
    T: AsRef<[u8]>,
{
    // Stage 1
    let stage_1 = phash_stage_1(password, salt, env.clone()).await;
    let phc = call_phash_stage_2(stage_1, salt, env.clone()).await?;

    Ok(phc)
}

/// Computes the hash of some data using the specified algorithm using the native WebCrypto API (available in workers)
pub async fn native_webcrypto_hash<T: AsRef<[u8]>>(algorithm: DigestStreamAlgorithm, data: T) -> Vec<u8> {
    let hasher = DigestStream::new(algorithm);

    let data_array = Uint8Array::new_from_slice(data.as_ref());
    let underlying_source = Object::new();

    // We need to

    let start_fn = Closure::once_into_js(move |controller: JsValue| {
        // Cast the value we receive from the closure into ReadableByteStreamController
        // This should always work because that is what the closure receives
        // If we do not do this, our code becomes a lot uglier
        let controller = ReadableByteStreamController::from(controller);

        controller.enqueue_with_js_u8_array(&data_array).unwrap();
        controller.close().unwrap();
    });

    // Set the start property of the underlying source to our start function
    // Sadly, there is no Object::set or similar function
    Reflect::set(
        &underlying_source,
        &JsValue::from_str("start"),
        &start_fn
    ).unwrap();

    // Get a ReadableStream from our underlying source which we can use to pipe data to the hasher
    let stream = web_sys::ReadableStream::new_with_underlying_source(&underlying_source).unwrap();

    // We do not need to await this promise, only the promise returned by .digest()
    let _ = stream.pipe_to(hasher.raw());

    let bytes = hasher.digest().await.unwrap().to_vec();

    bytes.try_into().unwrap()
}

/// Calculate the Hmac<Sha256> hash using only the native SubtleCrypto API.
pub async fn native_hmac_sha256<A: AsRef<[u8]>, B: AsRef<[u8]>>(key: A, data: B) -> Vec<u8> {
    let (key, data) = (key.as_ref(), data.as_ref());
    let key = Uint8Array::new_from_slice(key);

    let subtle = webcrypto().subtle();

    let algorithm = Object::new();
    Reflect::set(&algorithm, &JsValue::from_str("name"), &JsValue::from_str("HMAC")).unwrap();
    Reflect::set(&algorithm, &JsValue::from_str("hash"), &JsValue::from_str("SHA-256")).unwrap();

    let key_usages = Array::new();
    key_usages.push(&JsValue::from_str("sign"));
    key_usages.push(&JsValue::from_str("verify"));


    let key = JsFuture::from(subtle.import_key_with_object(
        "raw", // format = raw bytes
        &key,
        &algorithm,
        false,
        &key_usages
    ).unwrap()).await.unwrap();

    let key = CryptoKey::from(key);

    let signature = JsFuture::from(subtle.sign_with_str_and_u8_array(
        "HMAC",
        &key,
        &data
    ).unwrap()).await.unwrap();

    let signature_buffer = ArrayBuffer::try_from_js_value(signature).unwrap();

    Uint8Array::new(&signature_buffer).to_vec()
}
