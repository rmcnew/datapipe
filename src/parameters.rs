/// This struct gives all the parameters needed to start a datapipe instance
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::reader::Reader;
use crate::writer::Writer;

pub struct Parameters {
    pub reader: Reader,
    pub maybe_decryptor: Option<StreamDecryptor>,
    pub maybe_encryptor: Option<StreamEncryptor>,
    pub writers: Vec<Writer>,
}

impl std::default::Default for Parameters {
    fn default() -> Self {
        Self {
            reader: Reader::default(),
            maybe_decryptor: None,
            maybe_encryptor: None,
            writers: vec![Writer::default()],
        }
    }
}
