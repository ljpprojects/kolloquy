#![feature(iter_collect_into)]
#![feature(format_args_nl)]
#![feature(box_vec_non_null)]
#![feature(string_from_utf8_lossy_owned)]
#![feature(ptr_as_ref_unchecked)]
#![feature(core_float_math)]
#![feature(push_mut)]
#![feature(associated_type_defaults)]
#![feature(generic_const_exprs)]
#![feature(type_alias_impl_trait)]
#![feature(int_roundings)]
#![no_std]

use crate::util::ArrayB64EncodeSmallString;
pub mod api;
pub mod user;
pub mod state;
pub mod email;
pub mod verify;
pub mod crypto;
pub mod session;
pub mod pwned;
pub mod aws;
pub mod reflexive;
pub mod realtime;
pub mod kv;
pub mod d1;
pub mod refresh;
pub mod util;
pub mod webauthn;

extern crate core;
extern crate alloc;

#[global_allocator]
static A: rlsf::SmallGlobalTlsf = rlsf::SmallGlobalTlsf::new();

use crate::session::SESSION_ID_SIZE;
use crate::refresh::RFTK_SIZE;
use core::time::Duration;
use alloc::{boxed::Box, format, string::{String, ToString}, sync::Arc, vec::Vec};
use core::str::FromStr;
use axum_cookie::{CookieLayer, CookieManager, cookie::Cookie, prelude::SameSite};
use base64::{Engine, prelude::BASE64_STANDARD};
use http::{Method, StatusCode, header::{CONTENT_ENCODING, CONTENT_TYPE, LOCATION}};
use wasm_bindgen::prelude::wasm_bindgen;
use worker::{Context, Env, Fetcher, console_error, Date, Response};
use axum::{Extension, response::IntoResponse, routing::{get, post}, extract::Json};
use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use password_hash::phc::PasswordHash;
use tower_http::cors::CorsLayer;
use tower_service::Service;
use serde_json::json;
use stack_string::SmallString;
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
    }, kv::KvInterface, pwned::password_plaintext_pwned, session::Session, state::WorkerState, user::{
        User,
        UserIdentifyingKey
    }, verify::{
        MAX_RETRIES,
        VerificationSession
    }
};
use crate::d1::D1Core;
use crate::kv::{KvCore, KvInterfaceOwned};
use crate::refresh::RefreshToken;
use crate::user::IDENT_SIZE;

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
    let password_hash_phc =
        match compute_password_hash(payload.password, salt, state.env.clone()).await {
            Ok(phc) => phc,
            Err(e) => return GenericAPIResponse(
                StatusCode::BAD_GATEWAY,
                &[("Content-Type", "application/json")],
                AuthRegisterResponse {
                    status: "error".to_string(),
                    error: Some(AuthRegisterError {
                        code: AuthRegisterErrorCode::Other,
                        message: format!("The password failed to be hashed, with error: {e}")
                    }),
                }
            )
        };

    // Prepare email
    let email = VerificationEmail::new(payload.email.clone(), payload.display_name.clone());

    // Create verification session
    let session = VerificationSession::new(
        payload.email,
        payload.display_name,
        password_hash_phc,
        *email.code()
    );

    let vssid_cookie = Cookie::new("vssid", session.id.clone())
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

        let mut ssid_b64 = SmallString::<24>::new();
        BASE64_STANDARD.encode_slice(
            session.session_id(),
            unsafe { ssid_b64.as_bytes_mut() }, // Aliasing
        );

        // Set the cookie
        let session_cookie = Cookie::new("ssid", ssid_b64)
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

        let mut user_id = [0u8; IDENT_SIZE];
        BASE64_STANDARD.decode_slice(
            ($user).id,
            &mut user_id,
        ).unwrap();

        // Create a refresh token
        let refresh_token = RefreshToken::new(user_id);
        if let Err(e) = refresh_token.put_to_remote(($state).env.clone()).await {
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
        }

        let refresh_token_cookie = Cookie::new("rftk", refresh_token.get_entropy_b64())
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

    // Ensure that a verification session exists for the user
    let mut session = match VerificationSession::owned_fetch_from_remote(SmallString::from_str(vssid.value()).unwrap(), state.env.clone()).await {
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
                    message: "The rate limit (once every 2 minutes) has been exceeded.".to_string()
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

    // Ensure that a verification session exists for the user
    let mut session = match VerificationSession::owned_fetch_from_remote(SmallString::from_str(vssid.value()).unwrap(), state.env.clone()).await {
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

    // I don't think this allocates??????? It shouldn't, right???
    let now = DateTime::<Utc>::from_timestamp_millis(Date::now().as_millis() as i64).unwrap();

    // Create the user
    let user = User {
        id: User::random_id(),
        email: session.user_email.clone(),
        display_name: session.display_name.clone(),
        password_hash_phc: session.password_hash_phc.clone(),
        participations: Vec::default(),
        pending_entrances: Vec::default(),
        creation_time: now,
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

    // Hash in DB
    let hash_a = PasswordHash::new(&user.password_hash_phc).unwrap();

    let mut salt = [0u8; 32];
    BASE64_STANDARD.decode_slice(
        hash_a.salt.unwrap(),
        &mut salt
    ).unwrap();

    // Hash of the password we received
    let hash_b =
        match compute_password_hash(payload.password, salt, state.env.clone()).await {
            Ok(hash) => hash,
            Err(e) => return GenericAPIResponse(
                StatusCode::BAD_GATEWAY,
                &[("Content-Type", "application/json")],
                AuthLoginResponse {
                    status: "error".to_string(),
                    error: Some(AuthLoginError {
                        code: AuthLoginErrorCode::NoSuchUser,
                        message: format!("Could not hash password: {e}"),
                    })
                }
            )
        };

    todo!("Verify hashes on offload server");

    let old_session = match cookie.get("ssid") {
        Some(ssid_cookie) => {
            let mut ssid = [0u8; SESSION_ID_SIZE];
            BASE64_STANDARD.decode_slice(
                ssid_cookie.value(),
                &mut ssid,
            ).unwrap();

            Session::fetch_from_remote(
                &ssid,
                state.env.clone(),
            ).await.ok().flatten()
        },
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
    let session = get_session_m!(
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

macro_rules! runtime_asset {
    ($assets: expr => [text] $name: literal) => {{
        let mut asset: Response = ($assets).fetch($name, None)
            .await
            .unwrap()
            .try_into()
            .unwrap();

        // ew
        asset.text().await.unwrap()
    }};

    ($assets: expr => [bytes] $name: literal) => {{
        let mut asset: Response = ($assets).fetch($name, None)
            .await
            .unwrap()
            .try_into()
            .unwrap();

        // ew
        asset.bytes().await.unwrap()
    }};

    ($assets: expr => [json<$t: path>] $name: literal) => {{
        let mut asset: Response = ($assets).fetch($name, None)
            .await
            .unwrap()
            .try_into()
            .unwrap();

        // ew
        asset.json::<$t>().await.unwrap()
    }};
}

/// vro wtf is going on why is the router function async (it is async because it loads shit at runtime)
async fn router(env: Arc<Env>, ctx: Arc<Context>) -> axum::Router {
    let assets = env.assets("ASSETS").unwrap();

    // Better allocate this whole template than embed it into the binary
    // macro magic
    let verification_email_template =
        runtime_asset!(assets => [text] "http://internal/verification-email.handlebars");

    // Same here
    let index_template =
        runtime_asset!(assets => [text] "http://internal/index.handlebars");

    let mut hbars = Handlebars::new();
    hbars.register_template_string("email_verification", verification_email_template).unwrap();
    hbars.register_template_string("index", index_template).unwrap();

    let hbars = Arc::new(hbars);

    axum::Router::new()
        .route("/", get(index))
        .route("/testmail", post(test_email))
        .nest("/auth", auth_router(env.clone(), ctx.clone(), hbars.clone()))
        //.nest("/rt", realtime_router(env.clone(), ctx.clone(), hbars.clone()))
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
    // Does NOT allocate
    let ctx = Context::new(ctx);

    // Allocates
    match worker::FromRequest::from_raw(req) {
        Ok(req) =>
            router(Arc::new(env), Arc::new(ctx))
                .await // don't ask
                .call(req) // Error type is Infallible, so unwrap is safe
                .await
                .map(worker::IntoResponse::into_raw)
                .unwrap()
                .map_err(Into::into)
                .unwrap(), // This unwrap isnt "safe" but like proper error handling is ugly here
        Err(err) => {
            let e: Box<dyn core::error::Error> = err.into();
            console_error!("Error converting request: {}", &e);

            worker::Response::error(e.to_string(), 500).unwrap().into()
        }
    }
}