use base64::Engine;
use stack_string::SmallString;
use web_sys::{Crypto, CryptoKey};
use worker::{
    Env,
    js_sys::{Array, Uint8Array},
    wasm_bindgen::JsValue,
    wasm_bindgen_futures::JsFuture,
};

fn webcrypto() -> Crypto {
    Reflect::get(&global(), &"crypto".into()).unwrap()
}

fn js_array_from_slice<const L: usize, T: Into<JsValue>>(slice: [T; L]) -> Array<JsValue> {
    let arr = Array::new_with_length(slice.len() as u32);

    for i in 0..L {
        arr.set(i as u32, slice[i].into())
    }

    arr
}

pub async fn get_totp_wrapping_key(env: Arc<Env>) -> Result<CryptoKey, JsValue> {
    let mut key_bytes = [0u8; 32];
    let encoded_key_bytes: String = env.secret("TOTP_WRAPPING_KEY").unwrap().to_string();
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode_slice(encoded_key_bytes, &mut key_bytes)
        .unwrap();

    let key_bytes_array = Uint8Array::new_from_slice(&key_bytes);
    let key_usages = js_array_from_slice(["wrapKey", "unwrapKey"]);

    let key = JsFuture::from(
        webcrypto()
            .subtle()
            .import_key_with_str("raw", &*key_bytes_array, "AWS-KW", true, &*key_usages)
            .unwrap(),
    )
    .await?;

    Ok(CryptoKey::from(key))
}

pub fn unwrap_totp_key(wrapped_secret: SmallString<43>) -> Result<[u8; 20], JsValue> {
    let mut decoded_wrapped =
}
