use chrono::Utc;
use poem::Middleware;
use poem::{Endpoint, Request};
use std::collections::HashMap;
use async_std::fs;
use async_std::io::Write;
use async_std::path::PathBuf;

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

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        match self.persistence.clone() {
            LoggingPersistence::MemoryOnly => todo!("In-memory logging"),
            LoggingPersistence::LogFileOnly(file) => {
                let log = format!(
                    "[{}] {} {} PATH {} PARAMS {} FROM {} HEADERS {}", Utc::now().to_rfc3339(),
                    req.scheme().as_str().to_ascii_uppercase(),
                    req.method().as_str().to_ascii_uppercase(),
                    req.uri().path(),
                    req.params::<HashMap<String, String>>().unwrap().iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(","),
                    req.remote_addr(),
                    req.headers().iter().map(|(k, v)| format!("{k}={v:?}")).collect::<Vec<_>>().join(","),
                );
                
                async_std::task::spawn(async move {
                    if &*String::from_utf8_lossy(fs::read(file.clone()).await.unwrap().as_slice()) == "" {
                        fs::write(file, &[log.as_bytes().to_owned()].concat()).await.unwrap();
                    } else {
                        fs::write(file.clone(), &[fs::read(file).await.unwrap(), vec!['\n' as u8], log.as_bytes().to_owned()].concat()).await.unwrap();
                    }
                });
            }
            LoggingPersistence::LogFileAndMemory(ref _map) => todo!("In-memory logging"),
        }

        self.inner.call(req).await
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

