#![feature(iter_collect_into)]
#![feature(format_args_nl)]
#![feature(box_vec_non_null)]
#![feature(string_from_utf8_lossy_owned)]
#![feature(ptr_as_ref_unchecked)]
#![feature(core_float_math)]
#![feature(push_mut)]
#![feature(associated_type_defaults)]
#![no_std]

mod api;
mod user;
mod state;
mod email;
mod verify;
pub(crate) mod crypto;
pub(crate) mod session;
pub(crate) mod pwned;
pub(crate) mod aws;
pub(crate) mod hex;
pub(crate) mod reflexive;
pub(crate) mod realtime;
pub(crate) mod kv;
pub(crate) mod d1;
pub(crate) mod chat;
pub mod refresh;

extern crate core;
extern crate alloc;

use handlebars::Handlebars;
use mini_alloc::MiniAlloc;
use tower_http::cors::CorsLayer;

pub const INDEX_TEMPLATE: &str = include_str!("../frontend/index.handlebars");

#[global_allocator]
static ALLOC:MiniAlloc = MiniAlloc::INIT;

use core::time::Duration;
use alloc::{boxed::Box, format, string::{String, ToString}, sync::Arc, vec::Vec};

use axum_cookie::{CookieLayer, CookieManager, cookie::Cookie, prelude::SameSite};
use base64::{Engine, prelude::BASE64_STANDARD};
use http::{Method, StatusCode, header::{CONTENT_ENCODING, CONTENT_TYPE, LOCATION}};
use wasm_bindgen::prelude::wasm_bindgen;
use worker::{Context, Env, Fetcher, console_error};
use axum::{Extension, response::IntoResponse, routing::{get, post}, extract::Json};
use tower_service::Service;
use serde_json::json;

use crate::{
    api::{
        AuthLoginError,
        AuthLoginErrorCode,
        AuthLoginRequest,
        AuthLoginResponse,
        AuthRegisterError,
        AuthRegisterErrorCode,
        AuthRegisterRequest,
        AuthRegisterResponse,
        AuthVerifyError,
        AuthVerifyErrorCode,
        AuthVerifyRequest,
        AuthVerifyResponse,
        GenericAPIResponse,
        GenericResponse
    }, crypto::{
        compute_password_hash,
        random_bytes_generic
    }, d1::D1Interface, email::{
        VERIFICATION_EMAIL_TEMPLATE,
        VerificationEmail
    }, kv::KvInterface, pwned::password_plaintext_pwned, realtime::api::realtime_router, session::Session, state::WorkerState, user::{
        User,
        UserIdentifyingKey
    }, verify::{
        MAX_RETRIES,
        VerificationSession
    }
};

#[worker::send]
pub async fn test_email(
    Extension(state): Extension<WorkerState>
) -> impl IntoResponse {
    let email = VerificationEmail::new("apol.gingham@gmail.com", "TEST EMAIL USER");

    match email.send(state.env).await {
        Ok(_) => (
            StatusCode::OK,
            [("Content-Type", "text/plain")],
            "Email sent".to_string(),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            [("Content-Type", "text/plain")],
            e.to_string(),
        ),
    }
}

#[worker::send]
pub async fn auth_register(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
    Json(payload): Json<AuthRegisterRequest>,
) -> GenericAPIResponse<AuthRegisterResponse> {
    // Check if the user exists already
   if User::fetch_from_remote(&UserIdentifyingKey::Email(payload.email.clone()), state.env.clone()).await.transpose().is_some() {
       return GenericAPIResponse(
           StatusCode::CONFLICT, // Status code
           &[("Content-Type", "application/json")], // Headers
           AuthRegisterResponse {
               status: "error".to_string(),
               error: Some(AuthRegisterError {
                   code: AuthRegisterErrorCode::UserExists,
                   message: "A user with this email is already registered.".to_string()
               }),
           }
        )
   }

    // Check if the password is pwned
    if password_plaintext_pwned(&payload.password).await.is_ok_and(|v| v) {
        return GenericAPIResponse(
            StatusCode::UNPROCESSABLE_ENTITY, // Status code
            &[("Content-Type", "application/json")], // Headers
            AuthRegisterResponse {
                status: "error".to_string(),
                error: Some(AuthRegisterError {
                    code: AuthRegisterErrorCode::PasswordPwned,
                    message: "The chosen password has been detected in a data breach.".to_string()
                }),
            }
        )
    }

    let salt = random_bytes_generic::<32>();
    let password_hash = compute_password_hash(payload.password, salt, state.env.clone());

    // Prepare email
    let email = VerificationEmail::new(payload.email.clone(), payload.display_name.clone());

    // Create verification session
    let session = VerificationSession::new(
        payload.email,
        payload.display_name,
        password_hash,
        salt,
        *email.code()
    );

    let vssid_cookie = Cookie::new("vssid", BASE64_STANDARD.encode(session.id()))
        .with_secure(true)
        .with_http_only(true)
        .with_max_age(Duration::from_secs(verify::SESSION_MAX_AGE))
        .with_same_site(SameSite::Strict)
        .with_path("/");

    cookie.add(vssid_cookie);

    if let Err(e) = session.put_to_remote(state.env.clone()).await {
        return GenericAPIResponse(
            StatusCode::INTERNAL_SERVER_ERROR,
            &[("Content-Type", "application/json")],
            AuthRegisterResponse {
                status: "error".to_string(),
                error: Some(AuthRegisterError {
                    code: AuthRegisterErrorCode::Other,
                    message: e.to_string()
                }),
            }
        )
    };

    // Send the email
    if let Err(e) = email.send(state.env).await {
        return GenericAPIResponse(
            StatusCode::BAD_GATEWAY,
            &[("Content-Type", "application/json")],
            AuthRegisterResponse {
                status: "error".to_string(),
                error: Some(AuthRegisterError {
                    code: AuthRegisterErrorCode::Other,
                    message: e.to_string()
                }),
            }
        )
    }

    GenericAPIResponse(
        StatusCode::ACCEPTED,
        &[("Content-Type", "application/json")],
        AuthRegisterResponse {
            status: "success".to_string(),
            error: None,
        }
    )
}

macro_rules! setup_user_session {
    (
        for $user:ident
        return $ret:ident,
            err $err:ident,
            code $code:ident;
        with
            cookie: $cookie:expr,
            state: $state:expr,
            !old_session
    ) => {
        let session = Session::new(($user).email, ($user).display_name);

        // Set the cookie
        let session_cookie = Cookie::new("ssid", BASE64_STANDARD.encode(session.session_id()))
            .with_secure(true)
            .with_http_only(true)
            .with_max_age(Duration::from_secs(30 * 60))
            .with_same_site(SameSite::Strict)
            .with_path("/");

        // Set the refresh token cookie
        ($cookie).add(session_cookie);

        // Put the session into KV
        if let Err(e) = session.put_to_remote(($state).env.clone()).await {
            return GenericAPIResponse(
                StatusCode::BAD_GATEWAY,
                &[("Content-Type", "application/json")],
                $ret {
                    status: "error".to_string(),
                    error: Some($err {
                        code: $code::Other,
                        message: e.to_string()
                    }),
                }
            )
        };

        todo!("FIX THE FUCKING REFRESH TOKEN SYSTEM HOLY FUCK SHIT FUCK IS IT BROKEN LIKE HOLY FUCKING GOD");

        // Create a refresh token (112 random bits + 144 bit user id)
        let refresh_token: [u8; 32] =
            [random_bytes_generic::<14>().as_slice(), &*BASE64_STANDARD.decode(($user).id).unwrap()]
                .concat()
                .try_into()
                .unwrap();

        let refresh_token_cookie = Cookie::new("rftk", BASE64_STANDARD.encode(refresh_token))
            .with_secure(true)
            .with_http_only(true)
            .with_max_age(Duration::from_secs(7 * 24 * 60 * 60))
            .with_same_site(SameSite::Strict)
            .with_path("/");

        // Set the refresh token cookie
        ($cookie).add(refresh_token_cookie);
    };

    (
        for $user:ident
        return $ret:ident,
            err $err:ident,
            code $code:ident;
        with
            cookie: $cookie:expr,
            state: $state:expr,
            old_session?: $old:expr
    ) => {
        if let Some(old_session) = ($old) {
            if let Err(e) = old_session.delete_from_remote(($state).env.clone()).await {
                return GenericAPIResponse(
                    StatusCode::BAD_GATEWAY,
                    &[("Content-Type", "application/json")],
                    $ret {
                        status: "error".to_string(),
                        error: Some($err {
                            code: $code::Other,
                            message: e.to_string()
                        }),
                    }
                )
            };
        }

        // Start a user session
        setup_user_session!(
            for $user
            return $ret,
                err $err,
                code $code;
             with
                cookie: ($cookie),
                state: ($state),
                !old_session
        );
    };
}

#[worker::send]
pub async fn auth_email_resend(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
) -> GenericAPIResponse<AuthVerifyResponse> {
    // Make sure the vssid cookie is set
    let Some(vssid) = cookie.get("vssid") else {
        return GenericAPIResponse(
            StatusCode::UNAUTHORIZED,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Unauthenticated,
                    message: "The vssid cookie was not set (did you call /auth/register?).".to_string()
                }),
            }
        )
    };

    let decoded_vssid: [u8; 18] = BASE64_STANDARD.decode(vssid.value()).unwrap().try_into().unwrap();

    // Ensure that a verification session exists for the user
    let mut session = match VerificationSession::fetch_from_remote(&decoded_vssid, state.env.clone()).await {
        Err(e) => {
            return GenericAPIResponse(
                StatusCode::INTERNAL_SERVER_ERROR,
                &[("Content-Type", "text/plain")],
                AuthVerifyResponse {
                    status: "error".to_string(),
                    error: Some(AuthVerifyError {
                        code: AuthVerifyErrorCode::Other,
                        message: e.to_string()
                    }),
                }
            )
        },
        Ok(Some(session)) => session,
        Ok(None) => return GenericAPIResponse(
            StatusCode::UNAUTHORIZED,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Unauthenticated,
                    message: "The vssid cookie did not point to a valid verification session (call /auth/register again).".to_string()
                }),
            }
        )
    };

    // Ensure that the rate limit is not exceeded
    if !session.can_send() {
        return GenericAPIResponse(
            StatusCode::TOO_MANY_REQUESTS,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::RateLimit,
                    message: "The rate limit (once per 2 minutes) has been exceeded.".to_string()
                }),
            }
        )
    }

    // Increase timeout
    session.increase_send_timeout_end_utc();

    // Put the updated session to the KV
    if let Err(e) = session.put_to_remote(state.env.clone()).await {
        return GenericAPIResponse(
            StatusCode::BAD_GATEWAY,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Other,
                    message: e.to_string()
                }),
            }
        )
    };

    // Prepare email
    let code_string = session.code_string();
    let email = VerificationEmail::new(session.user_email, code_string);

    // Send the email
    if let Err(e) = email.send(state.env).await {
        return GenericAPIResponse(
            StatusCode::BAD_GATEWAY,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Other,
                    message: e.to_string()
                }),
            }
        )
    }

    GenericAPIResponse(
        StatusCode::ACCEPTED,
        &[("Content-Type", "application/json")],
        AuthVerifyResponse {
            status: "sent".to_string(),
            error: None,
        }
    )
}

#[worker::send]
pub async fn auth_verify(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
    Json(payload): Json<AuthVerifyRequest>,
) -> GenericAPIResponse<AuthVerifyResponse> {
    // Ensure the sent code is in fact a 6-digit code (as a string)
    if payload.code.len() != 6 || payload.code.chars().any(|c| !c.is_ascii_digit()) {
        return GenericAPIResponse(
            StatusCode::BAD_REQUEST, // Status code
            &[("Content-Type", "application/json")], // Headers
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::MalformedCode,
                    message: "The code the server received was not a 6-digit sequence.".to_string()
                }),
            }
        )
    }

    // Make sure the vssid cookie is set
    let Some(vssid) = cookie.get("vssid") else {
        return GenericAPIResponse(
            StatusCode::UNAUTHORIZED,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Unauthenticated,
                    message: "The vssid cookie was not set (did you call /auth/register?).".to_string()
                }),
            }
        )
    };

    let decoded_vssid: [u8; 18] = BASE64_STANDARD.decode(vssid.value()).unwrap().try_into().unwrap();

    // Ensure that a verification session exists for the user
    let mut session = match VerificationSession::fetch_from_remote(&decoded_vssid, state.env.clone()).await {
        Err(e) => {
            return GenericAPIResponse(
                StatusCode::INTERNAL_SERVER_ERROR,
                &[("Content-Type", "text/plain")],
                AuthVerifyResponse {
                    status: "error".to_string(),
                    error: Some(AuthVerifyError {
                        code: AuthVerifyErrorCode::Other,
                        message: e.to_string()
                    }),
                }
            )
        },
        Ok(Some(session)) => session,
        Ok(None) => return GenericAPIResponse(
            StatusCode::UNAUTHORIZED,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Unauthenticated,
                    message: "The vssid cookie did not point to a valid verification session (call /auth/register again).".to_string()
                }),
            }
        )
    };

    let retries_are_over_limit = session.inc_retries();

    // Ensure codes match (and we have retries remaining)
    if session.code_string() != payload.code && !retries_are_over_limit {
        let message = format!("The code given was incorrect ({} retries remaining).", MAX_RETRIES - session.total_retries);

        // Put the updated session to the KV
        if let Err(e) = session.put_to_remote(state.env).await {
            return GenericAPIResponse(
                StatusCode::BAD_GATEWAY,
                &[("Content-Type", "application/json")],
                AuthVerifyResponse {
                    status: "error".to_string(),
                    error: Some(AuthVerifyError {
                        code: AuthVerifyErrorCode::Other,
                        message: e.to_string()
                    }),
                }
            )
        };

        return GenericAPIResponse(
            StatusCode::UNPROCESSABLE_ENTITY,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::IncorrectCode,
                    message,
                }),
            }
        )
    } else if session.code_string() != payload.code && retries_are_over_limit {
        // Abort the verification (delete session & cookie)
        if let Err(e) = session.delete_from_remote(state.env).await {
            return GenericAPIResponse(
                StatusCode::BAD_GATEWAY,
                &[("Content-Type", "application/json")],
                AuthVerifyResponse {
                    status: "error".to_string(),
                    error: Some(AuthVerifyError {
                        code: AuthVerifyErrorCode::Other,
                        message: e.to_string()
                    }),
                }
            )
        };

        cookie.remove("vssid");

        return GenericAPIResponse(
            StatusCode::UNPROCESSABLE_ENTITY,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::VerificationAborted,
                    message: "The code given was incorrect too many times (verification aborted).".to_string()
                }),
            }
        )
    }

    // Create the user
    let user = User {
        id: User::random_id(),
        email: session.user_email.clone(),
        email_verified: true,
        display_name: session.display_name.clone(),
        password_salt: session.password_salt,
        password_hash: session.password_hash,
        participations: Vec::default(),
        pending_entrances: Vec::default(),
    };

    // Put the user to the db
    if let Err(e) = user.put_to_remote(state.env.clone()).await {
        return GenericAPIResponse(
            StatusCode::INTERNAL_SERVER_ERROR,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Other,
                    message: e.to_string()
                }),
            }
        )
    };

    // End the session
    if let Err(e) = session.delete_from_remote(state.env.clone()).await {
        return GenericAPIResponse(
            StatusCode::BAD_GATEWAY,
            &[("Content-Type", "application/json")],
            AuthVerifyResponse {
                status: "error".to_string(),
                error: Some(AuthVerifyError {
                    code: AuthVerifyErrorCode::Other,
                    message: e.to_string()
                }),
            }
        )
    };

    // Remove the cookie
    cookie.remove("vssid");

    // Start a user session
    setup_user_session!(
        for user
        return AuthVerifyResponse,
            err AuthVerifyError,
            code AuthVerifyErrorCode;
         with
            cookie: cookie,
            state: state,
            !old_session
    );

    GenericAPIResponse(
        StatusCode::CREATED,
        &[("Content-Type", "application/json")],
        AuthVerifyResponse {
            status: "success".to_string(),
            error: None,
        }
    )
}

#[worker::send]
pub async fn auth_login(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
    Json(payload): Json<AuthLoginRequest>,
) -> GenericAPIResponse<AuthLoginResponse> {
    let user = match User::fetch_from_remote(&UserIdentifyingKey::Email(payload.email), state.env.clone()).await {
        Ok(user) => user,
        Err(e) => return GenericAPIResponse(
            StatusCode::INTERNAL_SERVER_ERROR,
            &[("Content-Type", "application/json")],
            AuthLoginResponse {
                status: "error".to_string(),
                error: Some(AuthLoginError {
                    code: AuthLoginErrorCode::Other,
                    message: e.to_string(),
                })
            }
        ),
    };

    let Some(user) = user else {
        return GenericAPIResponse(
            StatusCode::UNAUTHORIZED,
            &[("Content-Type", "application/json")],
            AuthLoginResponse {
                status: "error".to_string(),
                error: Some(AuthLoginError {
                    code: AuthLoginErrorCode::NoSuchUser,
                    message: "No user with these credentials exists.".to_string(),
                })
            }
        )
    };

    // The hash in the DB
    let hash_a = user.password_hash;

    // The hash of the password we received
    let hash_b = compute_password_hash(payload.password, user.password_salt, state.env.clone());

    if hash_a != hash_b {
        return GenericAPIResponse(
            StatusCode::UNAUTHORIZED,
            &[("Content-Type", "application/json")],
            AuthLoginResponse {
                status: "error".to_string(),
                error: Some(AuthLoginError {
                    code: AuthLoginErrorCode::NoSuchUser,
                    message: "No user with these credentials exists.".to_string(),
                })
            }
        )
    };

    let old_session = match cookie.get("ssid") {
        Some(ssid) => Session::fetch_from_remote(
            &BASE64_STANDARD.decode(ssid.value()).unwrap().try_into().unwrap(),
            state.env.clone(),
        ).await.ok().flatten(),
        None => None,
    };

    // Start a user session
    setup_user_session!(
        for user
        return AuthLoginResponse,
            err AuthLoginError,
            code AuthLoginErrorCode;
         with
            cookie: cookie,
            state: state,
            old_session?: old_session
    );

    GenericAPIResponse(
        StatusCode::OK,
        &[("Content-Type", "application/json")],
        AuthLoginResponse {
            status: "success".to_string(),
            error: None,
        }
    )
}

fn auth_router(env: Arc<Env>, ctx: Arc<Context>, hbars: Arc<Handlebars<'static>>) -> axum::Router {
    axum::Router::new()
        .route("/register", post(auth_register))
        .route("/verify", post(auth_verify))
        .route("/resend", post(auth_email_resend))
        .route("/login", post(auth_login))
        .layer(CookieLayer::default())
        .layer(
            CorsLayer::new()
                .allow_methods([Method::POST])
                .allow_origin([
                    "https://kolloquy.com".parse().unwrap(),
                    "https://localhost:8787".parse().unwrap(), // Dev origin, but allowed because it is restricted to the host device
                ])
                .allow_headers([
                    "x-client-version".parse().unwrap() // Used to reject requests if the client version is outdated
                ])
                .expose_headers([
                    CONTENT_ENCODING,
                    CONTENT_TYPE,
                    LOCATION,
                    "x-server-version".parse().unwrap() // Used by the client
                ])
        )
        .layer(Extension(WorkerState {
            env,
            ctx,
            hbars
        }))
}

#[worker::send]
pub async fn index(
    Extension(state): Extension<WorkerState>,
    cookie: CookieManager,
) -> GenericResponse<String> {
    let session = get_session!(
        with
            cookie_jar: cookie,
            state: state
    );

    let data = json!({
        "display_name": session.user_display_name(),
    });

    let content = state.hbars.render("index", &data).unwrap();

    GenericResponse(
        StatusCode::OK,
        &[("Content-Type", "text/html")],
        content,
    )
}

fn router(env: Arc<Env>, ctx: Arc<Context>) -> axum::Router {
    let mut hbars = Handlebars::new();
    hbars.register_template_string("email_verification", VERIFICATION_EMAIL_TEMPLATE).unwrap();
    hbars.register_template_string("index", INDEX_TEMPLATE).unwrap();

    let hbars = Arc::new(hbars);

    env.get_binding::<Fetcher>("MTLS_CLIENT_CERT").unwrap();

    axum::Router::new()
        .route("/", get(index))
        .route("/testmail", post(test_email))
        .nest("/auth", auth_router(env.clone(), ctx.clone(), hbars.clone()))
        .nest("/rt", realtime_router(env.clone(), ctx.clone(), hbars.clone()))
        .layer(CookieLayer::default())
        .layer(Extension(WorkerState {
            env,
            ctx,
            hbars,
        }))
}

// Manually expanded from #[event(fetch)] (see https://github.com/cloudflare/workers-rs/blob/main/worker-macros/src/event.rs)
#[wasm_bindgen]
pub async fn fetch(
    req: web_sys::Request,
    env: Env,
    ctx: worker::worker_sys::Context
) -> web_sys::Response {
    let ctx = worker::Context::new(ctx);

    match worker::FromRequest::from_raw(req) {
        Ok(req) => {
            match router(Arc::new(env), Arc::new(ctx)).call(req).await {
                Ok(raw_res) => {
                    match worker::IntoResponse::into_raw(raw_res) {
                        Ok(res) => res,
                        Err(err) => {
                            let e: Box<dyn core::error::Error> = err.into();
                            console_error!("Error converting response: {}", &e);

                            worker::Response::error(e.to_string(), 500).unwrap().into()
                        }
                    }
                },
                Err(err) => {
                    let e: Box<dyn core::error::Error> = err.into();
                    console_error!("{}", &e);

                    worker::Response::error(e.to_string(), 500).unwrap().into()
                }
            }
        },
        Err(err) => {
            let e: Box<dyn core::error::Error> = err.into();
            console_error!("Error converting request: {}", &e);

            worker::Response::error(e.to_string(), 500).unwrap().into()
        }
    }
}
