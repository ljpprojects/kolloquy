pub(crate) mod user;
pub(crate) mod data;
mod logging;

use poem::Response;
use crate::data::KolloquyDB;
use crate::logging::{LoggingFormat, LoggingMiddleware, LoggingPersistence};
use crate::user::{RegisterBody, User, UserQuery};
use base64::alphabet::Alphabet;
use base64::engine::GeneralPurpose;
use base64::Engine;
use dotenv::dotenv;
use poem::http::StatusCode;
use poem::middleware::{Cors, CorsEndpoint};
use poem::{get, handler, listener::TcpListener, web::Path, Body, EndpointExt, FromRequest, IntoResponse, Route, Server};
use rand::{Rng, RngCore, SeedableRng};
use regex::Regex;
use serde_json::json;
use std::env;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{LazyLock, RwLock};
use chrono::{DateTime, Utc};
use poem::endpoint::StaticFilesEndpoint;

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

pub static ACTIVE_SESSIONS: RwLock<Vec<String>> = RwLock::new(Vec::new());

fn create_avatar() -> svg::Document {
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
    
    let svg = svg::Document::new()
        .set("width", "100px")
        .set("height", "100px")
        .set("style", format!("background: {gradient}"));
    
    svg
}

define_static_files! {
    signup_page ("text/html") => "../../client/signup.html",
    login_page ("text/html") => "../../client/login.html",
    login_css ("text/css") => "../../client/login.css",
    login_js ("application/javascript") => "../../client/dist/login.js",
}

#[handler]
async fn rng() -> String {
    let url = "https://csprng.xyz/v1/api";

    reqwest::get(url).await.unwrap().text().await.unwrap()[9..=52].to_owned()
}

const USE_NET_RAND: bool = false;
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
async fn get_user(Path(id): Path<String>) -> String {
    let db = KolloquyDB::new();

    db.execute(&UserQuery::GetByID(id)).await.unwrap()
}

#[handler]
async fn register_user(body: Body) -> impl IntoResponse {
    let Ok(body) = body.into_json::<RegisterBody>().await else {
        let error_json = json!({
            "success": false,
            "error": {
                "code": 200,
                "message": "Invalid schema for JSON body"
            }
        });
        
        return (StatusCode::BAD_REQUEST, serde_json::to_string(&error_json).unwrap());
    };

    let user = User {
        email: body.email,
        handle: body.handle,
        password: body.password,
        age: body.age as i32,
        country: "NULL".to_string(),
        preferences: "{}".to_string(),
        suspended: false,
        age_verified: false,
        user_id: random_user_id().await,
        phone_number: "".to_string(),
        joined: Default::default(),
        description: "".to_string(),
        last_agent: "".to_string(),
        last_approx_country: "".to_string(),
        avatar_url: "".to_string(),
        email_verified: false,
        last_login: Utc::now(),
        failed_login_attempts: 0,
        locked_until: DateTime::<Utc>::from_timestamp_millis(0).unwrap(),
        timezone: "NULL".to_string(),
    };

    // Check the email against the RFC 5233 regex
    if !EMAIL_REGEX.deref().is_match(&user.email) {
        return (StatusCode::BAD_REQUEST, "Invalid email.".to_string());
    }

    // Check the handle
    if !HANDLE_REGEX.deref().is_match(&user.handle) {
        return (StatusCode::BAD_REQUEST, "Invalid handle.".to_string());
    }

    // Check the password (Base64 hash)
    if !PASSWORD_REGEX.deref().is_match(&user.password) {
        return (StatusCode::BAD_REQUEST, "Invalid handle.".to_string());
    }
    
    let sid = random_session_id().await;

    (StatusCode::CREATED, "test".to_string())
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
        .at("/dist/login.js", get(login_js));
    
    let app = apply_cors(Route::new()
        .nest(
            "/",
            user_facing,
        )
        .at("/user/:id", get(get_user))
        .at("/register", register_user))
        .with(LoggingMiddleware {
            persistence: LoggingPersistence::LogFileOnly(PathBuf::from("logs/log.txt")),
            format: LoggingFormat::LBL,
        });
    
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
    use std::ops::Deref;
    use crate::{random_session_id, random_user_id, EMAIL_REGEX};

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
}