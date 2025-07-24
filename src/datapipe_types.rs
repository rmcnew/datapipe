/// Shared datapipe types
use bytes::Bytes;
use log::error;
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use std::io::{Error, ErrorKind};
use url::Url;

// datapipe error types
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum DatapipeError {
    /// Configuration error
    #[error("ConfigurationError: {0}")]
    ConfigurationError(String),
    /// Input-Output error
    #[error("InputOutputError: {0}")]
    InputOutputError(String),
    /// Encryption error
    #[error("EncryptionError: {0}")]
    EncryptionError(String),
    /// Provided parameter is not valid
    #[error("ValidationError: {0}")]
    ValidationError(String),
}

impl From<chacha20poly1305::Error> for DatapipeError {
    fn from(error: chacha20poly1305::Error) -> Self {
        Self::EncryptionError(format!("{error}"))
    }
}

impl From<std::io::Error> for DatapipeError {
    fn from(error: std::io::Error) -> Self {
        let error_string = error_root_cause(&error);
        Self::InputOutputError(error_string)
    }
}

impl From<std::env::VarError> for DatapipeError {
    fn from(error: std::env::VarError) -> Self {
        Self::ConfigurationError(format!("{error}"))
    }
}

impl From<rustls::Error> for DatapipeError {
    fn from(error: rustls::Error) -> Self {
        Self::ConfigurationError(format!("{error}"))
    }
}

impl From<rustls::client::VerifierBuilderError> for DatapipeError {
    fn from(error: rustls::client::VerifierBuilderError) -> Self {
        Self::ConfigurationError(format!("{error}"))
    }
}

/// get the cause of an error
pub fn error_root_cause(mut err: &(dyn std::error::Error + 'static)) -> String {
    use std::fmt::Write;
    let mut s = format!("{}", err);
    while let Some(src) = err.source() {
        let _ = write!(s, "\n\tCaused by: {}", src);
        err = src;
    }
    s
}

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

/// generate a String of the given length with pseudorandom content
pub fn generate_random_string(len: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

// typedefs for EncryptionKey fields
const KEY_LENGTH: usize = 32;
type KeyBytes = [u8; KEY_LENGTH];
const NONCE_LENGTH: usize = 19;
type NonceBytes = [u8; NONCE_LENGTH];
const REQUIRED_LENGTH: usize = KEY_LENGTH + NONCE_LENGTH;

/// Symmetric encryption key and nonce
#[derive(Clone, Debug)]
pub struct EncryptionKey {
    pub key: KeyBytes,
    pub nonce: NonceBytes,
}

impl EncryptionKey {

    /// create an EncryptionKey using the provided String; must be exactly 51 bytes
    pub fn new(encryption_key: &str) -> Result<Self, DatapipeError> {
        if encryption_key.len() != REQUIRED_LENGTH {
            let error_message = format!(
                "Encryption key must be {} bytes long; provided encryption key is {} bytes long",
                REQUIRED_LENGTH,
                encryption_key.len()
            );
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        let encryption_key_bytes = encryption_key.as_bytes();
        Ok(Self {
            key: <KeyBytes>::try_from(&encryption_key_bytes[0..KEY_LENGTH]).unwrap(),
            nonce: <NonceBytes>::try_from(&encryption_key_bytes[KEY_LENGTH..]).unwrap(),
        })
    }

    pub fn generate() -> Self {
        let encryption_key = generate_random_string(REQUIRED_LENGTH);
        let encryption_key_bytes = encryption_key.into_bytes();
        Self {
            key: <KeyBytes>::try_from(&encryption_key_bytes[0..KEY_LENGTH]).unwrap(),
            nonce: <NonceBytes>::try_from(&encryption_key_bytes[KEY_LENGTH..]).unwrap(),
        }
    }

    pub fn to_string(&self) -> String {
        let mut bytes: [u8; REQUIRED_LENGTH] = [0; REQUIRED_LENGTH]; 
        bytes[0..KEY_LENGTH].copy_from_slice(self.key.as_slice());
        bytes[KEY_LENGTH..REQUIRED_LENGTH].copy_from_slice(self.nonce.as_slice());
        String::from_utf8(bytes.to_vec()).unwrap()
    }
}

pub fn good_url(maybe_url: &str, prefix: &str) -> Result<url::Url, Error> {
    match maybe_url.starts_with(prefix) {
        true => match Url::parse(maybe_url) {
            Ok(url) => Ok(url),
            Err(error) => {
                let error_message = format!("Error parsing URL '{}': {}", maybe_url, error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::InvalidInput, error_message))
            }
        },
        false => {
            let error_message = format!("URL '{}' must start with '{}'", maybe_url, prefix);
            error!("{}", error_message);
            Err(Error::new(ErrorKind::InvalidInput, error_message))
        }
    }
}
