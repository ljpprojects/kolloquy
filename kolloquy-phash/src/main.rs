// Not working; need to fix error handling

#![feature(const_ops)]
#![feature(const_trait_impl)]

#[cfg(target_os = "windows")]
compile_error!("WINDOWS!?!?! WINDOWS. FUCKING WINDOWS. YOU THINK I WILL ACCEPT W I N D O W S?????");

pub mod cli;
pub mod hash;
pub mod secret;
pub mod consts;

#[cfg(feature = "logging")]
use {
    axum::http::Request,
    std::time::Duration,
    tokio::time::Instant,
    tower_http::{classify::StatusInRangeFailureClass, trace::TraceLayer},
    tracing::Span,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
};

use crate::{
    cli::Arguments, consts::PASS2_PEPPER_VAR_NAME, hash::compute_stage_2_digest, secret::{get_thyme_from_keyring, put_thyme_to_keyring}
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, Response, StatusCode},
    middleware::{MapRequestLayer, map_request},
    routing::post,
    serve::Serve,
};

use base64::{
    DecodeSliceError, Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};

use clap::Parser;
use core::slice;
use dotenvy::Error as DotenvyError;
use keyring_core::Error as KeyringError;
use kolloquy_consts::{KOLLOQUY_VERSION_STR, PASS1_DIGEST_SIZE, PASS2_SALT_SIZE};
use mimalloc::MiMalloc;
use region::Protection;
use secrecy::{SecretBox, SecretSlice};
use serde::Deserialize;
use stack_string::SmallString;
use zeroize::{Zeroize, Zeroizing};

use std::{
    env,
    io::{self, Error, Read},
    num::IntErrorKind::Zero,
    ops::Mul,
    process::ExitCode,
    sync::Arc,
};

use tokio::{
    net::TcpListener,
    task::{JoinHandle, JoinSet},
    try_join,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub struct ServerState {
    pub thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>,
}

impl Zeroize for ServerState {
    fn zeroize(&mut self) {
        let thyme = self.thyme.as_ptr() as *mut u8;

        // Mutable alias of self.thyme, but this never happened, trust me
        let mut thyme = unsafe { slice::from_raw_parts_mut(thyme, self.thyme.len()) };

        thyme.zeroize();
    }
}

#[derive(Deserialize)]
struct PHashRequest {
    salt2: SmallString<{ PASS2_SALT_SIZE.mul(8).div_ceil(6) }>, // Digest is in the url as unpadded Base64 url-safe encoded bytes
}

impl Zeroize for PHashRequest {
    fn zeroize(&mut self) {
        self.salt2.zeroize();
    }
}

async fn handle_phash_req(
    State(state): State<Arc<ServerState>>,
    Path(b64_digest): Path<SmallString<{ PASS1_DIGEST_SIZE.mul(8).div_ceil(6) }>>,
    headers: HeaderMap,
    Json(body): Json<PHashRequest>,
) -> Response<String> {
    use crate::errcodes::{INVALID_DIGEST, INVALID_DIGEST_SIZE, INVALID_SALT_SIZE, INVALID_SALT, NO_VERSION_SPECIFIED, VERSION_MISMATCH};

    #[cfg(feature = "logging")]
    tracing::info!("PHash request reached backend");

    let Some(version) = headers.get("X-Kolloquy-Server-Version") else {
        #[cfg(feature = "logging")]
        tracing::debug!("Requesting server did not specify the version it is running.");

        let mut err = Response::new(format!(
            "{NO_VERSION_SPECIFIED}: X-Kolloquy-Server-Version header is required"
        ));

        *(err.status_mut()) = StatusCode::BAD_REQUEST;

        err.headers_mut().insert(
            "Content-Type",
            "text/plain".try_into().unwrap()
        );

        err.headers_mut().insert(
            "X-Kolloquy-Server-Version",
            KOLLOQUY_VERSION_STR.try_into().unwrap(),
        );

        return err;
    };

    if version.to_str().unwrap() != KOLLOQUY_VERSION_STR {
        #[cfg(feature = "logging")]
        tracing::debug!(
            "Version of requesting server does not match the version of the PHash server."
        );

        let mut err = Response::new(format!(
            "{VERSION_MISMATCH}: Version {} is not equal to phash server version of {KOLLOQUY_VERSION_STR}",
            version.to_str().unwrap()
        ));

        *(err.status_mut()) = StatusCode::BAD_REQUEST;

        err.headers_mut().insert(
            "Content-Type",
            "text/plain".try_into().unwrap()
        );

        err.headers_mut().insert(
            "X-Kolloquy-Server-Version",
            KOLLOQUY_VERSION_STR.try_into().unwrap(),
        );
        return err;
    };

    let mut digest_1 = [0u8; PASS1_DIGEST_SIZE];
    match URL_SAFE_NO_PAD.decode_slice(b64_digest, &mut digest_1) {
        Ok(n) if n != PASS1_DIGEST_SIZE => {
            let mut err = Response::new(format!(
                "{INVALID_DIGEST_SIZE}: The given digest is not {PASS1_DIGEST_SIZE} bytes long."
            ));
            err.headers_mut()
                .insert("Content-Type", "text/plain".try_into().unwrap());
            err.headers_mut().insert(
                "X-Kolloquy-Server-Version",
                KOLLOQUY_VERSION_STR.try_into().unwrap(),
            );
            *(err.status_mut()) = StatusCode::BAD_REQUEST;
            return err;
        }
        Ok(_) => (),
        Err(e) => match e {
            DecodeSliceError::OutputSliceTooSmall => {
                let mut err = Response::new(format!(
                    "{INVALID_DIGEST_SIZE}: The given digest is not {PASS1_DIGEST_SIZE} bytes long."
                ));

                *(err.status_mut()) = StatusCode::BAD_REQUEST;

                err.headers_mut().insert(
                    "Content-Type",
                    "text/plain".try_into().unwrap()
                );

                err.headers_mut().insert(
                    "X-Kolloquy-Server-Version",
                    KOLLOQUY_VERSION_STR.try_into().unwrap(),
                );

                return err;
            }
            DecodeSliceError::DecodeError(de) => {
                let mut err = Response::new(format!(
                    "{INVALID_DIGEST}: The given digest is not valid base64: {de:?}."
                ));

                *(err.status_mut()) = StatusCode::BAD_REQUEST;

                err.headers_mut().insert(
                    "Content-Type",
                    "text/plain".try_into().unwrap()
                );

                err.headers_mut().insert(
                    "X-Kolloquy-Server-Version",
                    KOLLOQUY_VERSION_STR.try_into().unwrap(),
                );

                return err;
            }
        },
    };

    let mut salt_2 = [0u8; PASS2_SALT_SIZE];
    match URL_SAFE_NO_PAD.decode_slice(body.salt2, &mut salt_2) {
        Ok(n) if n != PASS2_SALT_SIZE => {
            let mut err = Response::new(format!(
                "{INVALID_SALT_SIZE}: The given salt is not {PASS2_SALT_SIZE} bytes long."
            ));

            *(err.status_mut()) = StatusCode::BAD_REQUEST;

            err.headers_mut().insert(
                "Content-Type",
                "text/plain".try_into().unwrap()
            );

            err.headers_mut().insert(
                "X-Kolloquy-Server-Version",
                KOLLOQUY_VERSION_STR.try_into().unwrap(),
            );

            return err;
        }
        Ok(_) => (),
        Err(e) => match e {
            DecodeSliceError::OutputSliceTooSmall => {
                let mut err = Response::new(format!(
                    "{INVALID_SALT_SIZE}: The given salt is not {PASS2_SALT_SIZE} bytes long.",
                ));

                *(err.status_mut()) = StatusCode::BAD_REQUEST;

                err.headers_mut().insert(
                    "Content-Type",
                    "text/plain".try_into().unwrap()
                );

                err.headers_mut().insert(
                    "X-Kolloquy-Server-Version",
                    KOLLOQUY_VERSION_STR.try_into().unwrap(),
                );

                return err;
            }
            DecodeSliceError::DecodeError(de) => {
                let mut err = Response::new(format!(
                    "{INVALID_SALT}: The given digest is not valid base64: {de:?}."
                ));

                *(err.status_mut()) = StatusCode::BAD_REQUEST;

                err.headers_mut().insert(
                    "Content-Type",
                    "text/plain".try_into().unwrap()
                );

                err.headers_mut().insert(
                    "X-Kolloquy-Server-Version",
                    KOLLOQUY_VERSION_STR.try_into().unwrap(),
                );

                return err;
            }
        },
    };

    #[cfg(feature = "logging")]
    let start = {
        tracing::debug!("Computing digest...");
        Instant::now()
    };

    let phc = compute_stage_2_digest(digest_1, salt_2, state);

    #[cfg(feature = "logging")]
    tracing::debug!("Computed digest after {:?}", start.elapsed());

    let mut res = Response::new(phc.to_string());

    res.headers_mut().insert(
        "Content-Type",
        "text/plain".try_into().unwrap()
    );

    res.headers_mut().insert(
        "X-Kolloquy-Server-Version",
        KOLLOQUY_VERSION_STR.try_into().unwrap(),
    );

    res
}

#[cfg(feature = "logging")]
fn router(secret_thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>) -> Router {
    Router::new()
        .route("/{digest}", post(handle_phash_req))
        .with_state(Arc::new(ServerState {
            thyme: secret_thyme,
        }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| {
                    tracing::info_span!(
                        "req",
                        method = %req.method(),
                        uri = %req.uri(),
                    )
                })
                .on_request(|req: &Request<_>, span: &Span| {
                    span.in_scope(|| tracing::info!(path = req.uri().path(), "Received request"))
                })
                .on_response(|res: &Response<_>, latency: Duration, span: &Span| {
                    span.in_scope(|| {
                        tracing::info!(
                            status = %res.status(),
                            latency_ms = %latency.as_millis(),
                            "Response sent"
                        )
                    });
                }),
        )
}

#[cfg(not(feature = "logging"))]
fn router(secret_thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>) -> Router {
    Router::new()
        .route("/", post(handle_phash_req))
        .with_state(Arc::new(ServerState {
            thyme: secret_thyme,
        }))
}

cfg_select! {
    feature = "logging" => {
        /// Checks if all required environment variables are set.
        ///
        /// Also runs dotenvy::dotenv
        fn envcheck() -> bool {
            dotenvy::dotenv();

            if env::var(PASS2_PEPPER_VAR_NAME).is_err() {
                tracing::error!(
                    target: "envcheck",
                    name: "required_env_var_missing",
                    "The {PASS2_PEPPER_VAR_NAME} environment variable is required (try including it in your .env file)"
                );

                false
            } else if env::var("BIND_TO").is_err() {
                tracing::error!(
                    target: "envcheck",
                    name: "required_env_var_missing",
                    "The BIND_TO environment variable is required (try including it in the command invocation, e.g. BIND_TO=127.0.0.1:8080,[::1]:8080 kolloquy-phash ...)."
                );

                false
            } else {
                true
            }
        }
    }
    _ => {
        /// Checks if all required environment variables are set.
        ///
        /// Also runs dotenvy::dotenv
        fn envcheck() -> bool {
            dotenvy::dotenv();

            if env::var(PASS2_PEPPER_VAR_NAME).is_err() {
                eprintln!(
                    "The {PASS2_PEPPER_VAR_NAME} environment variable is required (try including it in your .env file)"
                );

                false
            } else if env::var("BIND_TO").is_err() {
                tracing::eprintln!(
                    "The BIND_TO environment variable is required (try including it in the command invocation, e.g. BIND_TO=127.0.0.1:8080,[::1]:8080 kolloquy-phash ...)."
                );

                false
            } else {
                true
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Option<Box<dyn std::error::Error>>> {
    if !envcheck() {
        // envcheck handles logging, we just need to exit
        return Err(None)
    }

    let args = Arguments::parse();

    #[cfg(feature = "logging")]
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Should we try to load the thyme from the keyring (or should we try to set
    // it if we later find -S is set)
    let use_keyring = env::var("USE_KEYRING").map_or(false, |v| &*v == "1");

    // Is only None if use_keyring is false
    let thyme_id = match args.thyme_id {
        Some(i) => Some(i),
        None if use_keyring => {
            #[cfg(feature = "logging")]
            tracing::error!(name: "required_arg_missing", "The `--thyme-id N` (`-T N`) argument is required if USE_KEYRING is enabled.");

            #[cfg(not(feature = "logging"))]
            eprintln!(
                "The `--thyme-id N` (`-T N`) argument is required if USE_KEYRING is enabled."
            );

            return Err(None);
        }
        _ => None,
    };

    // I can't help but wonder what he though while he did those things...?
    // Like as I was standing in line what was he thinking?
    // As he was moving his desk over in religion what was he thinking?
    // A n d  w h y ?
    //
    // W  h  y  ?
    //
    // Had he just reduced me to an object of his sexual desire..?
    //
    // And what can I do?
    // Even if it cannot happen more,
    // Even if I have been moved classes,
    // Even if nothing has happened to him other than a piece of paper,
    // What else can be done about what has already happened?

    if use_keyring {
        #[cfg(target_os = "macos")]
        keyring_core::set_default_store(
            apple_native_keyring_store::keychain::Store::new().unwrap(),
        );

        #[cfg(target_os = "linux")]
        keyring_core::set_default_store(linux_keyutils_keyring_store::Store::new().unwrap());
    }

    let thyme = if use_keyring && !args.should_set_thyme {
        // Load from keyring
        match get_thyme_from_keyring(thyme_id.unwrap()) {
            Ok(t) => t,
            Err(e) => match e {
                KeyringError::NoEntry => {
                    #[cfg(all(feature = "logging", target_os = "linux"))]
                    tracing::error!(name: "no_thyme_entry", "No entry exists for thyme {} (the entries do not persist across reboots, it has to be set again).", thyme_id.unwrap());

                    #[cfg(all(feature = "logging", not(target_os = "linux")))]
                    tracing::error!(name: "no_thyme_entry", "No entry exists for thyme {}.", thyme_id.unwrap());

                    #[cfg(all(not(feature = "logging"), target_os = "linux"))]
                    eprintln!(
                        "No entry exists for thyme {} (the entries do not persist across reboots, it has to be set again).",
                        thyme_id.unwrap()
                    );

                    #[cfg(all(not(feature = "logging"), not(target_os = "linux")))]
                    eprintln!("No entry exists for thyme {}.", thyme_id.unwrap());

                    return Error(Some(Box::new(e)));
                }
                KeyringError::NoStorageAccess(pe) => {
                    #[cfg(feature = "logging")]
                    tracing::error!(?pe, name: "thyme_unaccessible", "The entry for thyme {} cannot be accessed.", thyme_id.unwrap());

                    #[cfg(not(feature = "logging"))]
                    eprintln!(
                        "The entry for thyme {} cannot be accessed: {pe}",
                        thyme_id.unwrap()
                    );

                    return Error(Some(pe));
                }
                e => {
                    #[cfg(feature = "logging")]
                    tracing::error!(?e, name: "thyme_read_failure", "An error occurred while reading the entry for thyme {}.", thyme_id.unwrap());

                    #[cfg(not(feature = "logging"))]
                    eprintln!(
                        "An error occurred while reading the entry for thyme {}: {e}",
                        thyme_id.unwrap()
                    );

                    return Error(Some(Box::new(e)));
                }
            },
        }
    } else if use_keyring {
        let thyme_id = thyme_id.unwrap();

        eprintln!("This will set thyme {thyme_id}'s value in the keyring.");
        eprintln!(
            "This will invalidate ALL existing password hashes computed using thyme {thyme_id}."
        );

        eprintln!("Are you sure that you want to replace thyme {thyme_id}? [y/N]");

        let mut line = String::new();
        io::stdin().read_line(&mut line);
        line.make_ascii_lowercase();

        match &*line {
            "y\n" | "yes\n" => {
                eprintln!(
                    "The new thyme needs to be read ({PASS2_THYME_SIZE} bytes will beread from stdin, base64-encoded; {} characters). [Y/n]",
                    PASS2_THYME_SIZE.mul(8).div_ceil(6)
                );

                line.clear();
                io::stdin().read_line(&mut line);
                line.make_ascii_lowercase();

                if &*line == "n\n" || &*line == "no\n" {
                    eprintln!("Aborting...");
                    return Err(None);
                }

                eprint!(
                    "Paste secret ({} base64-encoded chars): ",
                    PASS2_THYME_SIZE.mul(8).div_ceil(6)
                );

                let thyme = {
                    let mut b64_thyme = Zeroizing::new([0u8; PASS2_THYME_SIZE.mul(8).div_ceil(6)]);
                    io::stdin().read_exact(&mut *b64_thyme);

                    #[cfg(target_pointer_width = "7")]
                    compile_error!("Why do I feel nothing?");

                    #[cfg(target_pointer_width = "307")]
                    compile_error!("Even the though of H I M doesn't conjure emotion");

                    #[cfg(target_pointer_width = "29387")]
                    compile_error!("Not even anger, or disgust, or hurt; nothing");

                    let mut t = Zeroizing::new([0u8; PASS2_THYME_SIZE]);
                    STANDARD.decode_slice(&b64_thyme, &mut *t);

                    t
                };

                #[cfg(feature = "logging")]
                tracing::info!("Setting thyme in keyring...");

                #[cfg(not(feature = "logging"))]
                eprintln!("Setting thyme in keyring...");

                if let None = put_thyme_to_keyring(&thyme, thyme_id) {
                    #[cfg(feature = "logging")]
                    tracing::error!("Failed to set thyme in keyring.");

                    #[cfg(not(feature = "logging"))]
                    eprintln!("Failed to set thyme in keyring.");

                    return Err(None);
                };

                #[cfg(feature = "logging")]
                tracing::info!("The thyme was stored in the keyring successfully.");

                #[cfg(not(feature = "logging"))]
                eprintln!("The thyme was stored in the keyring successfully.");

                thyme
            }
            _ => {
                eprintln!("Aborting...");
                return Err(None);
            }
        }
    } else {
        eprint!(
            "Paste secret ({} base64-encoded chars): ",
            PASS2_THYME_SIZE.mul(8).div_ceil(6)
        );
        let thyme = {
            let mut b64_thyme = Zeroizing::new([0u8; PASS2_THYME_SIZE.mul(8).div_ceil(6)]);
            io::stdin().read_exact(&mut *b64_thyme);

            let mut t = Zeroizing::new([0u8; PASS2_THYME_SIZE]);
            STANDARD.decode_slice(&b64_thyme, &mut *t);

            t
        };

        /*
         * ------  --------    ---         -----
         *    ---  ---  ---   -------    ------   --------
         * ------  --------  ---   ---  --- ---   --------
         *    ---  ---  ---       ---       ---
         *
         * Who needs ASCII art when you have Katakana art
         *
         * And in vertical:
         *
         * --------
         *     ----
         * --------
         *     ----
         *
         * --------
         * ---  ---
         * --------
         * ---  ---
         *
         *   ---
         *  -------
         * ---   ---
         *      ---
         *
         *    -----
         *  ------
         * --- ---
         *     ---
         *
         * ----
         * ----
         * ----
         * ----
         *
         * (vertical chōonpu mentioned)
         */

        /*
         * 1/2 triplets = 640
         * 1/2 5ths     = 384
         *
         */

        thyme
    };

    // Make sure to mprotect the memory so it doesnt get swapped
    //
    // ...
    //
    // Wait
    // Wouldn't it be on the stack???
    // Can you mlock the stack memory??
    // Well I hope so
    let _guard_1 = match region::lock(&raw const thyme, PASS2_THYME_SIZE) {
        Ok(page) => page,
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!("Could not mprotect memory of the thyme; aborting.");

            #[cfg(not(feature = "logging"))]
            eprintln!("Could not mprotect memory of the thyme; aborting.");

            return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
        }
    };

    // Now we need to box the secret too...
    // And mlock that
    let thyme_heap = Arc::new(thyme);
    let _guard_2 = match region::lock(thyme_heap.as_ptr(), PASS2_THYME_SIZE) {
        Ok(page) => page,
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!("Could not mprotect memory of the thyme; aborting.");

            #[cfg(not(feature = "logging"))]
            eprintln!("Could not mprotect memory of the thyme; aborting.");

            return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
        }
    };

    let Ok(addresses) = env::var("BIND_TO") else {
        #[cfg(feature = "logging")]
        tracing::error!("No addresses could be found to bind to (set BIND_TO?)");

        #[cfg(not(feature = "logging"))]
        eprintln!("No addresses could be found to bind to (set BIND_TO?)");

        return Err(None);
    };

    let mut handles = JoinSet::new();

    for address in addresses.split_terminator(",") {
        let l = match TcpListener::bind(address).await {
            Ok(l) => {
                #[cfg(feature = "logging")]
                tracing::info!("The PHash server will run on http://{address}");

                l
            }
            Err(e) if args.fail_forward_on_bind => {
                #[cfg(feature = "logging")]
                tracing::warn!(
                    name: "Could not bind to address",
                    ?address,
                );

                #[cfg(not(feature = "logging"))]
                eprintln!("Could not bind to address {address}; continuing...");

                continue;
            }
            Err(e) => {
                #[cfg(feature = "logging")]
                tracing::error!(?address,);

                #[cfg(not(feature = "logging"))]
                eprintln!("Could not bind to address {address}; aborting...");

                return Err(Some(Box::new(e)));
            }
        };

        let thyme_heap = thyme_heap.clone();
        handles.spawn(async { axum::serve(l, router(thyme_heap)).await });
    }

    let results = handles.join_all().await;
    let (_, errs): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
    let errs = errs.into_iter().map(Result::unwrap_err).collect::<Vec<_>>();

    if errs.is_empty() {
        Ok(())
    } else {
        Err(Some(Box::new(unsafe {
            (errs.first().unwrap() as *const std::io::Error).read()
        })))
    }
}
