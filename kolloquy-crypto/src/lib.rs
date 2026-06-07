pub mod phash;

use std::sync::Arc;

use base64::Engine;
use stack_string::SmallString;
use web_sys::{Crypto, CryptoKey};
use worker::{
    Env,
    js_sys::{self, Array, Object, Reflect, Uint8Array, global},
    wasm_bindgen::{JsValue, convert::js_value_vector_from_abi},
    wasm_bindgen_futures::JsFuture,
};

fn webcrypto() -> Crypto {
    Reflect::get(&global().into(), &"crypto".into()).unwrap().into()
}

pub async fn get_totp_wrapping_key(env: Arc<Env>) -> Result<CryptoKey, JsValue> {
    let mut key_bytes = [0u8; 32];

    let encoded_key_bytes: String = env.secret("TOTP_WRAPPING_KEY").unwrap().to_string();
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode_slice(encoded_key_bytes, &mut key_bytes)
        .unwrap();

    let key_bytes_array = Uint8Array::new_from_slice(&key_bytes);

    let key_usages = ["wrapKey", "unwrapKey"].into_iter().map(JsValue::from).collect::<Array>();

    let key = JsFuture::from(
        webcrypto()
            .subtle()
            .import_key_with_str("raw", &key_bytes_array, "AWS-KW", true, &key_usages)
            .unwrap(),
    )
    .await?;

    Ok(CryptoKey::from(key))
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    webcrypto().get_random_values_with_u8_array(&mut bytes).unwrap();
    bytes
}

pub async fn unwrap_totp_key(wrapped_secret: [u8; 32], env: Arc<Env>) -> Result<CryptoKey, JsValue> {
    let wrapping_key = get_totp_wrapping_key(env.clone()).await?;

    // TOTP secret is a HMAC-SHA1 key of length 160 bits
    let key_alg = {
        let obj = Object::new();
        Reflect::set_str(&obj, &"name".into(), &"HMAC".into()).unwrap();
        Reflect::set_str(&obj, &"hash".into(), &"SHA-1".into()).unwrap();
        obj
    };

    let usages = ["sign", "verify"].into_iter().map(JsValue::from).collect::<Array>();

    let key = JsFuture::from(webcrypto().subtle().unwrap_key_with_u8_array_and_str_and_object(
        "raw",
        &wrapped_secret,
        &wrapping_key,
        "AES-KW",
        &key_alg,
        false,
        &usages
    )?).await?;

    Ok(CryptoKey::from(key))
}

pub async fn wrap_totp_key(key: &CryptoKey, env: Arc<Env>) -> Result<[u8; 32], JsValue> {
    let bytes = JsFuture::from(webcrypto().subtle().wrap_key_with_str(
        "raw",
        &key,
        &get_totp_wrapping_key(env).await?,
        "AES-KW"
    )?).await?;

    let bytes = Uint8Array::from(bytes);

    let mut dst = [0u8; 32];

    // Don't you DARE heap-allocate on me
    // huh? everything above probably allocates?
    // huh? this is premature optimisation?
    // what? the unsafe block is probably a bigger problem than heap usage?
    // what? i cant even say memory usage is a problem because I have never ran this code?
    unsafe {
        let dst = dst.as_mut_ptr();
        bytes.raw_copy_to_ptr(dst);
    };

    Ok(dst)
}

