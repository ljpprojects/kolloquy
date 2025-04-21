use std::io::Write;
use std::alloc::{alloc, dealloc, Layout};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use poem::{Endpoint, Request};
use memmap2::{Mmap, MmapAsRawDesc, MmapRawDescriptor};
use poem::Middleware;
use std::fs::File;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum LoggingPersistence {
    MemoryOnly,
    LogFileOnly(PathBuf),
    LogFileAndMemory(PathBuf),
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Debug, Copy)]
pub enum LoggingFormat {
    // JSON,
    // YAML,
    // Protobuf,
    // Binary,
    LBL,
    CompressedLBL,
    // CompressedJSON,
    // CompressedYAML,
    // CompressedProtobuf,
    // CompressedBinary,
}

pub struct LoggingMiddleware {
    pub persistence: LoggingPersistence,
    pub format: LoggingFormat,
}

pub struct LoggedEndpoint<E: Endpoint> {
    inner: E,
    persistence: LoggingPersistence,
    format: LoggingFormat,
}

impl<E: Endpoint> Endpoint for LoggedEndpoint<E> {
    type Output = E::Output;

    fn call(&self, req: Request) -> impl Future<Output=poem::Result<Self::Output>> + Send {
        match self.persistence {
            LoggingPersistence::MemoryOnly => todo!("In-memory logging"),
            LoggingPersistence::LogFileOnly(ref file) => {
                let log = format!(
                    "[{}] {} {} PATH {} PARAMS {} FROM {} HEADERS {}", Utc::now().to_rfc3339(),
                    req.scheme().as_str().to_ascii_uppercase(),
                    req.method().as_str().to_ascii_uppercase(),
                    req.uri().path(),
                    req.params::<HashMap<String, String>>().unwrap().iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(","),
                    req.remote_addr(),
                    req.headers().iter().map(|(k, v)| format!("{k}={v:?}")).collect::<Vec<_>>().join(","),
                );
                
                if &*String::from_utf8_lossy(fs::read(file).unwrap().as_slice()) == "" {
                    fs::write(file, &[log.as_bytes().to_owned()].concat()).unwrap();
                } else {
                    fs::write(file, &[fs::read(file).unwrap(), vec!['\n' as u8], log.as_bytes().to_owned()].concat()).unwrap();
                }
            }
            LoggingPersistence::LogFileAndMemory(ref _map) => todo!("In-memory logging"),
        }

        println!("LOGGED");

        self.inner.call(req)
    }
}

impl<E: Endpoint> Middleware<E> for LoggingMiddleware {
    type Output = LoggedEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        LoggedEndpoint {
            inner: ep,
            persistence: self.persistence.clone(),
            format: self.format,
        }
    }
}

