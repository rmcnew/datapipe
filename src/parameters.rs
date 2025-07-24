/// This struct gives all the parameters needed to start a datapipe instance
use crate::datapipe_types::DatapipeError;
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::reader::Reader;
use crate::writer::Writer;
use log::error;

/// Parameters needed to run datapipe
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

#[test]
fn test_parameters_builder_build() {
    use crate::stdin_reader::StdinReader;
    use crate::stdout_writer::StdoutWriter;

    let _parameters = ParametersBuilder::new()
        .reader(Reader::from(StdinReader::new()))
        .writer(Writer::from(StdoutWriter::new()))
        .build()
        .unwrap();
}

/// Builder for Parameters
pub struct ParametersBuilder {
    maybe_reader: Option<Reader>,
    maybe_decryptor: Option<StreamDecryptor>,
    maybe_encryptor: Option<StreamEncryptor>,
    writers: Vec<Writer>,
}

impl ParametersBuilder {
    pub fn new() -> Self {
        Self {
            maybe_reader: None,
            maybe_decryptor: None,
            maybe_encryptor: None,
            writers: Vec::new(),
        }
    }

    pub fn reader(mut self, reader: Reader) -> Self {
        self.maybe_reader = Some(reader);
        self
    }

    pub fn decryptor(mut self, decryptor: StreamDecryptor) -> Self {
        self.maybe_decryptor = Some(decryptor);
        self
    }

    pub fn encryptor(mut self, encryptor: StreamEncryptor) -> Self {
        self.maybe_encryptor = Some(encryptor);
        self
    }

    pub fn writer(mut self, writer: Writer) -> Self {
        self.writers.push(writer);
        self
    }

    pub fn build(self) -> Result<Parameters, DatapipeError> {
        if self.maybe_reader.is_none() {
            let error_message =
                format!("No input source!  Please configure a Reader to provide input.");
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        if self.writers.is_empty() {
            let error_message =
                format!("No output destination!  Please configure a Writer for output.");
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        Ok(Parameters {
            reader: self.maybe_reader.unwrap(),
            maybe_decryptor: self.maybe_decryptor,
            maybe_encryptor: self.maybe_encryptor,
            writers: self.writers,
        })
    }
}
