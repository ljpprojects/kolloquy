use core::fmt::Write;
use alloc::{string::{ToString}, sync::Arc};
use const_format::formatcp;
use http::StatusCode;
use wasm_bindgen::JsValue;
use worker::{crypto::DigestStreamAlgorithm, *};
use serde_json::json;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::Serialize;
use arrayvec::ArrayString;
use crate::crypto::{native_hmac_sha256, native_webcrypto_hash};

/// Call the AWS SES API and return the response
pub async fn send_ses_mail(
    env: Arc<Env>,
    from: &str,
    recipient: &str,
    subject: &str,
    content: &str,
) -> Result<Response> {
    // Allocation here is inevitable
    let access_key =
        env.secret("AWS_ACCESS_KEY_ID").unwrap().to_string();
    let secret_key =
        env.secret("AWS_SECRET_ACCESS_KEY").unwrap().to_string();

    const REGION: &str = "ap-southeast-2";
    const SERVICE: &str = "ses";

    const HOST: &str = formatcp!("email.{REGION}.amazonaws.com");
    const URL: &str = formatcp!("https://{HOST}/v2/email/outbound-emails");

    let mut body = ArrayString::<{ 1024 * 1024 }>::new();
    let body_json = json!({
        "FromEmailAddress": from,
        "Destination": { "ToAddresses": [recipient] },
        "Content": {
            "Simple": {
                "Subject": { "Data": subject },
                "Body": { "Html": { "Data": content } }
            }
        }
    });

    write!(
        body,
        "{body_json}"
    ).unwrap();

    let now = DateTime::<Utc>::from_timestamp_millis(Date::now().as_millis() as i64).unwrap();

    // ISO 8601
    let mut amz_date = ArrayString::<16>::new();
    write!(
        &mut amz_date,
        "{YR}{M:02}{D:02}T{H:02}{Mn:02}{S:02}Z",
        YR = now.year(),
        M = now.month(),
        D = now.day(),
        H = now.hour(),
        Mn = now.minute(),
        S = now.second(),
    ).unwrap();

    // %Y%m%d
    let mut date_string = ArrayString::<8>::new();
    write!(
        &mut date_string,
        "{YR}{M:02}{D:02}",
        YR = now.year(),
        M = now.month(),
        D = now.day(),
    ).unwrap();

    let mut body_hash = ArrayString::<64>::zero_filled();

    unsafe {
        console_warn!("body_hash.as_bytes_mut().len() = {}", body_hash.as_bytes_mut().len());
    }

    hex::encode_to_slice(
        native_webcrypto_hash::<32, _>(DigestStreamAlgorithm::Sha256, body.as_str()).await.unwrap(),
        unsafe { body_hash.as_bytes_mut() } // Aliasing here
    ).unwrap();

    const HEADER_KEYS: [&'static str; 4] = ["content-type", "host", "x-amz-content-sha256", "x-amz-date"];
    // 12 + 1 + 4 + 1 + 20 + 1 + 10
    // 49
    const HEADER_KEYS_JOINED: &'static str = formatcp!(
        "{};{};{};{}",
        HEADER_KEYS[0],
        HEADER_KEYS[1],
        HEADER_KEYS[2],
        HEADER_KEYS[3]
    );

    let header_values: [&str; 4] = [
        "application/json",
        HOST,
        body_hash.as_str(),
        amz_date.as_str()
    ];

    // Length for sorted_headers
    // 12 + 16 + 1 = 29
    // 4 + 34 + 1 = 39
    // 20 + 64 + 1 = 85
    // 10 + 20 + 1 = 30
    // TOTAL: 184 (?)

    let mut sorted_headers = ArrayString::<184>::new();

    let mut i = 0;
    for header_key in HEADER_KEYS {
        sorted_headers.push_str(header_key);
        sorted_headers.push_str(":");
        sorted_headers.push_str(header_values[i]);
        sorted_headers.push_str("\n");

        i += 1;
    }

    console_debug!("sorted_headers = {}", sorted_headers);

    // 32 + 185 + 50 + 64
    let mut canonical_req = ArrayString::<331>::new();
    write!(
        &mut canonical_req,
        "POST\n/v2/email/outbound-emails\n\n{}\n{}\n{body_hash}",
        sorted_headers,
        HEADER_KEYS_JOINED,
    ).unwrap();

    let mut canonical_req_hash = ArrayString::<64>::zero_filled();
    hex::encode_to_slice::<[u8; 32]>(
        native_webcrypto_hash(DigestStreamAlgorithm::Sha256, canonical_req.as_bytes()).await.unwrap(),
        unsafe { canonical_req_hash.as_bytes_mut() } // Aliasing here
    ).unwrap();

    let mut scope = ArrayString::<40>::new();
    write!(
        &mut scope,
        "{date_string}/{REGION}/ses/aws4_request"
    ).unwrap();

    // Why does AWS need to use such an AI-style, vague, general name as StringToSign???
    // Like it has a meaning, right?????

    let mut string_that_shall_be_signed_in_due_time_and_is_named_in_unequivocally_unspecific_aws_fashion
         = ArrayString::<141>::new();
    write!(
        &mut string_that_shall_be_signed_in_due_time_and_is_named_in_unequivocally_unspecific_aws_fashion,
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{canonical_req_hash}"
    ).unwrap();

    let mut date_key = ArrayString::<44>::new();
    write!(date_key, "AWS4{secret_key}").unwrap();

    let date_key = native_hmac_sha256(
        date_key.as_bytes(),
        date_string.as_bytes(),
    ).await;

    let date_region_key = native_hmac_sha256(
        date_key,
        REGION,
    ).await;

    let date_region_service_key = native_hmac_sha256(
        date_region_key,
        SERVICE,
    ).await;

    let signing_key = native_hmac_sha256(
        date_region_service_key,
        "aws4_request",
    ).await;

    // maybe an ALLOCATION?
    let signature =
        native_hmac_sha256(
            signing_key,
            string_that_shall_be_signed_in_due_time_and_is_named_in_unequivocally_unspecific_aws_fashion
                .as_bytes()
        ).await;

    let mut sig_hex = ArrayString::<64>::zero_filled();
    hex::encode_to_slice(
        signature,
        unsafe { sig_hex.as_bytes_mut() }, // Aliasing
    ).unwrap();

    let mut authorisation = ArrayString::<228>::new();

    // 28 + 20 + 1 + 40 + 15 + 49 + 11 + 64

    write!(
        &mut authorisation,
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope},SignedHeaders={},Signature={}",
        HEADER_KEYS_JOINED,
        sig_hex,
    ).unwrap();

    console_log!("authorisation = {authorisation}");

    // Create a request
    // Let's hope to GOD this doesnt allocate
    let mut req_init = RequestInit::new();
    req_init
        .with_method(Method::Post)
        .with_body(Some(JsValue::from_str(&*body)));

    // FUCK THIS ALLOCATES NOOOOOOOOOOOOOOOOO
    let mut i = 0;
    for header_name in HEADER_KEYS {
        req_init.headers.set(header_name, header_values[i]).unwrap();
        i += 1;
    }

    req_init.headers.set("authorization", &*authorisation).unwrap();

    let req = Request::new_with_init(URL, &req_init).unwrap();

    let mut res = Fetch::Request(req)
        .send()
        .await?;

    if !StatusCode::from_u16(res.status_code()).unwrap().is_success() {
        return Err(Error::RustError(res.text().await.unwrap()))
    }

    Ok(res)
}
