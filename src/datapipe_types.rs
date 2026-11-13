/// Shared datapipe types
use bytes::Bytes;
use log::error;
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
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
    /// Child thread error
    #[error("JoinError: {0}")]
    JoinError(String),
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

impl From<tokio::task::JoinError> for DatapipeError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::JoinError(format!("{error}"))
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

/// Writer trait to simplify writing to various output sinks
#[allow(async_fn_in_trait)]
pub trait OutputWriter {
    /// Writes a slice of bytes to the underlying output destination.
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;

    /// Flushes and gracefully closes the output writer after all stream data has been written.
    async fn close(&mut self) -> Result<(), Error> {
        Ok(())
    }
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
const KEY_DPK1_PREFIX: &str = "dpk1:";
const KEY_HEX_PREFIX: &str = "hex:";
const KEY_DPK1_BASE64_LEN: usize = 68; // 51 bytes = 17 * 3 bytes = 68 base64 chars
const BASE64URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn encode_base64url_51(bytes: &[u8; REQUIRED_LENGTH]) -> String {
    let mut out = String::with_capacity(KEY_DPK1_PREFIX.len() + KEY_DPK1_BASE64_LEN);
    out.push_str(KEY_DPK1_PREFIX);
    for chunk in bytes.as_chunks::<3>().0 {
        let b0 = chunk[0] as usize;
        let b1 = chunk[1] as usize;
        let b2 = chunk[2] as usize;
        out.push(char::from(BASE64URL_ALPHABET[b0 >> 2]));
        out.push(char::from(
            BASE64URL_ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)],
        ));
        out.push(char::from(
            BASE64URL_ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)],
        ));
        out.push(char::from(BASE64URL_ALPHABET[b2 & 0x3f]));
    }
    out
}

fn decode_base64_char(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    }
}

fn decode_base64_51(encoded: &str) -> Result<[u8; REQUIRED_LENGTH], DatapipeError> {
    let encoded_bytes = encoded.as_bytes();
    if encoded_bytes.len() != KEY_DPK1_BASE64_LEN {
        let error_message = format!(
            "Prefixed encryption key payload must be {KEY_DPK1_BASE64_LEN} base64 characters after '{KEY_DPK1_PREFIX}'; got {} characters",
            encoded_bytes.len()
        );
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }

    let mut decoded = [0u8; REQUIRED_LENGTH];
    let mut out_idx = 0;
    for chunk in encoded_bytes.as_chunks::<4>().0 {
        let v0 = decode_base64_char(chunk[0]).ok_or_else(|| {
            DatapipeError::ValidationError("Invalid base64 character in encryption key".to_string())
        })?;
        let v1 = decode_base64_char(chunk[1]).ok_or_else(|| {
            DatapipeError::ValidationError("Invalid base64 character in encryption key".to_string())
        })?;
        let v2 = decode_base64_char(chunk[2]).ok_or_else(|| {
            DatapipeError::ValidationError("Invalid base64 character in encryption key".to_string())
        })?;
        let v3 = decode_base64_char(chunk[3]).ok_or_else(|| {
            DatapipeError::ValidationError("Invalid base64 character in encryption key".to_string())
        })?;
        decoded[out_idx] = (v0 << 2) | (v1 >> 4);
        decoded[out_idx + 1] = (v1 << 4) | (v2 >> 2);
        decoded[out_idx + 2] = (v2 << 6) | v3;
        out_idx += 3;
    }
    Ok(decoded)
}

fn decode_hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn decode_hex_51(encoded: &str) -> Result<[u8; REQUIRED_LENGTH], DatapipeError> {
    let encoded_bytes = encoded.as_bytes();
    let expected_hex_len = REQUIRED_LENGTH * 2;
    if encoded_bytes.len() != expected_hex_len {
        let error_message = format!(
            "Hex encryption key must be {expected_hex_len} hex characters after '{KEY_HEX_PREFIX}'; got {} characters",
            encoded_bytes.len()
        );
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }

    let mut decoded = [0u8; REQUIRED_LENGTH];
    for (idx, pair) in encoded_bytes.as_chunks::<2>().0.iter().enumerate() {
        let hi = decode_hex_nibble(pair[0]).ok_or_else(|| {
            DatapipeError::ValidationError("Invalid hex character in encryption key".to_string())
        })?;
        let lo = decode_hex_nibble(pair[1]).ok_or_else(|| {
            DatapipeError::ValidationError("Invalid hex character in encryption key".to_string())
        })?;
        decoded[idx] = (hi << 4) | lo;
    }
    Ok(decoded)
}

/// Symmetric encryption key (32 bytes) and stream nonce (19 bytes).
///
/// Supports both full-entropy prefixed keys (`dpk1:<68 base64url chars>` or
/// `hex:<102 hex chars>`) and legacy 51-byte UTF-8 keys. Key material is
/// redacted in [`Debug`](std::fmt::Debug) output and zeroized on drop.
///
/// # Examples
///
/// ```
/// use datapipe::datapipe_types::{DatapipeError, EncryptionKey};
///
/// # fn main() -> Result<(), DatapipeError> {
/// let generated = EncryptionKey::generate();
/// let encoded = generated.to_string();
/// assert!(encoded.starts_with("dpk1:"));
/// let parsed = EncryptionKey::new(&encoded)?;
/// assert_eq!(parsed.key, generated.key);
/// assert_eq!(parsed.nonce, generated.nonce);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct EncryptionKey {
    /// 256-bit XChaCha20-Poly1305 encryption key.
    pub key: KeyBytes,
    /// 152-bit base nonce for `EncryptorBE32` / `DecryptorBE32`.
    pub nonce: NonceBytes,
}

impl std::fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionKey")
            .field("key", &"[REDACTED]")
            .field("nonce", &"[REDACTED]")
            .finish()
    }
}

impl Drop for EncryptionKey {
    fn drop(&mut self) {
        self.key.fill(0);
        self.nonce.fill(0);
        std::hint::black_box((&self.key, &self.nonce));
    }
}

impl EncryptionKey {
    fn from_raw_bytes(bytes: &[u8; REQUIRED_LENGTH]) -> Self {
        let mut key = [0u8; KEY_LENGTH];
        let mut nonce = [0u8; NONCE_LENGTH];
        key.copy_from_slice(&bytes[0..KEY_LENGTH]);
        nonce.copy_from_slice(&bytes[KEY_LENGTH..REQUIRED_LENGTH]);
        Self { key, nonce }
    }

    /// Parses an [`EncryptionKey`] from either a full-entropy prefixed key
    /// (`dpk1:<68 base64 chars>` or `hex:<102 hex chars>`) or a legacy
    /// 51-byte UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::ValidationError`] if the key format or length is invalid.
    pub fn new(encryption_key: &str) -> Result<Self, DatapipeError> {
        if let Some(rest) = encryption_key.strip_prefix(KEY_DPK1_PREFIX) {
            let mut raw = decode_base64_51(rest)?;
            let key = Self::from_raw_bytes(&raw);
            raw.fill(0);
            std::hint::black_box(&raw);
            return Ok(key);
        }
        if let Some(rest) = encryption_key.strip_prefix(KEY_HEX_PREFIX) {
            let mut raw = decode_hex_51(rest)?;
            let key = Self::from_raw_bytes(&raw);
            raw.fill(0);
            std::hint::black_box(&raw);
            return Ok(key);
        }
        if encryption_key.len() != REQUIRED_LENGTH {
            let error_message = format!(
                "Encryption key must be a '{KEY_DPK1_PREFIX}' key or exactly {REQUIRED_LENGTH} bytes long; provided encryption key is {} bytes long",
                encryption_key.len()
            );
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        let encryption_key_bytes = encryption_key.as_bytes();
        let mut key = [0u8; KEY_LENGTH];
        let mut nonce = [0u8; NONCE_LENGTH];
        key.copy_from_slice(&encryption_key_bytes[0..KEY_LENGTH]);
        nonce.copy_from_slice(&encryption_key_bytes[KEY_LENGTH..REQUIRED_LENGTH]);
        Ok(Self { key, nonce })
    }

    /// Generates a fresh full-entropy random [`EncryptionKey`] (256-bit key + 152-bit nonce).
    pub fn generate() -> Self {
        let mut random_bytes = [0u8; REQUIRED_LENGTH];
        rng().fill(&mut random_bytes);
        let key = Self::from_raw_bytes(&random_bytes);
        random_bytes.fill(0);
        std::hint::black_box(&random_bytes);
        key
    }
}

impl std::fmt::Display for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut bytes = [0u8; REQUIRED_LENGTH];
        bytes[0..KEY_LENGTH].copy_from_slice(self.key.as_slice());
        bytes[KEY_LENGTH..REQUIRED_LENGTH].copy_from_slice(self.nonce.as_slice());
        let formatted = encode_base64url_51(&bytes);
        bytes.fill(0);
        std::hint::black_box(&bytes);
        f.write_str(&formatted)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key_formats_and_redacted_debug() {
        let generated = EncryptionKey::generate();
        let dpk1 = generated.to_string();
        assert!(dpk1.starts_with(KEY_DPK1_PREFIX));

        let parsed_dpk1 = EncryptionKey::new(&dpk1).expect("dpk1 key should parse");
        assert_eq!(parsed_dpk1.key, generated.key);
        assert_eq!(parsed_dpk1.nonce, generated.nonce);

        let mut raw = Vec::with_capacity(REQUIRED_LENGTH);
        raw.extend_from_slice(generated.key.as_slice());
        raw.extend_from_slice(generated.nonce.as_slice());
        let hex_key = format!(
            "{KEY_HEX_PREFIX}{}",
            raw.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        let parsed_hex = EncryptionKey::new(&hex_key).expect("hex key should parse");
        assert_eq!(parsed_hex.key, generated.key);
        assert_eq!(parsed_hex.nonce, generated.nonce);

        let legacy = "T8BRXrN15Xpz0KE2FjiZEYGmPk4IpHQmweh2DXERhx7vU6OIEJx";
        assert_eq!(legacy.len(), REQUIRED_LENGTH);
        assert!(EncryptionKey::new(legacy).is_ok());

        assert!(EncryptionKey::new("dpk1:short").is_err());
        assert!(EncryptionKey::new("hex:abcd").is_err());
        assert!(EncryptionKey::new("short").is_err());

        let dbg = format!("{generated:?}");
        assert!(dbg.contains("[REDACTED]"));
        assert!(!dbg.contains(&dpk1));
    }
}
