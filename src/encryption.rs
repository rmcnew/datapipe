use crate::datapipe_types::{DatapipeError, EncryptionKey};
use chacha20poly1305::aead::stream::{self, DecryptorBE32, EncryptorBE32};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use log::{error, trace, warn};

// This stream encryption library appends a 16-byte authentication tag to the end of the
// encrypted message.  We use the following static tag to start each message in the
// encrypted stream, followed by a variable-byte length that gives the length of the encrypted
// message data AND the 16-byte authentication tag.  Thus each message in the encypted
// stream has the following structure:
// [MESSAGE_START][MESSAGE_LENGTH][encrypted message][authentication tag]
pub const MESSAGE_START: [u8; 4] = [0x29, 0x16, 0x4B, 0x74];
const MESSAGE_START_LENGTH: usize = 4;
const MIN_PREFIX_LENGTH: usize = 5; // MESSAGE_START length + minimum MessageLength length

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

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct MessageLength {
    pub length_length: usize,  // how many bytes does the length use?
    pub message_length: usize, // how many bytes does the respective value use?
}

impl MessageLength {
    // internal function so we can determine length_length from new
    pub fn encode_internal(message_length: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        match message_length {
            usize::MIN..=127 => {
                bytes.push(message_length as u8);
            }
            128..=255 => {
                // one byte
                bytes.push(0x81);
                bytes.push(message_length as u8);
            }
            256..=65_535 => {
                // two bytes
                bytes.push(0x82);
                let be_bytes = message_length.to_be_bytes();
                let last_two_bytes = &be_bytes[be_bytes.len() - 2..=be_bytes.len() - 1];
                bytes.extend_from_slice(last_two_bytes);
            }
            65_536..=16_777_215 => {
                // three bytes
                bytes.push(0x83);
                let be_bytes = message_length.to_be_bytes();
                let last_three_bytes = &be_bytes[be_bytes.len() - 3..=be_bytes.len() - 1];
                bytes.extend_from_slice(last_three_bytes);
            }
            16_777_216..=4_294_967_295 => {
                // four bytes
                bytes.push(0x84);
                let be_bytes = message_length.to_be_bytes();
                let last_four_bytes = &be_bytes[be_bytes.len() - 4..=be_bytes.len() - 1];
                bytes.extend_from_slice(last_four_bytes);
            }
            4_294_967_296..=1_099_511_627_775 => {
                // five bytes
                bytes.push(0x85);
                let be_bytes = message_length.to_be_bytes();
                let last_five_bytes = &be_bytes[be_bytes.len() - 5..=be_bytes.len() - 1];
                bytes.extend_from_slice(last_five_bytes);
            }
            1_099_511_627_776..=281_474_976_710_655 => {
                // six bytes
                bytes.push(0x86);
                let be_bytes = message_length.to_be_bytes();
                let last_six_bytes = &be_bytes[be_bytes.len() - 6..=be_bytes.len() - 1];
                bytes.extend_from_slice(last_six_bytes);
            }
            281_474_976_710_656..=72_057_594_037_927_935 => {
                // seven bytes
                bytes.push(0x87);
                let be_bytes = message_length.to_be_bytes();
                let last_seven_bytes = &be_bytes[be_bytes.len() - 7..=be_bytes.len() - 1];
                bytes.extend_from_slice(last_seven_bytes);
            }
            72_057_594_037_927_936..=usize::MAX => {
                // eight bytes
                bytes.push(0x88);
                let be_bytes = message_length.to_be_bytes();
                bytes.extend_from_slice(&be_bytes);
            }
            // when we go to 128-bit machines, more branches might be needed here
            _ => {
                panic!("Out of bounds MessageLength representation!");
            }
        }
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

    pub fn decode(bytes: &[u8]) -> Option<Decoded<Self>> {
        match bytes.first() {
            Some(b_0) => {
                if b_0 <= &127 {
                    // BER short form
                    Some(Decoded::new(
                        MessageLength {
                            length_length: 1,
                            message_length: *b_0 as usize,
                        },
                        1,
                    ))
                } else {
                    // BER long form
                    let length_byte_count = (b_0 & 0x7F) as usize;
                    if length_byte_count > 8 {
                        // Unlikely, but we do not have a way to represent this:
                        // usize MAX is (1 << 64) - 1
                        return None;
                    }
                    match bytes.get(1..=length_byte_count) {
                        Some(b_x) => {
                            //println!("b_x is {:?}", b_x);
                            let mut value: usize = 0;
                            for b in b_x {
                                //println!("b is {}, value is {}", b, value);
                                value = (value << 8) | (*b as usize);
                                //println!("value is now {}", value);
                            }
                            let length_len = length_byte_count + 1;

                            Some(Decoded::new(
                                MessageLength {
                                    length_length: length_len,
                                    message_length: value,
                                },
                                length_len,
                            ))
                        }
                        // Why couldn't we get the slice?
                        // Is the data corrupted?
                        None => None,
                    }
                }
            }
            None => None,
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
    let mut decryptor = StreamDecryptor::new(key.clone()).unwrap();

    let plain = "There once was a ship that put to sea; The name of the ship was the Billy of Tea; The winds blew up, her bow dipped down; blow, me bully boys, blow.".to_string();
    let mut plain_clone = plain.clone(); // keep original to compare against
    let mut cipher = encryptor
        .encrypt(unsafe { plain_clone.as_mut_vec() })
        .unwrap();
    let plain2 = String::from_utf8(decryptor.decrypt(&mut cipher).unwrap()).unwrap();
    assert_eq!(plain, plain2);
}

pub struct StreamEncryptor {
    encryptor: EncryptorBE32<XChaCha20Poly1305>,
}

impl StreamEncryptor {
    pub fn new(encryption_key: EncryptionKey) -> Result<Self, DatapipeError> {
        match XChaCha20Poly1305::new_from_slice(&encryption_key.key) {
            Ok(aead) => {
                let encryptor =
                    stream::EncryptorBE32::from_aead(aead, &encryption_key.nonce.into());
                Ok(Self { encryptor })
            }
            Err(error) => {
                let error_message = format!("Error initializing StreamEncryptor: {error}");
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    /// note that this consumes the contents of clear_data
    pub fn encrypt(&mut self, clear_data: &mut Vec<u8>) -> Result<Vec<u8>, DatapipeError> {
        let clear_data_length = clear_data.len();
        trace!("clear_data length is: {clear_data_length}");
        if clear_data.is_empty() {
            Ok(Vec::new())
        } else {
            let mut message = Vec::new();
            message.extend_from_slice(&MESSAGE_START[..]);
            let payload: Vec<u8> = std::mem::take(clear_data);
            let mut cipher_data = self.encryptor.encrypt_next(&payload[..])?;
            let cipher_data_message_length = MessageLength::new(cipher_data.len());
            message.append(&mut cipher_data_message_length.encode());
            message.append(&mut cipher_data);
            Ok(message)
        }
    }
}

pub struct StreamDecryptor {
    decryptor: DecryptorBE32<XChaCha20Poly1305>,
}

impl StreamDecryptor {
    pub fn new(encryption_key: EncryptionKey) -> Result<Self, DatapipeError> {
        match XChaCha20Poly1305::new_from_slice(&encryption_key.key) {
            Ok(aead) => {
                let decryptor =
                    stream::DecryptorBE32::from_aead(aead, &encryption_key.nonce.into());
                Ok(Self { decryptor })
            }
            Err(error) => {
                let error_message = format!("Error initializing StreamDecryptor: {error}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    /// note that this consumes the some of the contents of cipher_data
    fn decrypt_one(&mut self, cipher_data: &mut Vec<u8>) -> Result<Vec<u8>, DatapipeError> {
        let cipher_data_length = cipher_data.len();
        trace!("cipher_data length is: {cipher_data_length}");
        if cipher_data_length >= MIN_PREFIX_LENGTH {
            if cipher_data[0..MESSAGE_START_LENGTH] == MESSAGE_START {
                match MessageLength::decode(&cipher_data[MESSAGE_START_LENGTH..]) {
                    Some(decoded_length) => {
                        let message_length = decoded_length.decoded;
                        let start = MESSAGE_START_LENGTH + message_length.length_length;
                        let end = start + message_length.message_length;
                        if end > cipher_data_length {
                            // incomplete cipher data; wait for more data to arrive
                            return Ok(Vec::new());
                        }
                        let message: Vec<u8> = cipher_data.drain(0..end).collect();
                        trace!(
                            "Draining {end} bytes from cipher_data; cipher_data length is now: {}",
                            cipher_data.len()
                        );
                        let clear_data = self.decryptor.decrypt_next(&message[start..end])?;
                        Ok(clear_data)
                    }
                    None => {
                        let error_message =
                            "Could not decode encrypted message length!".to_string();
                        error!("{error_message}");
                        Err(DatapipeError::EncryptionError(error_message))
                    }
                }
            } else {
                // MESSAGE_START does not match
                let error_message = "Encrypted message start sequence does not match!".to_string();
                error!("{error_message}");
                Err(DatapipeError::EncryptionError(error_message))
            }
        } else {
            // cipher_data is too short
            let warn_message = "Encrypted message is too short to decrypt!".to_string();
            warn!("{warn_message}");
            Ok(Vec::new())
        }
    }

    /// note that this consumes as much of the contents of cipher_data as possible
    pub fn decrypt(&mut self, cipher_data: &mut Vec<u8>) -> Result<Vec<u8>, DatapipeError> {
        let mut clear_data: Vec<u8> = Vec::new();
        loop {
            let mut one_clear_data = self.decrypt_one(cipher_data)?;
            if one_clear_data.is_empty() {
                break;
            } else {
                clear_data.append(&mut one_clear_data);
            }
        }
        Ok(clear_data)
    }
}
