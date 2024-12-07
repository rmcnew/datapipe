/// Shared datapipe types

use bytes::Bytes;
use std::time::Duration;
use std::io::Error;

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

/// How often should the HTTP(S) input receive or HTTP(S) output send?
pub struct Rate(Duration);

