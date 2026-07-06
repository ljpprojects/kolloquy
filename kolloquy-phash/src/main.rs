// Working?

#![feature(const_ops)]
#![feature(const_trait_impl)]

#[cfg(target_os = "windows")]
compile_error!("WINDOWS!?!?! WINDOWS. FUCKING WINDOWS. YOU THINK I WILL ACCEPT W I N D O W S?????");

pub mod api;
pub mod cli;
pub mod consts;
pub mod error;
pub mod hash;
pub mod secret;

#[cfg(feature = "tracing")]
use {
    axum::http::Request,
    std::time::Duration,
    tokio::time::Instant,
    tower_http::trace::TraceLayer,
    tracing::Span,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
};

use crate::consts::PASS2_PEPPER_SIZE;
use crate::{
    cli::Arguments, consts::{PASS2_PEPPER_VAR_NAME, PASS2_THYME_SIZE}, hash::compute_stage_2_digest, secret::{get_thyme_from_keyring, put_thyme_to_keyring}
};

#[cfg(feature = "tracing")]
use axum::routing::get;
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, Response, StatusCode},
    routing::post,
};

use base64::{
    DecodeSliceError, Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};

use clap::Parser;
use core::slice;
use keyring_core::Error as KeyringError;
use kolloquy_consts::{KOLLOQUY_VERSION_STR, PASS1_DIGEST_SIZE, PASS2_SALT_SIZE};
use mimalloc::MiMalloc;
use stack_string::SmallString;
use zeroize::{Zeroize, Zeroizing};

use std::{
    env, fs::File, io, ops::Mul, os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc,
};

use tokio::{
    net::TcpListener,
    task::JoinSet,
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
        let thyme = unsafe { slice::from_raw_parts_mut(thyme, self.thyme.len()) };
        thyme.zeroize();
    }
}

// Am I crazy or the less sleep I get the more productive I am? Like I am running on 3 hours of sleep (4 AM - 7 AM) and have been more productive than I have for the past like two weeks
// I feel more productive (and I don't even think I made a single commit with changes over the past fortnight until today either, and that was on 8 hours of sleep daily), but whether this will come to haunt me is unknown
// Lol I have time I don't have anything to do tomorrow, let's fuck with my health!
// Actually though staying up to 4 AM was such a drag
// I feel like caffeine would have helped a LOT
// You aren't meant to be awake for 21 hours straight
// Maybe 5 AM -7 AM tomorrow?
// But 5 is so close to 7 that I may as well just not sleep at all
// Also I don't even feel tired, so either I am not tired or I am so tired that I cannot perceive tiredness
// Dont worry though the bulk of the code was written on sane amounts of sleep (8+), particularly that relating to cryptography
// Also tomorrow is the Sabbath day
// What's the worst that can happen from getting like 5 hours total of sleep for just two days
// I'll review the code eventually
// This is v0.1.0 anyway expect code written on 3 hours of sleep
// Basically, don't trust any commits made today or tomorrow, maybe you can start
// trusting again on Wednesday
// HOLY SHIT MY ARCTIC PURPLE NOKIA 110 (2023) IS DUE BY WEDNESDAY
// Somehow I do manage at least to be in the recommended sleep duration bracket for someone my age (albeit at the literal bare minimum of 8 hours, so go infer my age from that if you want I guess, if you want another hint I cannot even register interest for organ donation, and I live in NSW)
// I was actually doing something from 9 PM to 2AM but then that stopped and so I had to fill 2 hours by reading and watching YouTube (and then the clock struck 4 and I went to sleep much faster than I would normally, think 5 mins vs 30 mins)
async fn handle_phash_req(
    State(state): State<Arc<ServerState>>,
    Path((b64_digest, b64_salt)): Path<(SmallString<{ PASS1_DIGEST_SIZE.mul(8).div_ceil(6) }>, SmallString<{ PASS2_SALT_SIZE.mul(8).div_ceil(6) }>)>,
    headers: HeaderMap,
) -> Response<String> {
    use crate::consts::errcodes::{INVALID_DIGEST, INVALID_DIGEST_SIZE, INVALID_SALT_SIZE, INVALID_SALT, NO_VERSION_SPECIFIED, VERSION_MISMATCH, INVALID_VERSION};

    #[cfg(feature = "tracing")]
    tracing::info!("PHash request reached backend");

    let Some(version) = headers.get("X-Kolloquy-Server-Version") else {
        raise_error!(
            name: "remote_ver_unspecified",
            target: "handle_req",
            "The remote did not specify the version it was running."
        );

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

    let Ok(version) = version.to_str() else {
        raise_error!(
            name: "remote_ver_invalid",
            target: "handle_req",
            "The remote specified the version it was running but it contained invalid characters (non-ASCII characters)."
        );

        let mut err = Response::new(format!(
            "{INVALID_VERSION}: The value in the X-Kolloquy-Server-Version header must contain visible ASCII characters only."
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

    if version != KOLLOQUY_VERSION_STR {
        raise_error!(
            name: "remote_ver_mismatch",
            target: "handle_req",
            "The remote's version ({version}) does not match the version of this server ({KOLLOQUY_VERSION_STR})."
        );

        let mut err = Response::new(format!(
            "{VERSION_MISMATCH}: Version {version} is not equal to phash server version of {KOLLOQUY_VERSION_STR}"
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

            err.headers_mut().insert(
                "Content-Type",
                "text/plain".try_into().unwrap()
            );

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
    match URL_SAFE_NO_PAD.decode_slice(b64_salt, &mut salt_2) {
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

    #[cfg(feature = "tracing")]
    let start = {
        tracing::info!("Computing digest...");
        Instant::now()
    };

    let phc = compute_stage_2_digest(digest_1, salt_2, state);

    #[cfg(feature = "tracing")]
    tracing::info!("Computed digest after {:?}", start.elapsed());

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

#[cfg(feature = "tracing")]
fn router(secret_thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>) -> Router {
    Router::new()
        .route("/{digest}/{salt}", get(handle_phash_req))
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

#[cfg(not(feature = "tracing"))]
fn router(secret_thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>) -> Router {
    Router::new()
        .route("/", post(handle_phash_req))
        .with_state(Arc::new(ServerState {
            thyme: secret_thyme,
        }))
}

/// Checks if all required environment variables are set, if cwd is
/// equal to the directory specified by EXPECT_CWD or /home/klqy
/// (canonicalised).
///
/// Also runs dotenvy::dotenv, ignoring any errors emitted by it.
fn envcheck() -> Result<bool, std::io::Error> {
    let _ = dotenvy::dotenv_override(); // Ignore errors from dotenvy

    let expected_cwd = env::var("EXPECT_CWD").unwrap_or("/home/klqy".to_string());
    let expected_cwd = PathBuf::from(expected_cwd);

    let expected_cwd = match expected_cwd.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            raise_error!(
                name: "invalid_ewd",
                target: "envcheck",
                "The expected working directory ({expected_cwd:?}) is not readable."
            );

            return Err(e);
        }
    };

    let cwd = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            raise_error!(
                name: "invalid_cwd",
                target: "envcheck",
                "The current working directory is not readable."
            );

            return Err(e);
        }
    };

    if expected_cwd != cwd {
        raise_error!(
            name: "invalid_cwd",
            target: "envcheck",
            "The current working directory ({cwd:?}) and the expected working directory ({expected_cwd:?}) do not match."
        );

        // Just exit
        panic!("envcheck failed")
    }

    let Ok(pass2_pepper) = env::var(PASS2_PEPPER_VAR_NAME) else {
        raise_error!(
            name: "required_env_var_missing",
            target: "envcheck",
            "The {PASS2_PEPPER_VAR_NAME} environment variable is required (try including it in your .env file)"
        );

        return Ok(false)
    };

    // Ensure that the pepper is PASS2_PEPPER_SIZE bytes long
    if pass2_pepper.len() != PASS2_PEPPER_SIZE.mul(8).div_ceil(6) {
        raise_error!(
            name: "incorrect_pepper_length",
            target: "envcheck",
            "The pepper ({pass2_pepper}) is not 30 bytes long."
        );

        return Ok(false)
    }

    // Ensure that the pepper is valid base64 by decoding
    let mut _buf = [0u8; PASS2_PEPPER_SIZE];
    if let Err(e) = STANDARD.decode_slice_unchecked(pass2_pepper.clone(), &mut _buf) { // We know the pepper is 30 bytes long, so we know that _buf is large enough
        raise_error!(
            name: "invalid_pepper",
            target: "envcheck",
            "The pepper ({pass2_pepper}) is not valid base64: {e}"
        );

        return Ok(false)
    };

    if env::var("BIND_TO").is_err() {
        raise_error!(
            name: "required_env_var_missing",
            target: "envcheck",
            "The BIND_TO environment variable is required (try including it in the command invocation, e.g. BIND_TO=127.173.197.139:7399,[::1]:8080 kolloquy-phash ...)."
        );

        Ok(false)
    } else {
        Ok(true)
    }
}

/// Check required files and directories are present at the moment the function
/// is run, and if their permissions are correct. Panics if any aren't.
///
/// envcheck should have been run already.
///
/// This can be disabled by setting --no-filecheck
fn filecheck() {
    let required_files = [
        (env::current_exe().unwrap(), 0o4500),

        (PathBuf::from("./phash/mtls.crt"), 0o400),
        (PathBuf::from("./phash/mtls.key"), 0o400),
        (PathBuf::from("./phash/mtls-ca.crt"), 0o400),

        (PathBuf::from("./phash/ssl.crt"), 0o400),
        (PathBuf::from("./phash/ssl.key"), 0o400),

        (PathBuf::from("./cache"), 0o600), // Technically a directory but doesn't change how we check
    ];

    for (path, expected_perms) in required_files {
        if !path.exists() {
            raise_error!(
                name: "required_file_missing",
                target: "filecheck",
                "The {path:?} file must exist to continue running kolloquy-phash."
            );

            panic!("Required file {path:?} is not present.") // TODO: fix an issue where after panicking here, the process will hang
        }

        let file = match File::open(path.clone()) {
            Ok(f) => f,
            Err(e) => {
                raise_error!(
                    name: "required_file_unreadable",
                    target: "filecheck",
                    "The {path:?} file could not be opened: {e}."
                );

                panic!("Required file {path:?} is not readable.")
            }
        };

        let perms = match file.metadata() {
            Ok(m) => m.permissions(),
            Err(e) => {
                raise_error!(
                    name: "required_file_unreadable",
                    target: "filecheck",
                    "The {file:?} file could not be opened: {e}."
                );

                panic!("Required file {file:?} is not readable.")
            }
        };

        let perms: u32 = perms.mode()  & 0o7777;

        if perms != expected_perms {
            raise_error!(
                name: "required_file_unreadable",
                target: "filecheck",
                "The {file:?} file's permissions of {perms:o} do not match the expected permission of {expected_perms:o}."
            );

            panic!("Required file {file:?} is not readable.")
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Option<Box<dyn std::error::Error>>> {
    let args = Arguments::parse();

    #[cfg(feature = "tracing")]
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=info,tower_http=debug,axum::rejection=trace,envcheck=info,get_thyme=info,filecheck=info,permcheck=info", // Apparently the target was being filtered out...
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // So many arguments...
    //
    // kolloquy-phash --help
    // Usage: kolloquy-phash [OPTIONS]
    //
    // Options:
    //   -S, --set-thyme            Overwrite the thyme indicated by --thyme-id. Persists indefinitely on macOS and until reboot on Linux
    //   -T, --thyme-id <THYME_ID>  ID of the thyme to use for this server
    //   -B, --ff-on-bind           Do not exit if an address fails to bind
    //       --no-filecheck         Skip checking for required files and directories, and skip checking their permissions. You should set this if you do not plan to use the default nginx config provided
    //   -h, --help                 Print help

    match envcheck() {
        Ok(res) if !res => return Err(None), // envcheck handled logging
        Ok(_) => (),
        Err(e) => {
            return Err(Some(Box::new(e) as Box<dyn std::error::Error>))
        }
    };

    if !args.skip_filecheck {
        filecheck();
    }

    // Should we try to load the thyme from the keyring (or should we try to set
    // it if we later find -S is set)
    let use_keyring = env::var("USE_KEYRING").map_or(false, |v| &*v == "1");

    // Is only None if use_keyring is false
    let thyme_id = match args.thyme_id {
        Some(i) => Some(i),
        None if use_keyring => {
            raise_error!(
                name: "required_arg_missing",
                "The `--thyme-id N` (`-T N`) argument is required if USE_KEYRING is enabled."
            );

            return Err(None);
        }
        _ => None,
    };

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
                    #[cfg(target_os = "linux")]
                    raise_error!(
                        name: "no_thyme_entry",
                        target: "get_thyme",
                        "No entry exists for thyme {} (the entries do not persist across reboots, it may need to be set again).",
                        thyme_id.unwrap()
                    );

                    #[cfg(not(target_os = "linux"))]
                    raise_error!(
                        name: "no_thyme_entry",
                        target: "get_thyme",
                        "No entry exists for thyme {}.",
                        thyme_id.unwrap()
                    );

                    return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
                }
                KeyringError::NoStorageAccess(pe) => {
                    raise_error!(
                        name: "thyme_unaccessible",
                        target: "get_thyme",
                        "The entry for thyme {} cannot be accessed: {pe}.",
                        thyme_id.unwrap()
                    );

                    return Err(Some(pe as Box<dyn std::error::Error>));
                }
                e => {
                    raise_error!(
                        name: "thyme_read_failure",
                        target: "get_thyme",
                        "An error occurred while reading the entry for thyme {}: {e}.",
                        thyme_id.unwrap()
                    );

                    return Err(Some(Box::new(e)));
                }
            },
        }
    } else if use_keyring {
        let thyme_id = thyme_id.unwrap();

        eprintln!("This will set thyme {thyme_id}'s value in the keyring.");
        eprintln!(
            "This will invalidate ALL existing password hashes computed using thyme {thyme_id}."
        );

        eprint!("Are you sure that you want to replace thyme {thyme_id}? [y/N] ");

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap(); // Not everything can get proper error handling
        line.make_ascii_lowercase();

        match &*line {
            "y\n" | "yes\n" => {
                eprint!(
                    "The new thyme needs to be read ({PASS2_THYME_SIZE} bytes will beread from stdin, base64-encoded; {} characters). [Y/n] ",
                    PASS2_THYME_SIZE.mul(8).div_ceil(6)
                );

                line.clear();
                io::stdin().read_line(&mut line).unwrap();
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
                    let b64_thyme = Zeroizing::new(rpassword::read_password().unwrap());
                    let mut t = Zeroizing::new([0u8; PASS2_THYME_SIZE]);
                    STANDARD.decode_slice(&b64_thyme, &mut *t).unwrap();

                    t
                };

                #[cfg(feature = "tracing")]
                tracing::info!("Setting thyme in keyring...");

                #[cfg(not(feature = "tracing"))]
                eprintln!("Setting thyme in keyring...");

                if let Err(e) = put_thyme_to_keyring(&thyme, thyme_id) {
                    raise_error!(
                        name: "set_thyme_failure",
                        target: "set_thyme",
                        "Failed to set thyme in keyring: {e}."
                    );

                    return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
                };

                #[cfg(feature = "tracing")]
                tracing::info!("The thyme was stored in the keyring successfully.");

                #[cfg(not(feature = "tracing"))]
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
            let b64_thyme = Zeroizing::new(rpassword::read_password().unwrap());
            let mut t = Zeroizing::new([0u8; PASS2_THYME_SIZE]);
            STANDARD.decode_slice(&b64_thyme, &mut *t).unwrap();

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
            raise_error!(
                name: "mlock_failed",
                target: "mlock_thyme",
                "Could not mlock memory of the thyme; aborting."
            );

            return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
        }
    };

    // Now we need to box the secret too...
    // And mlock that
    let thyme_heap = Arc::new(thyme);
    let _guard_2 = match region::lock(thyme_heap.as_ptr(), PASS2_THYME_SIZE) {
        Ok(page) => page,
        Err(e) => {
            raise_error!(
                name: "mlock_failed",
                target: "mlock_thyme",
                "Could not mlock memory of the thyme; aborting."
            );

            return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
        }
    };

    let Ok(addresses) = env::var("BIND_TO") else {
        raise_error!(
            name: "no_bindings",
            target: "addr_bind",
            "No addresses could be found to bind to (set BIND_TO?)"
        );

        return Err(None);
    };

    let mut handles = JoinSet::new();

    for address in addresses.split_terminator(",") {
        let l = match TcpListener::bind(address).await {
            Ok(l) => {
                #[cfg(feature = "tracing")]
                tracing::info!("The PHash server will run on http://{address}.");

                l
            }
            Err(e) if args.fail_forward_on_bind => {
                raise_warning!(
                    name: "addr_bind_failure",
                    target: "addr_bind",
                    "Could not bind to address {address}: {e}"
                );

                continue;
            }
            Err(e) => {
                raise_error!(
                    name: "addr_bind_failure",
                    target: "addr_bind",
                    "Could not bind to address {address}; aborting...",
                );

                return Err(Some(Box::new(e) as Box<dyn std::error::Error>));
            }
        };

        let thyme_heap = thyme_heap.clone();
        handles.spawn(async { axum::serve(l, router(thyme_heap)).await });
    }

    let results = handles.join_all().await;
    let (_, errs): (Vec<_>, Vec<Result<(), io::Error>>) = results.into_iter().partition(Result::is_ok);
    let errs = errs.into_iter().map(Result::unwrap_err).collect::<Vec<_>>();

    if errs.is_empty() {
        Ok(())
    } else {
        Err(Some(Box::new(errs.into_iter().next().unwrap()) as Box<dyn std::error::Error>))
    }
}
