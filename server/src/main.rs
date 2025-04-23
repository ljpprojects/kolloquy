pub(crate) mod user;
pub(crate) mod data;
mod logging;

use crate::data::{KolloquyDB, KolloquyR2, QueryError};
use crate::logging::{LoggingFormat, LoggingMiddleware, LoggingPersistence};
use crate::user::{AuthenticateBody, RegisterBody, User, UserQuery};
use ammonia::UrlRelative;
use std::io::Cursor;
use async_std::path::PathBuf;
use async_std::sync::{Arc, RwLock};
use awscreds::Credentials;
use base64::alphabet::Alphabet;
use base64::engine::GeneralPurpose;
use base64::Engine;
use brotli::BrotliDecompress;
use chrono::{DateTime, TimeDelta, Utc};
use dotenv::dotenv;
use handlebars::{Context, Handlebars};
use poem::http::StatusCode;
use poem::middleware::{AddData, CookieJarManager, Cors, CorsEndpoint};
use poem::web::cookie::{CookieJar, SameSite};
use poem::web::{cookie, Data, Redirect};
use poem::Response;
use poem::{get, handler, listener::TcpListener, web::Path, Body, EndpointExt, FromRequest, IntoResponse, Route, Server};
use rand::{Rng, RngCore, SeedableRng};
use regex::Regex;
use s3::{Bucket, Region};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;
use svg::node::element::SVG;
use svg::node::Value;
use svg::parser::Event;

#[derive(Default, Clone)]
pub struct ServerState {
    open_sessions: Arc<RwLock<HashMap<String, (User, DateTime<Utc>)>>>,
}

macro_rules! define_static_files {
    {
        $(
            $name:ident ($mime:literal) => $path:literal
        ),* $(,)?
    } => {
        $(
            #[handler]
            async fn $name() -> Response {
                Response::builder().body(include_str!($path)).set_content_type($mime)
            }
        )*
    };
}

pub fn create_avatar() -> svg::Document {
    let mut random = rand::rng();

    let hue = random.random_range(0..360);
    let sat = random.random_range(75..100);
    let lit = random.random_range(40..50);

    let wrapping_add = |x: i32, y: i32, thresh: i32| {
        if x + y > thresh {
            x + y - thresh
        } else {
            x + y
        }
    };

    let gradient = format!("linear-gradient(135deg, hsl({hue}deg, {sat}%, {lit}%), hsl({}deg, {sat}%, {lit}%))", wrapping_add(hue, 50, 360));

    let fe_turbelence = svg::node::element::FilterEffectTurbulence::new()
        .set("type", "fractalNoise")
        .set("baseFrequency", "5")
        .set("numOctaves", "3")
        .set("stitchTiles", "noStitch");

    let filter = svg::node::element::Filter::new()
        .set("id", "noise")
        .add(fe_turbelence);

    let circ_radius = random.random_range(25..40);

    let svg = svg::Document::new()
        .set("width", "100px")
        .set("height", "100px")
        .set("style", format!("background: {gradient}"))
        .add(filter)
        .add(
            svg::node::element::Rectangle::new()
                .set("filter", "url(#noise)")
                .set("opacity", "90%")
                .set("height", "100%")
                .set("width", "100%")
        )
        .add(
            svg::node::element::Circle::new()
                .set("cx", "50%")
                .set("cy", "50%")
                .set("r", format!("{}", circ_radius))
                .set("fill", "#e9eaff")
        );
    
    svg
}

define_static_files! {
    signup_page ("text/html") => "../../client/signup.html",
    login_page ("text/html") => "../../client/login.html",
    index_css ("text/css") => "../../client/index.css",
    account_css ("text/css") => "../../client/account.css",
    login_css ("text/css") => "../../client/login.css",
    login_js ("application/javascript") => "../../client/dist/login.js",
    register_js ("application/javascript") => "../../client/dist/register.js",
}

#[handler]
async fn rng() -> String {
    let url = "https://csprng.xyz/v1/api";

    reqwest::get(url).await.unwrap().text().await.unwrap()[9..=52].to_owned()
}

const USE_NET_RAND: bool = false;
const ACCOUNT_TEMPLATE: &str = include_str!("../../client/account.handlebars");

static USER_AVATAR_BUCKET: LazyLock<Box<Bucket>, fn() -> Box<Bucket>> = LazyLock::new(|| Bucket::new(
    "kolloquy-user-avatars",
    Region::R2 { account_id: env::var("CLOUDFLARE_ACC_ID").unwrap() },
    Credentials::new(
        Some(&*env::var("R2_ACCESS_KEY").unwrap()),
        Some(&*env::var("R2_SECRET_KEY").unwrap()),
        None,
        None,
        None,
    ).unwrap(),
).unwrap().with_path_style());

static UID_REGEX: LazyLock<Regex, fn() -> Regex> = LazyLock::new(|| Regex::from_str(r"[a-z]{2}+\d{2}+[a-z]{3}+").unwrap());
static UID_FILTERING_REGEX: LazyLock<Regex, fn() -> Regex> = LazyLock::new(|| Regex::from_str(r"([a-z])[^a-z]*([a-z])[^a-z]*(\d)[^\d]*(\d)[^\d]*([a-z])[^a-z]*([a-z])[^a-z]*([a-z])[^a-z]*").unwrap());

/// A monolithic regex for matching RFC 5233 email addresses.
static EMAIL_REGEX: LazyLock<Regex, fn() -> Regex> = LazyLock::new(|| Regex::from_str(r#"^(\(\w+\))?(([A-Za-z\d]+!)?\w([A-Za-z\d][-.\w]?)+[-A-Za-z\d]|"([-\] (),.:;<>@\[\w]|\\\\"?)+")(\/[A-Za-z\d]+)?(\+[A-Za-z\d]+)?(%(([A-Za-z\d]+)(\.[A-Za-z\d]+)*|\[((((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))|([A-Fa-f\d]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1?\d)?\d)\.){3}(25[0-5]|(2[0-4]|1?\d)?\d)|([A-Fa-f\d]{1,4}:){7}[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,7}:|([A-Fa-f\d]{1,4}:){1,6}:[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,5}(:[A-Fa-f\d]{1,4}){1,2}|([A-Fa-f\d]{1,4}:){1,4}(:[A-Fa-f\d]{1,4}){1,3}|([A-Fa-f\d]{1,4}:){1,3}(:[A-Fa-f\d]{1,4}){1,4}|([A-Fa-f\d]{1,4}:){1,2}(:[A-Fa-f\d]{1,4}){1,5}|[A-Fa-f\d]{1,4}:((:[A-Fa-f\d]{1,4}){1,6})|:((:[A-Fa-f\d]{1,4}){1,7}|:)|fe80:(:[A-Fa-f\d]{0,4}){0,4}%[A-Za-z\d]+|::(ffff(:0{1,4})?:)?((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2})|([A-Fa-f\d]{1,4}:){1,4}:((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))]))?(\(\w+\))?@(([A-Za-z\d]+)(\.[A-Za-z\d]+)*$|\[((((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))|([A-Fa-f\d]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1?\d)?\d)\.){3}(25[0-5]|(2[0-4]|1?\d)?\d)|([A-Fa-f\d]{1,4}:){7}[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,7}:|([A-Fa-f\d]{1,4}:){1,6}:[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,5}(:[A-Fa-f\d]{1,4}){1,2}|([A-Fa-f\d]{1,4}:){1,4}(:[A-Fa-f\d]{1,4}){1,3}|([A-Fa-f\d]{1,4}:){1,3}(:[A-Fa-f\d]{1,4}){1,4}|([A-Fa-f\d]{1,4}:){1,2}(:[A-Fa-f\d]{1,4}){1,5}|[A-Fa-f\d]{1,4}:((:[A-Fa-f\d]{1,4}){1,6})|:((:[A-Fa-f\d]{1,4}){1,7}|:)|fe80:(:[A-Fa-f\d]{0,4}){0,4}%[A-Za-z\d]+|::(ffff(:0{1,4})?:)?((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2})|([A-Fa-f\d]{1,4}:){1,4}:((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))])$"#).unwrap());
static HANDLE_REGEX: LazyLock<Regex, fn() -> Regex> = LazyLock::new(|| Regex::from_str(r"^@?+[\w!$-.\\\/]{3,15}$").unwrap());
static REDIRECT_REGEX: LazyLock<Regex, fn() -> Regex> = LazyLock::new(|| Regex::from_str(r"^https?+:\/{2}+[a-zA-Z0-9]++(?:\.[a-zA-Z0-9]++)++\/?(?:\/[\w.\-~!$&'()*+,;=:@]++)*+$").unwrap());
static PASSWORD_REGEX: LazyLock<Regex, fn() -> Regex> = LazyLock::new(|| Regex::from_str(r"^[a-zA-Z0-9+\/]{43}+=$").unwrap());

pub async fn random_user_id() -> String {
    // Gen 1 = (?:([a-z0-9])(?:[^a-z0-9]*))(?:([a-z0-9])(?:[^a-z0-9]*))(?:([0-9])(?:[^0-9]*))(?:([0-9])(?:[^0-9]*))(?:([a-z])(?:[^a-z]*))(?:([a-z0-9])(?:[^a-z0-9]*))(?:([a-z])(?:[^a-z]*))
    // Gen 2a = ([a-z0-9])[^a-z0-9]*([a-z0-9])[^a-z0-9]*([0-9])[^0-9]*([0-9])[^0-9]*([a-z])[^a-z]*([a-z0-9])[^a-z0-9]*([a-z])[^a-z]*
    // Gen 2b = ([a-z\d])[^a-z\d]*([a-z\d])[^a-z\d]*(\d)[^\d]*(\d)[^\d]*([a-z])[^a-z]*([a-z\d])[^a-z\d]*([a-z])[^a-z]*
    let haystack = if env::var_os("USE_WEB_CSPRNG").is_some() {
        let url = "https://csprng.xyz/v1/api";

        reqwest::get(url).await.unwrap().text().await.unwrap()[9..=52].to_owned()
    } else {
        let mut std_rng = rand::rngs::StdRng::from_os_rng();
        let mut chacha_rng = rand_chacha::ChaCha20Rng::seed_from_u64(std_rng.next_u64());
        let mut bytes = vec![];
        const BYTE_COUNT: u8 = 41;
        
        for _ in 0..BYTE_COUNT {
            bytes.extend_from_slice(&(chacha_rng.next_u64() ^ chacha_rng.next_u64() & chacha_rng.next_u64()).to_be_bytes());
        }

        GeneralPurpose::new(&Alphabet::new("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890+/").unwrap(), Default::default()).encode(bytes)
    };

    let Some(binding) = UID_FILTERING_REGEX.deref().captures(&haystack) else {
        return Box::pin(random_user_id()).await;
    };

    let mut iter = binding.iter();

    iter.next();

    let needle = iter.map(|m| m.unwrap().as_str()).collect::<String>();

    needle
}

async fn random_session_id() -> String {
    if env::var_os("USE_WEB_CSPRNG").is_some() {
        let url = "https://csprng.xyz/v1/api";

        reqwest::get(url).await.unwrap().text().await.unwrap()[9..=52].to_owned()
    } else {
        let mut std_rng = rand::rngs::StdRng::from_os_rng();
        let mut chacha_rng = rand_chacha::ChaCha20Rng::seed_from_u64(std_rng.next_u64());
        let mut bytes = vec![];
        const BYTE_COUNT: u8 = 64;

        for _ in 0..(BYTE_COUNT / 8) {
            bytes.extend_from_slice(&(chacha_rng.next_u64() ^ chacha_rng.next_u64() & chacha_rng.next_u64()).to_be_bytes());
        }

        GeneralPurpose::new(&Alphabet::new("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890+/").unwrap(), Default::default()).encode(bytes).to_owned()[0..BYTE_COUNT as usize].to_owned()
    }
}

#[handler]
async fn account_page(jar: &CookieJar, state: Data<&Arc<ServerState>>) -> Response {
    let Some(mut sid) = jar.get("SSID").map(|cookie| (&cookie.to_string()[5..]).to_string()) else {
        return Redirect::temporary("/login").into_response();
    };

    sid = sid.replace("%22", "");
    sid = sid.replace("%2F", "/");

    let state_read = state.open_sessions.read().await;

    let Some((user, session_started)) = state_read.get(&sid) else {
        return Redirect::temporary("/login").into_response();
    };

    if Utc::now().naive_local() - session_started.naive_local() > TimeDelta::minutes(30) {
        state.open_sessions.write().await.remove(&sid);

        return Redirect::temporary("/login").into_response();
    }

    println!("{sid:?}");

    let state_read = state.open_sessions.read().await;

    let (user, session_started) = state_read.get(&sid).unwrap();
    
    let r2 = KolloquyR2::new(*USER_AVATAR_BUCKET.clone());
    let query = UserQuery::GetAvatar(user.clone());
    
    let mut compressed_avatar = Cursor::new(r2.execute(&query).await.unwrap().to_vec());
    let mut avatar = Cursor::new(Vec::new());
    
    BrotliDecompress(&mut compressed_avatar, &mut avatar).unwrap();

    let avatar_string = String::from_utf8(avatar.into_inner()).unwrap();

    let engine = Handlebars::new();
    let context = Context::from(json!({
        "user": {
            "avatar": avatar_string,
            "handle": user.handle,
            "joined": user.joined.naive_local().to_string(),
        }
    }));

    let rendered = engine.render_template_with_context(ACCOUNT_TEMPLATE, &context).unwrap();

    Response::builder()
        .body(rendered)
        .set_content_type("text/html")
        .with_status(StatusCode::OK)
        .into_response()
}

#[handler]
async fn authenticate_user(body: Body, jar: &CookieJar, state: Data<&Arc<ServerState>>) -> Response {
    let body_str = body.into_string().await.unwrap();

    let Ok(body) = serde_json::from_str::<AuthenticateBody>(&*body_str) else {
        let binding = format!(r#"
Expected JSON to match schema:
{{
    "email": "string",
    "password": "string",
    "redirect": "url",
}}

Got JSON:
{}
"#, body_str);

        let details = binding.trim();

        let error_json = json!({
            "success": false,
            "error": {
                "code": 200,
                "message": "Invalid schema for JSON body",
                "details": details,
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    };

    if !REDIRECT_REGEX.deref().is_match(&body.redirect) {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 204,
                "message": "Invalid redirect URL.",
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    // Check the email against the RFC 5233 regex
    if !EMAIL_REGEX.deref().is_match(&body.email) {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 201,
                "message": "Invalid email address.",
                "details": "Email address did not match the (partial) RFC 5233 regex."
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    if !PASSWORD_REGEX.deref().is_match(&body.password) {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 203,
                "message": "Invalid password hash.",
                "details": "Hash did not match required length and encoding.",
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    // Then, check if the user is already signed in and session is valid (if yes, do nothing)
    if let Some(sid) = jar.get("SSID").map(|cookie| (&cookie.to_string()[5..]).to_string()) {
        let state_read = state.open_sessions.read().await;

        if let Some((_, session_started)) = state_read.get(&sid) {
            if Utc::now().naive_local() - session_started.naive_local() > TimeDelta::minutes(30) {
                state.open_sessions.write().await.remove(&sid);
            } else {
                return Redirect::temporary("/account")
                    .with_status(StatusCode::OK)
                    .into_response();
            }
        };
    }

    let db = KolloquyDB::new();
    let query = UserQuery::GetByEmail(body.email);

    let user = match db.execute(&query).await {
        Ok(user) => user.unwrap(),
        Err(QueryError::NotFound) => {
            let error_json = json!({
                "success": false,
                "error": {
                    "code": 100,
                    "message": "A user with this email does not exist."
                }
            });

            return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
        },
        Err(e) => {
            let error_json = json!({
                "success": false,
                "error": {
                    "code": 300,
                    "message": "Could not access database.",
                    "details": format!("{:?}", e)
                }
            });

            return (StatusCode::INTERNAL_SERVER_ERROR, serde_json::to_string(&error_json).unwrap()).into_response();
        }
    };

    // Compare the password hashes
    if user.password != body.password {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 0,
                "message": "Incorrect password."
            }
        });

        return (StatusCode::FORBIDDEN, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    let sid = random_session_id().await;

    state.open_sessions.write().await.insert(sid.clone(), (user.clone(), Utc::now()));

    println!("{:?}", state.open_sessions.read().await);

    let mut cookie = cookie::Cookie::new("SSID", sid.clone());

    cookie.set_same_site(SameSite::Strict);
    cookie.set_http_only(true);
    cookie.set_max_age(Duration::from_secs(30 * 60));

    jar.add(cookie);

    Redirect::temporary("/account")
        .with_status(StatusCode::OK)
        .into_response()
}

#[handler]
async fn register_user(body: Body, jar: &CookieJar, state: Data<&Arc<ServerState>>) -> Response {
    let body_str = body.into_string().await.unwrap();
    
    let Ok(mut body) = serde_json::from_str::<RegisterBody>(&*body_str) else {
        let binding = format!(r#"
Expected JSON to match schema:
{{
    "email": "string",
    "handle": "string",
    "age": "u8",
    "password": "string"
}}

Got JSON:
{}
"#, body_str);

        let details = binding.trim();

        let error_json = json!({
            "success": false,
            "error": {
                "code": 200,
                "message": "Invalid schema for JSON body",
                "details": details,
            }
        });
        
        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    };
    
    let user_id = random_user_id().await;

    let user = User {
        email: body.email,
        handle: body.handle,
        password: body.password,
        age: body.age as i32,
        country: "NULL".to_string(),
        preferences: "{}".to_string(),
        suspended: false,
        age_verified: false,
        user_id: user_id.clone(),
        phone_number: "".to_string(),
        joined: Utc::now(),
        description: "".to_string(),
        last_agent: "".to_string(),
        last_approx_country: "".to_string(),
        avatar_url: format!("{user_id}.svg.br"),
        email_verified: false,
        last_login: Utc::now(),
        failed_login_attempts: 0,
        locked_until: DateTime::<Utc>::from_timestamp_millis(0).unwrap(),
        timezone: "NULL".to_string(),
    };

    // Check the email against the RFC 5233 regex
    if !EMAIL_REGEX.deref().is_match(&user.email) {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 201,
                "message": "Invalid email address.",
                "details": "Email address did not match the (partial) RFC 5233 regex."
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    // Check the handle
    if !HANDLE_REGEX.deref().is_match(&user.handle) {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 202,
                "message": "Invalid handle.",
                "details": r"Handle did not match the handle regex (/^@?[\w!$-.\\\/]{3,15}$/)"
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    // Check the password (Base64 hash)
    if !PASSWORD_REGEX.deref().is_match(&user.password) {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 203,
                "message": "Invalid password hash.",
                "details": "Hash did not match required length and encoding.",
            }
        });

        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
    }

    // Check if user already exists
    let db = KolloquyDB::new();
    let query = UserQuery::GetByEmail(user.email.clone());

    match db.execute(&query).await {
        Ok(_) => {
            let error_json = json!({
                "success": false,
                "error": {
                    "code": 100,
                    "message": "A user with this email already exists."
                }
            });

            return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
        }
        Err(QueryError::NotFound) => (),
        Err(e) => {
            let error_json = json!({
                "success": false,
                "error": {
                    "code": 300,
                    "message": "Could not access database.",
                    "details": format!("{:?}", e)
                }
            });

            return (StatusCode::INTERNAL_SERVER_ERROR, serde_json::to_string(&error_json).unwrap()).into_response();
        }
    };

    let query = UserQuery::GetByHandle(user.handle.clone());

    match db.execute(&query).await {
        Ok(_) => {
            let error_json = json!({
                "success": false,
                "error": {
                    "code": 101,
                    "message": "A user with this handle already exists."
                }
            });

            return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap()).into_response();
        }
        Err(QueryError::NotFound) => (),
        Err(e) => {
            let error_json = json!({
                "success": false,
                "error": {
                    "code": 300,
                    "message": "Could not access database.",
                    "details": format!("{:?}", e)
                }
            });

            return (StatusCode::INTERNAL_SERVER_ERROR, serde_json::to_string(&error_json).unwrap()).into_response();
        }
    };


    let avatar = create_avatar();

    // Upload the user's avatar
    let bucket = USER_AVATAR_BUCKET.clone();
    let query = UserQuery::UploadAvatar(user.clone(), Clone::clone(&avatar));
    let r2 = KolloquyR2::new(*bucket);

    r2.execute(&query).await.unwrap();

    // Put the user to the database
    let query = UserQuery::PutToDB(user.clone());

    db.execute(&query).await.unwrap();
    
    let sid = random_session_id().await;

    state.open_sessions.write().await.insert(sid.clone(), (user.clone(), Utc::now()));

    let success_json = json!({
        "success": true,
        "id": user.user_id,
        "avatar": avatar.to_string(),
    });

    let mut cookie = cookie::Cookie::new("SSID", sid.clone());

    cookie.set_same_site(SameSite::Strict);
    cookie.set_http_only(true);
    cookie.set_max_age(Duration::from_secs(30 * 60));

    jar.add(cookie);

    Response::builder()
        .body(serde_json::to_string(&success_json).unwrap())
        .set_content_type("application/json")
        .with_status(StatusCode::CREATED)
        .into_response()

}

fn apply_cors(app: Route) -> CorsEndpoint<Route> {
    let mut allowed_origins = vec![];

    // check if we are in a development environment
    if env::var_os("DEV").is_some() {
        // If we are using secure, do NOT allow http:// origins
        let using_secure = env::var_os("DEV_SECURE").is_some();

        // Allow ONLY developer origins, no production origins
        env::var_os("DEV_ORIGINS").unwrap().to_str().unwrap().split(',').map(ToString::to_string).for_each(|origin| {
            if using_secure {
                allowed_origins.push(format!("https://{origin}"))
            } else {
                allowed_origins.push(format!("http://{origin}"))
            }
        });
    } else {
        // If we are using secure, do NOT allow http:// origins
        let using_secure = env::var_os("PROD_SECURE").is_some();

        // Allow ONLY production origins, no developer origins
        env::var_os("PROD_ORIGINS").unwrap().to_str().unwrap().split(',').map(ToString::to_string).for_each(|origin| {
            if using_secure {
                allowed_origins.push(format!("https://{origin}"))
            } else {
                allowed_origins.push(format!("http://{origin}"))
            }
        });
    }
    
    app.with(Cors::new().allow_origins(allowed_origins))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    dotenv().ok();
    
    let user_facing = Route::new()
        .at("/signup", get(signup_page))
        .at("/login", get(login_page))
        .at("/login.css", get(login_css))
        .at("/index.css", get(index_css))
        .at("/account.css", get(account_css))
        .at("/dist/login.js", get(login_js))
        .at("/dist/register.js", get(register_js))
        .at("/account", get(account_page));
    
    let app = apply_cors(Route::new()
        .nest(
            "/",
            user_facing,
        )
        .at("/register", register_user)
        .at("/auth", authenticate_user))
        .with(LoggingMiddleware {
            persistence: LoggingPersistence::LogFileOnly(PathBuf::from("logs/log.txt")),
            format: LoggingFormat::LBL,
        })
        .with(AddData::new(Arc::new(ServerState::default())))
        .with(CookieJarManager::new());
    
    let use_ipv6 = env::var_os("USE_IPV6").is_some();
    
    if env::var_os("DEV").is_some() && !use_ipv6 {
        let addr = format!("{}:8080", env::var("DEV_IPV4").unwrap());
        
        println!("{}", format!("{}:8080", env::var("DEV_IPV4").unwrap()));
        
        Server::new(TcpListener::bind(addr))
            .run(app)
            .await
    } else if env::var_os("DEV").is_some() && use_ipv6 {
        println!("{}", format!("[{}]:8080", env::var("DEV_IPV4").unwrap()));

        Server::new(TcpListener::bind(format!("[{}]:8080", env::var("DEV_IPV6").unwrap())))
            .run(app)
            .await
    } else {
        println!("{}", format!("{}:80", env::var("PROD_IPV4").unwrap()));

        Server::new(TcpListener::bind(format!("{}:80", env::var("PROD_IPV4").unwrap())))
            .run(app)
            .await
    }
}

mod tests {
    use crate::data::KolloquyDB;
    use crate::user::UserQuery;
    use crate::{random_session_id, random_user_id, ACCOUNT_TEMPLATE, EMAIL_REGEX};
    use ammonia::UrlRelative;
    use dotenv::dotenv;
    use handlebars::{Context, Handlebars};
    use serde_json::json;
    use std::ops::Deref;

    #[tokio::test]
    async fn rand_user_ids() {
        let mut prev = vec![];

        for i in 0..10 {
            let id = random_user_id().await;

            if prev.contains(&id) {
                panic!("Duplicates detected.")
            }

            println!("{id}");

            prev.push(id);
        }
    }

    #[tokio::test]
    async fn rand_session_ids() {
        let mut prev = vec![];

        for i in 0..100_000 {
            let id = random_session_id().await;

            if prev.contains(&id) {
                eprintln!("{id}");
                eprintln!("{prev:?}");
                panic!("Duplicates detected.")
            }

            println!("{id}");

            prev.push(id);
        }
    }

    #[test]
    fn email_regex() -> Result<(), ()> {
        let email = "owne@r@ljpprojects.org";

        if EMAIL_REGEX.deref().is_match(&email) {
            Err(())
        } else {
            Ok(())
        }
    }

    #[test]
    fn ammonia() -> Result<(), ()> {
        let engine = Handlebars::new();

        let context = Context::from(json!({
            "user": {
                "avatar": r#"<svg height="200px" viewBox="0 0 200 200" width="200px" style="text-anchor: middle; dominant-baseline: middle; background: linear-gradient(135deg, hsl(335deg, 87%, 46%), hsl(25deg, 87%, 46%))" xmlns="http://www.w3.org/2000/svg"><filter id='noiseFilter'><feTurbulence type='fractalNoise' baseFrequency='5' numOctaves='10' stitchTiles='noStitch'/></filter><rect filter="url(#noiseFilter)" opacity="90%" height="100%" width="100%"/><circle cx="50%" cy="50%" r="25%" fill="\#e9eaff"/></svg>"#,
                "handle": "ljpprojects",
                "joined": "2025-04-22T03:13:34.076305+00:00 (22/04/2025, 13:13:34 pm)"
            }
        }));

        let rendered = engine.render_template_with_context(ACCOUNT_TEMPLATE, &context).unwrap();

        let cleaned = ammonia::Builder::new()
            .link_rel(None)
            .url_relative(UrlRelative::PassThrough)
            .strip_comments(true)
            .generic_attributes(["id"].into())
            .add_tags(["svg", "circle", "filter", "feTurbulence", "section", "main", "rect"])
            .add_tag_attributes("svg", ["style", "height", "viewBox", "width", "xmlns"])
            .add_tag_attributes("feTurbulence", ["type", "baseFrequency", "numOctaves", "stitchTiles", "xmlns"])
            .add_tag_attributes("rect", ["style", "filter", "opacity", "height", "width", "fill"])
            .add_tag_attributes("circle", ["cx", "cy", "r", "fill"])
            .clean(&rendered);

        /*
        <svg
            height="200px"
            viewBox="0 0 200 200"
            width="200px"
            style="text-anchor: middle; dominant-baseline: middle; background: linear-gradient(135deg, hsl(335deg, 87%, 46%), hsl(25deg, 87%, 46%))"
            xmlns="http://www.w3.org/2000/svg"
        >
            <filter id='noiseFilter'>
                <feTurbulence
                        type='fractalNoise'
                        baseFrequency='5'
                        numOctaves='10'
                        stitchTiles='noStitch'/>
            </filter>
            <rect filter="url(#noiseFilter)" opacity="90%" height="100%" width="100%"/><circle cx="50%" cy="50%" r="25%" fill="#e9eaff"/>
        </svg>
        */

        println!("{}", cleaned.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn get_user() {
        let handle = "ljpprojects";

        dotenv().ok();

        let db = KolloquyDB::new();
        let query = UserQuery::GetByHandle(handle.parse().unwrap());

        println!("{:#?}", db.execute(&query).await)
    }
}