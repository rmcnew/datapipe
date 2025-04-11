/// Shared datapipe types

use bytes::Bytes;
use log::error;
use std::io::{Error, ErrorKind};
use url::Url;

/// Reader trait to simplify reading from various input sources
#[allow(async_fn_in_trait)]
pub trait InputReader {
    async fn read(&mut self) -> Result<Bytes, Error>;
}

/// Writer trait to simplify reading from various output sinks
#[allow(async_fn_in_trait)]
pub trait OutputWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;
}

// type definitions
// No types needed for stdin and stdout
// Path and PathBuf for file reader and file writer
// TCP, TLS, and UDP readers and writers use String or str with the tokio::net::ToSocketAddrs 
//    to handle DNS resolution
// HTTP and HTTPS use url::Url

pub fn good_url(maybe_url: &str, prefix: &str) -> Result<url::Url, Error> {
    match maybe_url.starts_with(prefix) {
        true => match Url::parse(maybe_url) {
            Ok(url) => Ok(url),
            Err(error) => {
                let error_message = format!("Error parsing URL '{}': {}", maybe_url, error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::InvalidInput, error_message))
            }
        }
        false => {
            let error_message = format!("URL '{}' must start with '{}'", maybe_url, prefix);
            error!("{}", error_message);
            Err(Error::new(ErrorKind::InvalidInput, error_message))
        }
    }    
}

