use crate::datapipe_types::{DatapipeError, EncryptionKey};
use aead_stream::{DecryptorBE32, EncryptorBE32};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, aead::Payload};
use log::{error, trace, warn};

/// Legacy v1 frame start magic (4 bytes) without AAD header binding or authenticated EOF.
pub const MESSAGE_START: [u8; 4] = [0x29, 0x16, 0x4B, 0x74];
/// Version 2 frame start magic (4 bytes) with AAD-bound header and authenticated EOF.
pub const MESSAGE_START_V2: [u8; 4] = [0x29, 0x16, 0x4B, 0x75];

const MESSAGE_START_LENGTH: usize = 4;
const FRAME_TYPE_DATA: u8 = 0x01;
const FRAME_TYPE_LAST: u8 = 0x02;
const AEAD_TAG_LENGTH: usize = 16;
const MIN_PREFIX_LENGTH_V1: usize = 5; // MESSAGE_START (4) + min BER MessageLength (1)
const MIN_PREFIX_LENGTH_V2: usize = 6; // MESSAGE_START_V2 (4) + frame_type (1) + min BER MessageLength (1)

/// Maximum allowed ciphertext frame payload size (64 MiB), bounding decoder allocation
/// and preventing integer overflow / OOM DoS attacks from crafted length headers.
pub const MAX_CIPHER_FRAME_LEN: usize = 64 * 1024 * 1024;
const MAX_PLAINTEXT_CHUNK_LEN: usize = MAX_CIPHER_FRAME_LEN - AEAD_TAG_LENGTH;

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Decoded<T: Sized> {
    pub decoded: T,  // what was decoded from the byte stream?
    pub size: usize, // how many bytes was the decoded item?  (how many bytes do we need to move forward in the data stream?)
}

impl<T> Decoded<T> {
    pub fn new(decoded: T, size: usize) -> Self {
        Self { decoded, size }
    }
}

#[test]
fn test_message_length_encode_decode() {
    let message_length = MessageLength::new(1289);
    let encoded = message_length.encode();
    let decoded = MessageLength::decode(&encoded).unwrap().decoded;
    assert_eq!(message_length, decoded);
}

#[derive(Debug, Eq, PartialEq)]
enum DecodeLengthStatus {
    Complete(Decoded<MessageLength>),
    Incomplete,
    Invalid(String),
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct MessageLength {
    pub length_length: usize,  // how many bytes does the length use?
    pub message_length: usize, // how many bytes does the respective value use?
}

impl MessageLength {
    // internal function so we can determine length_length from new
    pub fn encode_internal(message_length: usize) -> Vec<u8> {
        if message_length <= 127 {
            return vec![message_length as u8];
        }
        let be_bytes = message_length.to_be_bytes();
        let first_nonzero = be_bytes
            .iter()
            .position(|&b| b != 0)
            .unwrap_or(be_bytes.len().saturating_sub(1));
        let sig_bytes = &be_bytes[first_nonzero..];
        let mut bytes = Vec::with_capacity(1 + sig_bytes.len());
        bytes.push(0x80u8 | (sig_bytes.len() as u8));
        bytes.extend_from_slice(sig_bytes);
        bytes
    }

    pub fn new(message_length: usize) -> MessageLength {
        MessageLength {
            message_length,
            length_length: Self::encode_internal(message_length).len(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        MessageLength::encode_internal(self.message_length)
    }

    fn decode_checked(bytes: &[u8], max_length: usize) -> DecodeLengthStatus {
        let Some(&b_0) = bytes.first() else {
            return DecodeLengthStatus::Incomplete;
        };
        if b_0 <= 127 {
            let value = b_0 as usize;
            if value > max_length {
                return DecodeLengthStatus::Invalid(format!(
                    "Encrypted frame length {value} exceeds maximum allowed size ({max_length} bytes)"
                ));
            }
            return DecodeLengthStatus::Complete(Decoded::new(
                MessageLength {
                    length_length: 1,
                    message_length: value,
                },
                1,
            ));
        }

        let length_byte_count = (b_0 & 0x7F) as usize;
        if length_byte_count == 0 {
            return DecodeLengthStatus::Invalid(
                "Indefinite-length BER encoding (0x80) is not supported".to_string(),
            );
        }
        if length_byte_count > 8 || length_byte_count > std::mem::size_of::<usize>() {
            return DecodeLengthStatus::Invalid(format!(
                "BER message length byte count {length_byte_count} exceeds maximum supported size"
            ));
        }
        let Some(b_x) = bytes.get(1..=length_byte_count) else {
            return DecodeLengthStatus::Incomplete;
        };

        let mut value: usize = 0;
        for &b in b_x {
            value = (value << 8) | (b as usize);
        }
        if value > max_length {
            return DecodeLengthStatus::Invalid(format!(
                "Encrypted frame length {value} exceeds maximum allowed size ({max_length} bytes)"
            ));
        }
        let length_len = length_byte_count + 1;
        DecodeLengthStatus::Complete(Decoded::new(
            MessageLength {
                length_length: length_len,
                message_length: value,
            },
            length_len,
        ))
    }

    pub fn decode(bytes: &[u8]) -> Option<Decoded<Self>> {
        match Self::decode_checked(bytes, usize::MAX) {
            DecodeLengthStatus::Complete(decoded) => Some(decoded),
            DecodeLengthStatus::Incomplete | DecodeLengthStatus::Invalid(_) => None,
        }
    }
}

#[test]
fn test_length_decode() {
    assert_eq!(
        MessageLength::decode(&[0x04]),
        Some(Decoded::new(
            MessageLength {
                length_length: 1,
                message_length: 4
            },
            1
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x7F]),
        Some(Decoded::new(
            MessageLength {
                length_length: 1,
                message_length: 127
            },
            1
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x81, 0x80]),
        Some(Decoded::new(
            MessageLength {
                length_length: 2,
                message_length: 128
            },
            2
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x81, 0xFF]),
        Some(Decoded::new(
            MessageLength {
                length_length: 2,
                message_length: 255
            },
            2
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x82, 0x48, 0xFF]),
        Some(Decoded::new(
            MessageLength {
                length_length: 3,
                message_length: 18_687
            },
            3
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x82, 0xAB, 0xCD]),
        Some(Decoded::new(
            MessageLength {
                length_length: 3,
                message_length: 43_981
            },
            3
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x83, 0xC0, 0xFF, 0xEE]),
        Some(Decoded::new(
            MessageLength {
                length_length: 4,
                message_length: 12_648_430
            },
            4
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x83, 0xA5, 0xB4, 0x51]),
        Some(Decoded::new(
            MessageLength {
                length_length: 4,
                message_length: 10_859_601
            },
            4
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x84, 0xCA, 0xFE, 0xBA, 0xBE]),
        Some(Decoded::new(
            MessageLength {
                length_length: 5,
                message_length: 3_405_691_582
            },
            5
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x84, 0xDE, 0xAD, 0xBE, 0xEF]),
        Some(Decoded::new(
            MessageLength {
                length_length: 5,
                message_length: 3_735_928_559
            },
            5
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x85, 0xFE, 0xED, 0xDA, 0xDA, 0xD5]),
        Some(Decoded::new(
            MessageLength {
                length_length: 6,
                message_length: 1_094_912_236_245
            },
            6
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x85, 0x0D, 0xED, 0x1C, 0xA7, 0xED]),
        Some(Decoded::new(
            MessageLength {
                length_length: 6,
                message_length: 59_812_653_037
            },
            6
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x86, 0x13, 0x37, 0xC0, 0xDE, 0xD0, 0x0D]),
        Some(Decoded::new(
            MessageLength {
                length_length: 7,
                message_length: 21_130_179_956_749
            },
            7
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x86, 0xFA, 0xCE, 0xB0, 0x0C, 0xDE, 0xAD]),
        Some(Decoded::new(
            MessageLength {
                length_length: 7,
                message_length: 275_765_623_840_429
            },
            7
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x87, 0x60, 0x0D, 0xF0, 0x0D, 0xD0, 0x0D, 0x50]),
        Some(Decoded::new(
            MessageLength {
                length_length: 8,
                message_length: 27_036_922_439_273_808
            },
            8
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x87, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32]),
        Some(Decoded::new(
            MessageLength {
                length_length: 8,
                message_length: 71_737_338_064_426_034
            },
            8
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x88, 0xFE, 0xED, 0xFA, 0xCE, 0xCA, 0xFE, 0xBE, 0xEF]),
        Some(Decoded::new(
            MessageLength {
                length_length: 9,
                message_length: 18_369_614_221_190_020_847
            },
            9
        ))
    );

    assert_eq!(
        MessageLength::decode(&[0x88, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]),
        Some(Decoded::new(
            MessageLength {
                length_length: 9,
                message_length: 1_311_768_467_294_899_695
            },
            9
        ))
    );
}

#[test]
fn test_length_encode() {
    assert_eq!(
        MessageLength {
            length_length: 1,
            message_length: 4
        }
        .encode(),
        vec![0x04]
    );

    assert_eq!(
        MessageLength {
            length_length: 1,
            message_length: 127
        }
        .encode(),
        vec![0x7F]
    );

    assert_eq!(
        MessageLength {
            length_length: 2,
            message_length: 128
        }
        .encode(),
        vec![0x81, 0x80]
    );

    assert_eq!(
        MessageLength {
            length_length: 2,
            message_length: 255
        }
        .encode(),
        vec![0x81, 0xFF]
    );

    assert_eq!(
        MessageLength {
            length_length: 3,
            message_length: 18_687
        }
        .encode(),
        vec![0x82, 0x48, 0xFF]
    );

    assert_eq!(
        MessageLength {
            length_length: 3,
            message_length: 43_981
        }
        .encode(),
        vec![0x82, 0xAB, 0xCD]
    );

    assert_eq!(
        MessageLength {
            length_length: 4,
            message_length: 12_648_430
        }
        .encode(),
        vec![0x83, 0xC0, 0xFF, 0xEE]
    );

    assert_eq!(
        MessageLength {
            length_length: 4,
            message_length: 10_859_601
        }
        .encode(),
        vec![0x83, 0xA5, 0xB4, 0x51]
    );

    assert_eq!(
        MessageLength {
            length_length: 5,
            message_length: 3_405_691_582
        }
        .encode(),
        vec![0x84, 0xCA, 0xFE, 0xBA, 0xBE]
    );

    assert_eq!(
        MessageLength {
            length_length: 5,
            message_length: 3_735_928_559
        }
        .encode(),
        vec![0x84, 0xDE, 0xAD, 0xBE, 0xEF]
    );

    assert_eq!(
        MessageLength {
            length_length: 6,
            message_length: 1_094_912_236_245
        }
        .encode(),
        vec![0x85, 0xFE, 0xED, 0xDA, 0xDA, 0xD5]
    );

    assert_eq!(
        MessageLength {
            length_length: 6,
            message_length: 59_812_653_037
        }
        .encode(),
        vec![0x85, 0x0D, 0xED, 0x1C, 0xA7, 0xED]
    );

    assert_eq!(
        MessageLength {
            length_length: 7,
            message_length: 21_130_179_956_749
        }
        .encode(),
        vec![0x86, 0x13, 0x37, 0xC0, 0xDE, 0xD0, 0x0D]
    );

    assert_eq!(
        MessageLength {
            length_length: 7,
            message_length: 275_765_623_840_429
        }
        .encode(),
        vec![0x86, 0xFA, 0xCE, 0xB0, 0x0C, 0xDE, 0xAD]
    );

    assert_eq!(
        MessageLength {
            length_length: 8,
            message_length: 27_036_922_439_273_808
        }
        .encode(),
        vec![0x87, 0x60, 0x0D, 0xF0, 0x0D, 0xD0, 0x0D, 0x50]
    );

    assert_eq!(
        MessageLength {
            length_length: 8,
            message_length: 71_737_338_064_426_034
        }
        .encode(),
        vec![0x87, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32]
    );

    assert_eq!(
        MessageLength {
            length_length: 9,
            message_length: 18_369_614_221_190_020_847
        }
        .encode(),
        vec![0x88, 0xFE, 0xED, 0xFA, 0xCE, 0xCA, 0xFE, 0xBE, 0xEF]
    );

    assert_eq!(
        MessageLength {
            length_length: 9,
            message_length: 1_311_768_467_294_899_695
        }
        .encode(),
        vec![0x88, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]
    );
}

#[test]
fn test_length_encode_decode_roundrobin() {
    use std::time::{SystemTime, UNIX_EPOCH};
    const MAX_ITERATIONS: usize = 20_000;
    for _ in 0..MAX_ITERATIONS {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let x = (now.as_secs() as usize).swap_bytes()
            | ((now.subsec_nanos() as usize).swap_bytes()
                ^ (now.subsec_micros() as usize).reverse_bits());
        //println!("Testing length encode / decode for: {}", x);
        let encoded = (MessageLength {
            length_length: 1,
            message_length: x,
        })
        .encode();
        let decoded = MessageLength::decode(&encoded).unwrap().decoded;
        assert_eq!(x, decoded.message_length);
    }
}

#[test]
fn test_encrypt_decrypt() {
    let key = EncryptionKey::generate();
    let mut encryptor = StreamEncryptor::new(key.clone()).unwrap();
    let mut decryptor = StreamDecryptor::new(key).unwrap();

    let plain = "There once was a ship that put to sea; The name of the ship was the Billy of Tea; The winds blew up, her bow dipped down; blow, me bully boys, blow.";
    let mut plain_bytes = plain.as_bytes().to_vec();
    let mut cipher = encryptor.encrypt(&mut plain_bytes).unwrap();
    cipher.extend_from_slice(&encryptor.finish().unwrap());
    let plain2 = String::from_utf8(decryptor.decrypt(&mut cipher).unwrap()).unwrap();
    decryptor.finish(&cipher).unwrap();
    assert_eq!(plain, plain2);
}

#[test]
fn test_decrypt_rejects_overflow_and_oversized_frame_lengths() {
    let key = EncryptionKey::generate();
    let mut decryptor = StreamDecryptor::new(key.clone()).unwrap();

    // Crafted v2 header claiming usize::MAX length (0x88 + 8 * 0xFF)
    let mut crafted_v2 = MESSAGE_START_V2.to_vec();
    crafted_v2.push(FRAME_TYPE_DATA);
    crafted_v2.push(0x88);
    crafted_v2.extend_from_slice(&[0xFF; 8]);
    assert!(decryptor.decrypt(&mut crafted_v2).is_err());

    // Crafted v2 header exceeding MAX_CIPHER_FRAME_LEN (0x84 + 0x05_00_00_00 = 80 MiB)
    let mut oversized_v2 = MESSAGE_START_V2.to_vec();
    oversized_v2.push(FRAME_TYPE_DATA);
    oversized_v2.push(0x84);
    oversized_v2.extend_from_slice(&[0x05, 0x00, 0x00, 0x00]);
    let mut decryptor_oversized = StreamDecryptor::new(key.clone()).unwrap();
    assert!(decryptor_oversized.decrypt(&mut oversized_v2).is_err());

    // Crafted legacy v1 header claiming usize::MAX with legacy enabled must also fail without panic
    let mut crafted_v1 = MESSAGE_START.to_vec();
    crafted_v1.push(0x88);
    crafted_v1.extend_from_slice(&[0xFF; 8]);
    let mut decryptor_v1 = StreamDecryptor::new_with_legacy(key, true).unwrap();
    assert!(decryptor_v1.decrypt(&mut crafted_v1).is_err());
}

#[test]
fn test_decrypt_handles_split_multibyte_ber_length_header() {
    let key = EncryptionKey::generate();
    let mut encryptor = StreamEncryptor::new(key.clone()).unwrap();
    let mut decryptor = StreamDecryptor::new(key).unwrap();

    // 300 bytes produces a 2-byte BER long-form length (0x82, 0x01, 0x3C)
    let original = vec![b'A'; 300];
    let mut plain = original.clone();
    let mut cipher = encryptor.encrypt(&mut plain).unwrap();
    cipher.extend_from_slice(&encryptor.finish().unwrap());

    // Feed 1 byte at a time so multi-byte header splits across reads
    let mut buffer = Vec::new();
    let mut recovered = Vec::new();
    for &byte in &cipher {
        buffer.push(byte);
        let chunk = decryptor.decrypt(&mut buffer).unwrap();
        recovered.extend_from_slice(&chunk);
    }
    decryptor.finish(&buffer).unwrap();
    assert_eq!(recovered, original);
}

#[test]
fn test_decrypt_detects_truncation_and_trailing_data() {
    let key = EncryptionKey::generate();
    let mut encryptor = StreamEncryptor::new(key.clone()).unwrap();
    let mut data = b"chunk one".to_vec();
    let frame1 = encryptor.encrypt(&mut data).unwrap();
    let mut eof_frame = encryptor.finish().unwrap();

    // 1. Clean frame-boundary truncation (missing EndOfStream frame) must fail finish()
    let mut trunc_decryptor = StreamDecryptor::new(key.clone()).unwrap();
    let mut only_frame1 = frame1.clone();
    assert_eq!(
        trunc_decryptor.decrypt(&mut only_frame1).unwrap(),
        b"chunk one"
    );
    assert!(trunc_decryptor.finish(&only_frame1).is_err());

    // 2. Complete empty stream with only finish() frame succeeds
    let mut empty_enc = StreamEncryptor::new(key.clone()).unwrap();
    let mut empty_eof = empty_enc.finish().unwrap();
    let mut empty_dec = StreamDecryptor::new(key.clone()).unwrap();
    assert!(empty_dec.decrypt(&mut empty_eof).unwrap().is_empty());
    assert!(empty_dec.finish(&empty_eof).is_ok());

    // 3. Zero-byte completely truncated stream must fail finish() in v2 mode
    let mut zero_dec = StreamDecryptor::new(key.clone()).unwrap();
    assert!(zero_dec.finish(&[]).is_err());

    // 4. Trailing injected bytes after authenticated EndOfStream must fail
    let mut full = frame1;
    full.append(&mut eof_frame);
    full.extend_from_slice(b"extra");
    let mut trailing_dec = StreamDecryptor::new(key).unwrap();
    assert!(trailing_dec.decrypt(&mut full).is_err());
}

#[test]
fn test_decrypt_rejects_legacy_v1_unless_opted_in() {
    let key = EncryptionKey::generate();
    // Construct a synthetic legacy v1 ciphertext frame using encrypt_next without AAD
    let aead = XChaCha20Poly1305::new_from_slice(&key.key).unwrap();
    let mut raw_enc = EncryptorBE32::from_aead(aead, &key.nonce.into());
    let mut v1_cipher = raw_enc.encrypt_next(b"legacy payload".as_slice()).unwrap();
    let mut v1_framed = MESSAGE_START.to_vec();
    v1_framed.append(&mut MessageLength::new(v1_cipher.len()).encode());
    v1_framed.append(&mut v1_cipher);

    let mut strict_dec = StreamDecryptor::new(key.clone()).unwrap();
    let mut strict_input = v1_framed.clone();
    assert!(strict_dec.decrypt(&mut strict_input).is_err());

    let mut legacy_dec = StreamDecryptor::new_with_legacy(key, true).unwrap();
    let out = legacy_dec.decrypt(&mut v1_framed).unwrap();
    legacy_dec.finish(&v1_framed).unwrap();
    assert_eq!(out, b"legacy payload");
}

/// Stateful streaming AEAD encryptor backed by [`XChaCha20Poly1305`] and [`EncryptorBE32`].
///
/// Emits v2 framed chunks with AAD-authenticated headers and a final authenticated
/// end-of-stream marker via [`StreamEncryptor::finish`].
///
/// # Examples
///
/// ```
/// use datapipe::datapipe_types::{DatapipeError, EncryptionKey};
/// use datapipe::encryption::StreamEncryptor;
///
/// # fn main() -> Result<(), DatapipeError> {
/// let key = EncryptionKey::generate();
/// let mut encryptor = StreamEncryptor::new(key)?;
/// let mut plaintext = b"hello world".to_vec();
/// let mut ciphertext = encryptor.encrypt(&mut plaintext)?;
/// ciphertext.extend_from_slice(&encryptor.finish()?);
/// assert!(!ciphertext.is_empty());
/// assert!(plaintext.is_empty());
/// # Ok(())
/// # }
/// ```
pub struct StreamEncryptor {
    encryptor: Option<EncryptorBE32<XChaCha20Poly1305>>,
    finished: bool,
}

impl StreamEncryptor {
    /// Creates a new [`StreamEncryptor`] from the provided [`EncryptionKey`].
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::ValidationError`] if the key slice length is invalid.
    pub fn new(encryption_key: EncryptionKey) -> Result<Self, DatapipeError> {
        match XChaCha20Poly1305::new_from_slice(&encryption_key.key) {
            Ok(aead) => {
                let encryptor = EncryptorBE32::from_aead(aead, &encryption_key.nonce.into());
                Ok(Self {
                    encryptor: Some(encryptor),
                    finished: false,
                })
            }
            Err(error) => {
                let error_message = format!("Error initializing StreamEncryptor: {error}");
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    fn build_v2_header(frame_type: u8, cipher_len: usize) -> Vec<u8> {
        let len_bytes = MessageLength::new(cipher_len).encode();
        let mut header = Vec::with_capacity(MESSAGE_START_LENGTH + 1 + len_bytes.len());
        header.extend_from_slice(&MESSAGE_START_V2[..]);
        header.push(frame_type);
        header.extend_from_slice(&len_bytes);
        header
    }

    /// Encrypts the next chunk(s) in the stream using v2 framing and AAD-bound headers.
    ///
    /// Consumes and zeroizes `clear_data`. Oversized inputs are automatically split into
    /// bounded frames up to [`MAX_CIPHER_FRAME_LEN`] bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::EncryptionError`] if the stream has already been finalized
    /// via [`finish`](Self::finish) or if AEAD stream encryption fails.
    pub fn encrypt(&mut self, clear_data: &mut Vec<u8>) -> Result<Vec<u8>, DatapipeError> {
        let clear_data_length = clear_data.len();
        trace!("clear_data length is: {clear_data_length}");
        if self.finished {
            return Err(DatapipeError::EncryptionError(
                "Cannot encrypt data after StreamEncryptor has been finalized".to_string(),
            ));
        }
        if clear_data.is_empty() {
            return Ok(Vec::new());
        }

        let Some(ref mut encryptor) = self.encryptor else {
            return Err(DatapipeError::EncryptionError(
                "StreamEncryptor internal cipher state missing".to_string(),
            ));
        };

        let mut payload: Vec<u8> = std::mem::take(clear_data);
        let mut out = Vec::new();
        for chunk in payload.chunks(MAX_PLAINTEXT_CHUNK_LEN) {
            let expected_cipher_len = chunk.len() + AEAD_TAG_LENGTH;
            let header = Self::build_v2_header(FRAME_TYPE_DATA, expected_cipher_len);
            let mut cipher_data = encryptor.encrypt_next(Payload {
                msg: chunk,
                aad: &header,
            })?;
            out.extend_from_slice(&header);
            out.append(&mut cipher_data);
        }
        payload.fill(0);
        std::hint::black_box(&payload);
        Ok(out)
    }

    /// Finalizes the encrypted stream by emitting an authenticated `EndOfStream` frame
    /// (`encrypt_last`). Calling `finish` after already finalized returns an empty buffer.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::EncryptionError`] if finalizing AEAD encryption fails.
    pub fn finish(&mut self) -> Result<Vec<u8>, DatapipeError> {
        if self.finished {
            return Ok(Vec::new());
        }
        self.finished = true;
        let Some(encryptor) = self.encryptor.take() else {
            return Ok(Vec::new());
        };
        let header = Self::build_v2_header(FRAME_TYPE_LAST, AEAD_TAG_LENGTH);
        let mut cipher_data = encryptor.encrypt_last(Payload {
            msg: &[],
            aad: &header,
        })?;
        let mut out = Vec::with_capacity(header.len() + cipher_data.len());
        out.extend_from_slice(&header);
        out.append(&mut cipher_data);
        Ok(out)
    }
}

impl std::fmt::Debug for StreamEncryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("StreamEncryptor")
            .field("finished", &self.finished)
            .finish()
    }
}

enum DecryptFrameStep {
    NeedMoreData,
    Frame(Vec<u8>),
}

/// Stateful streaming AEAD decryptor backed by [`XChaCha20Poly1305`] and [`DecryptorBE32`].
///
/// By default, enforces v2 framing with authenticated end-of-stream verification and rejects
/// unauthenticated legacy v1 streams unless constructed with [`StreamDecryptor::new_with_legacy`].
///
/// # Examples
///
/// ```
/// use datapipe::datapipe_types::{DatapipeError, EncryptionKey};
/// use datapipe::encryption::{StreamDecryptor, StreamEncryptor};
///
/// # fn main() -> Result<(), DatapipeError> {
/// let key = EncryptionKey::generate();
/// let mut encryptor = StreamEncryptor::new(key.clone())?;
/// let mut decryptor = StreamDecryptor::new(key)?;
///
/// let mut plaintext = b"hello world".to_vec();
/// let mut ciphertext = encryptor.encrypt(&mut plaintext)?;
/// ciphertext.extend_from_slice(&encryptor.finish()?);
/// let decrypted = decryptor.decrypt(&mut ciphertext)?;
/// decryptor.finish(&ciphertext)?;
/// assert_eq!(decrypted, b"hello world");
/// # Ok(())
/// # }
/// ```
pub struct StreamDecryptor {
    decryptor: Option<DecryptorBE32<XChaCha20Poly1305>>,
    allow_legacy_v1: bool,
    saw_v2: bool,
    saw_v1: bool,
    eof_verified: bool,
}

impl StreamDecryptor {
    /// Creates a strict [`StreamDecryptor`] requiring v2 framing and authenticated EOF.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::ValidationError`] if the key slice length is invalid.
    pub fn new(encryption_key: EncryptionKey) -> Result<Self, DatapipeError> {
        Self::new_with_legacy(encryption_key, false)
    }

    /// Creates a [`StreamDecryptor`] with explicit control over legacy v1 stream compatibility.
    ///
    /// Legacy v1 streams lack AAD header authentication and an authenticated EOF marker,
    /// so `allow_legacy_v1` should be `false` unless decrypting legacy archives.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::ValidationError`] if the key slice length is invalid.
    pub fn new_with_legacy(
        encryption_key: EncryptionKey,
        allow_legacy_v1: bool,
    ) -> Result<Self, DatapipeError> {
        match XChaCha20Poly1305::new_from_slice(&encryption_key.key) {
            Ok(aead) => {
                let decryptor = DecryptorBE32::from_aead(aead, &encryption_key.nonce.into());
                Ok(Self {
                    decryptor: Some(decryptor),
                    allow_legacy_v1,
                    saw_v2: false,
                    saw_v1: false,
                    eof_verified: false,
                })
            }
            Err(error) => {
                let error_message = format!("Error initializing StreamDecryptor: {error}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    fn decrypt_v2_frame(
        &mut self,
        cipher_data: &mut Vec<u8>,
    ) -> Result<DecryptFrameStep, DatapipeError> {
        if self.saw_v1 {
            return Err(DatapipeError::EncryptionError(
                "Mixed legacy v1 and v2 encrypted frames in stream".to_string(),
            ));
        }
        let cipher_data_length = cipher_data.len();
        if cipher_data_length < MIN_PREFIX_LENGTH_V2 {
            return Ok(DecryptFrameStep::NeedMoreData);
        }

        let frame_type = cipher_data[MESSAGE_START_LENGTH];
        if frame_type != FRAME_TYPE_DATA && frame_type != FRAME_TYPE_LAST {
            let error_message = format!("Unknown encrypted frame type: 0x{frame_type:02x}");
            error!("{error_message}");
            return Err(DatapipeError::EncryptionError(error_message));
        }

        let length_offset = MESSAGE_START_LENGTH + 1;
        let message_length = match MessageLength::decode_checked(
            &cipher_data[length_offset..],
            MAX_CIPHER_FRAME_LEN,
        ) {
            DecodeLengthStatus::Complete(decoded) => decoded.decoded,
            DecodeLengthStatus::Incomplete => return Ok(DecryptFrameStep::NeedMoreData),
            DecodeLengthStatus::Invalid(msg) => {
                error!("{msg}");
                return Err(DatapipeError::EncryptionError(msg));
            }
        };

        let start = length_offset
            .checked_add(message_length.length_length)
            .ok_or_else(|| {
                DatapipeError::EncryptionError("Encrypted frame header offset overflow".to_string())
            })?;
        let end = start
            .checked_add(message_length.message_length)
            .ok_or_else(|| {
                DatapipeError::EncryptionError("Encrypted frame total length overflow".to_string())
            })?;
        if end > cipher_data_length {
            return Ok(DecryptFrameStep::NeedMoreData);
        }

        let message: Vec<u8> = cipher_data.drain(0..end).collect();
        trace!(
            "Draining {end} bytes from cipher_data; cipher_data length is now: {}",
            cipher_data.len()
        );
        let aad = &message[..start];
        let payload = Payload {
            msg: &message[start..end],
            aad,
        };

        self.saw_v2 = true;
        if frame_type == FRAME_TYPE_DATA {
            let Some(ref mut decryptor) = self.decryptor else {
                return Err(DatapipeError::EncryptionError(
                    "Encrypted data frame received after stream end".to_string(),
                ));
            };
            let clear_data = decryptor.decrypt_next(payload)?;
            Ok(DecryptFrameStep::Frame(clear_data))
        } else {
            let Some(decryptor) = self.decryptor.take() else {
                return Err(DatapipeError::EncryptionError(
                    "Duplicate encrypted end-of-stream frame received".to_string(),
                ));
            };
            let clear_data = decryptor.decrypt_last(payload)?;
            self.eof_verified = true;
            if !cipher_data.is_empty() {
                return Err(DatapipeError::EncryptionError(
                    "Unexpected trailing bytes after encrypted end-of-stream".to_string(),
                ));
            }
            Ok(DecryptFrameStep::Frame(clear_data))
        }
    }

    fn decrypt_v1_frame(
        &mut self,
        cipher_data: &mut Vec<u8>,
    ) -> Result<DecryptFrameStep, DatapipeError> {
        if !self.allow_legacy_v1 {
            let error_message = "Legacy v1 encrypted stream without authenticated end-of-stream is rejected; pass --decrypt-allow-legacy if decrypting legacy ciphertext".to_string();
            error!("{error_message}");
            return Err(DatapipeError::EncryptionError(error_message));
        }
        if self.saw_v2 {
            return Err(DatapipeError::EncryptionError(
                "Mixed v2 and legacy v1 encrypted frames in stream".to_string(),
            ));
        }
        let cipher_data_length = cipher_data.len();
        if cipher_data_length < MIN_PREFIX_LENGTH_V1 {
            return Ok(DecryptFrameStep::NeedMoreData);
        }

        let message_length = match MessageLength::decode_checked(
            &cipher_data[MESSAGE_START_LENGTH..],
            MAX_CIPHER_FRAME_LEN,
        ) {
            DecodeLengthStatus::Complete(decoded) => decoded.decoded,
            DecodeLengthStatus::Incomplete => return Ok(DecryptFrameStep::NeedMoreData),
            DecodeLengthStatus::Invalid(msg) => {
                error!("{msg}");
                return Err(DatapipeError::EncryptionError(msg));
            }
        };

        let start = MESSAGE_START_LENGTH
            .checked_add(message_length.length_length)
            .ok_or_else(|| {
                DatapipeError::EncryptionError("Encrypted frame header offset overflow".to_string())
            })?;
        let end = start
            .checked_add(message_length.message_length)
            .ok_or_else(|| {
                DatapipeError::EncryptionError("Encrypted frame total length overflow".to_string())
            })?;
        if end > cipher_data_length {
            return Ok(DecryptFrameStep::NeedMoreData);
        }

        let message: Vec<u8> = cipher_data.drain(0..end).collect();
        let Some(ref mut decryptor) = self.decryptor else {
            return Err(DatapipeError::EncryptionError(
                "StreamDecryptor already finalized".to_string(),
            ));
        };
        let clear_data = decryptor.decrypt_next(&message[start..end])?;
        self.saw_v1 = true;
        Ok(DecryptFrameStep::Frame(clear_data))
    }

    /// Note that this consumes complete framed messages from `cipher_data`.
    fn decrypt_one(
        &mut self,
        cipher_data: &mut Vec<u8>,
    ) -> Result<DecryptFrameStep, DatapipeError> {
        let cipher_data_length = cipher_data.len();
        trace!("cipher_data length is: {cipher_data_length}");
        if self.eof_verified && cipher_data_length > 0 {
            return Err(DatapipeError::EncryptionError(
                "Unexpected trailing bytes after encrypted end-of-stream".to_string(),
            ));
        }
        if cipher_data_length < MESSAGE_START_LENGTH {
            if cipher_data_length > 0 {
                warn!("Encrypted message header is incomplete; buffering more bytes");
            }
            return Ok(DecryptFrameStep::NeedMoreData);
        }

        let magic = &cipher_data[0..MESSAGE_START_LENGTH];
        if magic == MESSAGE_START_V2 {
            self.decrypt_v2_frame(cipher_data)
        } else if magic == MESSAGE_START {
            self.decrypt_v1_frame(cipher_data)
        } else {
            let error_message = "Encrypted message start sequence does not match!".to_string();
            error!("{error_message}");
            Err(DatapipeError::EncryptionError(error_message))
        }
    }

    /// Decrypts all complete framed messages currently buffered in `cipher_data`.
    ///
    /// Note that this consumes as much of the contents of `cipher_data` as possible,
    /// leaving any trailing incomplete frame buffered in `cipher_data` for subsequent calls.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::EncryptionError`] if frame headers are malformed, unexpected
    /// trailing bytes follow EOF, legacy v1 streams are disallowed, or AEAD authentication fails.
    pub fn decrypt(&mut self, cipher_data: &mut Vec<u8>) -> Result<Vec<u8>, DatapipeError> {
        let mut clear_data: Vec<u8> = Vec::new();
        while let DecryptFrameStep::Frame(mut chunk) = self.decrypt_one(cipher_data)? {
            clear_data.append(&mut chunk);
        }
        Ok(clear_data)
    }

    /// Verifies that no trailing incomplete ciphertext remains and, for v2 streams,
    /// confirms that an authenticated end-of-stream frame was received.
    ///
    /// # Errors
    ///
    /// Returns [`DatapipeError::EncryptionError`] if unconsumed ciphertext bytes remain or
    /// if the v2 stream was truncated before its authenticated end-of-stream marker.
    pub fn finish(&mut self, remaining_cipher_data: &[u8]) -> Result<(), DatapipeError> {
        if !remaining_cipher_data.is_empty() {
            let error_message = format!(
                "Encrypted stream truncated mid-frame ({} unconsumed trailing bytes)",
                remaining_cipher_data.len()
            );
            error!("{error_message}");
            return Err(DatapipeError::EncryptionError(error_message));
        }
        if !self.eof_verified && !(self.allow_legacy_v1 && !self.saw_v2) {
            let error_message =
                "Encrypted stream ended before authenticated end-of-stream marker".to_string();
            error!("{error_message}");
            return Err(DatapipeError::EncryptionError(error_message));
        }
        Ok(())
    }
}

impl std::fmt::Debug for StreamDecryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("StreamDecryptor")
            .field("allow_legacy_v1", &self.allow_legacy_v1)
            .field("eof_verified", &self.eof_verified)
            .finish()
    }
}
