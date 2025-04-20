use crate::datapipe_types::{DatapipeError, EncryptionKey};
use chacha20poly1305::aead::stream::{self, DecryptorBE32, EncryptorBE32};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use log::error;

#[test]
fn test_encrypt_decrypt() {
    let key = EncryptionKey::generate();
    let mut encryptor = StreamEncryptor::new(key.clone()).unwrap();
    let mut decryptor = StreamDecryptor::new(key.clone()).unwrap();

    let plain = "There was a ship that put to sea; The name of the ship was the Billy of Tea; The winds blew up, her bow dipped down; O blow, my bully boys, blow.";
    let cipher = encryptor.encrypt(plain.as_bytes()).unwrap();
    let plain2 = String::from_utf8(decryptor.decrypt(&cipher).unwrap()).unwrap();
    assert_eq!(plain, &plain2);
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

    pub fn encrypt(&mut self, plain_data: &[u8]) -> Result<Vec<u8>, DatapipeError> {
        Ok(self.encryptor.encrypt_next(plain_data)?)
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

    pub fn decrypt(&mut self, cipher_data: &[u8]) -> Result<Vec<u8>, DatapipeError> {
        Ok(self.decryptor.decrypt_next(cipher_data)?)
    }
}
