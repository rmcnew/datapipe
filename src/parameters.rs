use crate::datapipe_types::DatapipeError;
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::reader::Reader;
use crate::writer::Writer;
use log::error;

/// Parameters needed to run datapipe
pub struct Parameters {
    /// Reader that will be used as the data source
    pub reader: Reader,
    /// optional StreamDecryption stage
    pub maybe_decryptor: Option<StreamDecryptor>,
    /// optional StreamEncryption stage
    pub maybe_encryptor: Option<StreamEncryptor>,
    /// Writer(s) that will be used as the data sinks
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
#[derive(Debug)]
pub struct ParametersBuilder {
    /// Reader that will be used as the data source
    maybe_reader: Option<Reader>,
    /// optional StreamDecryption stage
    maybe_decryptor: Option<StreamDecryptor>,
    /// optional StreamEncryption stage
    maybe_encryptor: Option<StreamEncryptor>,
    /// Writer(s) that will be used as the data sinks
    writers: Vec<Writer>,
}

impl ParametersBuilder {
    /// create a new ParametersBuilder
    pub fn new() -> Self {
        Self {
            maybe_reader: None,
            maybe_decryptor: None,
            maybe_encryptor: None,
            writers: Vec::new(),
        }
    }

    /// set the Reader
    pub fn reader(mut self, reader: Reader) -> Self {
        self.maybe_reader = Some(reader);
        self
    }

    /// set the StreamDecryptor
    pub fn decryptor(mut self, decryptor: StreamDecryptor) -> Self {
        self.maybe_decryptor = Some(decryptor);
        self
    }

    /// set the StreamEncryptor
    pub fn encryptor(mut self, encryptor: StreamEncryptor) -> Self {
        self.maybe_encryptor = Some(encryptor);
        self
    }

    /// add an additional Writer
    pub fn writer(mut self, writer: Writer) -> Self {
        self.writers.push(writer);
        self
    }

    /// build Parameters from this ParametersBuilder
    pub fn build(self) -> Result<Parameters, DatapipeError> {
        if self.maybe_reader.is_none() {
            let error_message =
                "No input source!  Please configure a Reader to provide input.".to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        if self.writers.is_empty() {
            let error_message =
                "No output destination!  Please configure at least one Writer for output."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        Ok(Parameters {
            reader: self.maybe_reader.ok_or_else(|| {
                DatapipeError::ValidationError(
                    "No input source! Please configure a Reader to provide input.".to_string(),
                )
            })?,
            maybe_decryptor: self.maybe_decryptor,
            maybe_encryptor: self.maybe_encryptor,
            writers: self.writers,
        })
    }
}

impl Default for ParametersBuilder {
    fn default() -> Self {
        Self::new()
    }
}
