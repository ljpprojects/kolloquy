#![feature(const_ops)]
#![feature(const_trait_impl)]

#[cfg(target_os = "windows")]
compile_error!("The Kolloquy PHash server does not suport windows environments.");

pub mod hash;
pub mod secret;
pub mod cli;

#[cfg(feature = "logging")]
use axum::http::Request;
#[cfg(feature = "logging")]
use tokio::time::Instant;
#[cfg(feature = "logging")]
use tower_http::{classify::StatusInRangeFailureClass, trace::TraceLayer};
#[cfg(feature = "logging")]
use tracing::Span;
#[cfg(feature = "logging")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use core::slice;
#[cfg(feature = "logging")]
use std::time::Duration;
use std::{env, io::{self, Error, Read}, num::IntErrorKind::Zero, ops::Mul, process::ExitCode, sync::Arc};
use axum::{Json, Router, extract::{Path, State}, http::{HeaderMap, Response, StatusCode}, middleware::{MapRequestLayer, map_request}, routing::post, serve::Serve};
use clap::Parser;
use mimalloc::MiMalloc;
use region::Protection;
use secrecy::{SecretBox, SecretSlice};
use serde::Deserialize;
use stack_string::SmallString;
use kolloquy_consts::{KOLLOQUY_VERSION_STR, PASS1_DIGEST_SIZE, PASS2_SALT_SIZE};
use base64::{Engine, engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}};
use tokio::{net::TcpListener, task::{JoinHandle, JoinSet}, try_join};
use zeroize::{Zeroize, Zeroizing};
use crate::{cli::Arguments, hash::compute_stage_2_digest, secret::{get_thyme_from_keyring, put_thyme_to_keyring}};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub const PASS2_PEPPER_SIZE: usize = 30;
pub const PASS2_THYME_SIZE: usize = 384;
pub const PASS2_PEPPER_VAR_NAME: &str = "PASS2_PEPPER";

pub struct ServerState {
    pub thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>,
}

impl Zeroize for ServerState {
    fn zeroize(&mut self) {
        let thyme = self.thyme.as_ptr() as *mut u8;

        // Mutable alias of self.thyme
        let mut thyme = unsafe {
            slice::from_raw_parts_mut(thyme, self.thyme.len())
        };

        thyme.zeroize();
    }
}

#[derive(Deserialize)]
struct PHashRequest {
    salt2: SmallString<{PASS2_SALT_SIZE.mul(8).div_ceil(6)}>, // Digest is in the url as not-padded Base64 url-safe encoded bytes
}

impl Zeroize for PHashRequest {
    fn zeroize(&mut self) {
        self.salt2.zeroize();
    }
}

async fn handle_phash_req(
    State(state): State<Arc<ServerState>>,
    Path(b64_digest): Path<SmallString<{PASS1_DIGEST_SIZE.mul(8).div_ceil(6)}>>,
    headers: HeaderMap,
    Json(body): Json<PHashRequest>,
) -> Response<String> {
    #[cfg(feature = "logging")]
    tracing::info!("PHash request reached backend");

    let Some(version) = headers.get("X-Kolloquy-Server-Version") else {
        #[cfg(feature = "logging")]
        tracing::error!("Requesting server did not specify the version it is running.");

        let mut err = Response::new("X-Kolloquy-Server-Version header is required".to_string());
        err.headers_mut().insert("Content-Type", "text/plain".try_into().unwrap());
        err.headers_mut().insert("X-Kolloquy-Server-Version", KOLLOQUY_VERSION_STR.try_into().unwrap());
        *(err.status_mut()) = StatusCode::BAD_REQUEST;
        return err;
    };

    if version.to_str().unwrap() != KOLLOQUY_VERSION_STR {
        #[cfg(feature = "logging")]
        tracing::error!("Version of requesting server does not match the version of the PHash server.");

        let mut err = Response::new(format!("Version {} is not equal to phash server version of {KOLLOQUY_VERSION_STR}", version.to_str().unwrap()));
        err.headers_mut().insert("Content-Type", "text/plain".try_into().unwrap());
        err.headers_mut().insert("X-Kolloquy-Server-Version", KOLLOQUY_VERSION_STR.try_into().unwrap());
        *(err.status_mut()) = StatusCode::BAD_REQUEST;
        return err;
    };

    let mut digest_1 = [0u8; 32];
    URL_SAFE_NO_PAD.decode_slice(b64_digest, &mut digest_1);

    let mut salt_2 = [0u8; PASS2_SALT_SIZE];
    URL_SAFE_NO_PAD.decode_slice(body.salt2, &mut salt_2);

    #[cfg(feature = "logging")]
    tracing::debug!("Computing digest...");

    #[cfg(feature = "logging")]
    let start = Instant::now();

    let phc = compute_stage_2_digest(digest_1, salt_2, state);

    #[cfg(feature = "logging")]
    tracing::debug!("Computed digest after {:?}", start.elapsed());

    let mut res = Response::new(phc.to_string());
    res.headers_mut().insert("Content-Type", "text/plain".try_into().unwrap());
    res.headers_mut().insert("X-Kolloquy-Server-Version", KOLLOQUY_VERSION_STR.try_into().unwrap());

    // Well shit
    // Kinda
    // Had the little tour of the Crookwell High School
    // And now ditching Trinity is an option
    // I could even wake up an hour later
    // Also I quite literally dropped off the face of the earth for 2 weeks nearly
    // And yet not a single person from school (especially not any peer) has even so much as asked where I have been
    // And they only JUST NOW found the DT assessment thing
    // 3 FUCKING WEEKS LATER
    // The only problem with CHS is that they set up one class of 24 or 26 at the start of the year
    // And throughout the year there has been an influx of students into CHS for year 8
    // And it is midway through the term so they cant split year 8 into multiple classes
    // So they have a temporary solution
    // Of just removing like half the class and putting them into the next classroom
    // And also CHS is much smaller
    // Like 4x smaller
    // Like the whole school is the size of just my year at Trinity

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
                    span.in_scope(|| {
                        tracing::info!(
                            path = req.uri().path(),
                            "Received request"
                        )
                    })
                })
                .on_response(|res: &Response<_>, latency: Duration, span: &Span| {
                    span.in_scope(|| {
                        tracing::info!(
                            status = %res.status(),
                            latency_ms = %latency.as_millis(),
                            "Response sent"
                        )
                    });
                })
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

#[tokio::main]
async fn main() -> Result<(), Option<Box<dyn std::error::Error>>> {
    dotenvy::dotenv().unwrap();

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

    // Where do we load the thyme from?
    let use_keyring = env::var("USE_KEYRING").map_or(false, |v| &*v == "1");

    // Is only None if use_keyring is false
    let thyme_id = match args.thyme_id {
        Some(i) => Some(i),
        None if use_keyring => {
            #[cfg(feature = "logging")]
            tracing::error!("The `--thyme-id N` (`-T N`) argument is required if USE_KEYRING is enabled.");

            #[cfg(not(feature = "logging"))]
            eprintln!("The `--thyme-id N` (`-T N`) argument is required if USE_KEYRING is enabled.");

            return Err(None)
        },
        _ => None
    };

    if use_keyring {
        #[cfg(target_os = "macos")]
        keyring_core::set_default_store(apple_native_keyring_store::keychain::Store::new().unwrap());

        // How to test...?

        #[cfg(target_os = "linux")]
        keyring_core::set_default_store(linux_keyutils_keyring_store::Store::new().unwrap());
    }

    let thyme = if use_keyring && !args.should_set_thyme {
        // Load from keyring
        match get_thyme_from_keyring(thyme_id.unwrap()) {
            Some(t) => t,
            None => {
                #[cfg(feature = "logging")]
                tracing::error!("The thyme could not be read the keyring; try re-running with the --set-thyme (or -S) flag set and pass the thyme into stdin.");

                #[cfg(not(feature = "logging"))]
                eprintln!("Thyme could not be read from keyring; try running again with the thyme passed to stdin and with the --set-thyme flag (or -S) set.");

                return Err(None)
            }
        }
    } else if use_keyring {
        let thyme_id = thyme_id.unwrap();

        eprintln!("This will set thyme {thyme_id}'s value in the keyring.");
        eprintln!("This will invalidate ALL existing password hashes computed using thyme {thyme_id}.");

        eprintln!("Are you sure that you want to replace thyme {thyme_id}? [y/N]");

        let mut line = String::new();
        io::stdin().read_line(&mut line);
        line.make_ascii_lowercase();

        match &*line {
            "y\n" | "yes\n" => {
                eprintln!("The new thyme needs to be read ({PASS2_THYME_SIZE} bytes will beread from stdin, base64-encoded; {} characters). [Y/n]", PASS2_THYME_SIZE.mul(8).div_ceil(6));

                line.clear();
                io::stdin().read_line(&mut line);
                line.make_ascii_lowercase();

                if &*line == "n\n" || &*line == "no\n" {
                    eprintln!("Aborting...");
                    return Err(None)
                }

                eprint!("Paste secret ({} base64-encoded chars): ", PASS2_THYME_SIZE.mul(8).div_ceil(6));

                let thyme = {
                    let mut b64_thyme = Zeroizing::new([0u8; PASS2_THYME_SIZE.mul(8).div_ceil(6)]);
                    io::stdin().read_exact(&mut *b64_thyme);

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

                    return Err(None)
                };

                #[cfg(feature = "logging")]
                tracing::info!("The thyme was stored in the keyring successfully.");

                #[cfg(not(feature = "logging"))]
                eprintln!("The thyme was stored in the keyring successfully.");

                thyme
            },
            _ => {
                eprintln!("Aborting...");
                return Err(None)
            }
        }
    } else {
        eprint!("Paste secret ({} base64-encoded chars): ", PASS2_THYME_SIZE.mul(8).div_ceil(6));
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

            return Err(Some(Box::new(e) as Box<dyn std::error::Error>))
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

            return Err(Some(Box::new(e) as Box<dyn std::error::Error>))
        }
    };

    let Ok(addresses) = env::var("BIND_TO") else {
        #[cfg(feature = "logging")]
        tracing::error!("No addresses could be found to bind to (set BIND_TO?)");

        #[cfg(not(feature = "logging"))]
        eprintln!("No addresses could be found to bind to (set BIND_TO?)");

        return Err(None)
    };

    let mut handles = JoinSet::new();

    for address in addresses.split_terminator(",") {
        let l = match TcpListener::bind(address).await {
            Ok(l) => {
                #[cfg(feature = "logging")]
                tracing::info!("The PHash server will run on http://{address}");

                l
            },
            Err(e) if args.fail_forward_on_bind => {
                #[cfg(feature = "logging")]
                tracing::warn!(
                    name: "Could not bind to address",
                    ?address,
                );

                #[cfg(not(feature = "logging"))]
                eprintln!("Could not bind to address {address}; continuing...");

                continue
            },
            Err(e) => {
                #[cfg(feature = "logging")]
                tracing::error!(
                    ?address,
                );

                #[cfg(not(feature = "logging"))]
                eprintln!("Could not bind to address {address}; aborting...");

                return Err(Some(Box::new(e)));
            }
        };

        let thyme_heap = thyme_heap.clone();
        handles.spawn(async {
            axum::serve(l, router(thyme_heap)).await
        });
    };

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