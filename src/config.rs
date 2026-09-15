//! Configuration file support for datapipe.
//!
//! This module provides the [`DatapipeConfig`] struct which represents a complete
//! configuration for a datapipe streaming pipeline in TOML format (`.toml`).
//!
//! It supports loading from file, saving to file, TOML serialization/deserialization,
//! and validation of datapipe parameters.

use crate::args::{
    DecryptionArgs, EncryptionArgs, HttpInputArgs, HttpOutputArgs, HttpsInputArgs, HttpsOutputArgs,
    InputArgs, LoggingArgs, OutputArgs, ProgramArgs, TlsInputArgs, TlsListenInputArgs,
    TlsOutputArgs,
};
use crate::datapipe_types::{DatapipeError, EncryptionKey, error_root_cause};
use log::error;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration structure for a datapipe instance, serializable to/from TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DatapipeConfig {
    /// Input source configuration
    #[serde(default, skip_serializing_if = "InputArgs::is_empty")]
    pub input: InputArgs,
    /// Additional HTTP input parameters
    #[serde(default, skip_serializing_if = "HttpInputArgs::is_empty")]
    pub http_input: HttpInputArgs,
    /// Additional HTTPS input parameters
    #[serde(default, skip_serializing_if = "HttpsInputArgs::is_empty")]
    pub https_input: HttpsInputArgs,
    /// Additional TLS input parameters
    #[serde(default, skip_serializing_if = "TlsInputArgs::is_empty")]
    pub tls_input: TlsInputArgs,
    /// Additional TLS listen input parameters
    #[serde(default, skip_serializing_if = "TlsListenInputArgs::is_empty")]
    pub tls_listen_input: TlsListenInputArgs,
    /// In-line decryption parameters
    #[serde(default, skip_serializing_if = "DecryptionArgs::is_empty")]
    pub decryption_args: DecryptionArgs,
    /// In-line encryption parameters
    #[serde(default, skip_serializing_if = "EncryptionArgs::is_empty")]
    pub encryption_args: EncryptionArgs,
    /// Output destinations configuration
    #[serde(default, skip_serializing_if = "OutputArgs::is_empty")]
    pub output: OutputArgs,
    /// Additional HTTP output parameters
    #[serde(default, skip_serializing_if = "HttpOutputArgs::is_empty")]
    pub http_output: HttpOutputArgs,
    /// Additional HTTPS output parameters
    #[serde(default, skip_serializing_if = "HttpsOutputArgs::is_empty")]
    pub https_output: HttpsOutputArgs,
    /// Additional TLS output parameters
    #[serde(default, skip_serializing_if = "TlsOutputArgs::is_empty")]
    pub tls_output: TlsOutputArgs,
    /// Logging configuration
    #[serde(default, skip_serializing_if = "LoggingArgs::is_empty")]
    pub logging_args: LoggingArgs,
}

impl DatapipeConfig {
    /// Create a new empty `DatapipeConfig`.
    ///
    /// # Example
    /// ```rust
    /// use datapipe::config::DatapipeConfig;
    ///
    /// let config = DatapipeConfig::new();
    /// assert!(config.input.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a `DatapipeConfig` from a TOML string.
    ///
    /// # Errors
    /// Returns [`DatapipeError::ValidationError`] if the TOML string cannot be parsed.
    ///
    /// # Example
    /// ```rust
    /// use datapipe::config::DatapipeConfig;
    ///
    /// # fn main() -> Result<(), datapipe::datapipe_types::DatapipeError> {
    /// let toml = r#"
    /// [input]
    /// file_input = "input.dat"
    ///
    /// [output]
    /// file_output = "output.dat"
    /// "#;
    /// let config = DatapipeConfig::from_toml_str(toml)?;
    /// assert_eq!(config.input.file_input.as_deref().unwrap().to_str().unwrap(), "input.dat");
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_toml_str(toml_str: &str) -> Result<Self, DatapipeError> {
        toml::from_str(toml_str).map_err(|error| {
            let error_message = format!("Error parsing TOML configuration: {error}");
            error!("{error_message}");
            DatapipeError::ValidationError(error_message)
        })
    }

    /// Serialize this configuration to a pretty-printed TOML string.
    ///
    /// # Errors
    /// Returns [`DatapipeError::ConfigurationError`] if serialization fails.
    ///
    /// # Example
    /// ```rust
    /// use datapipe::config::DatapipeConfig;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), datapipe::datapipe_types::DatapipeError> {
    /// let mut config = DatapipeConfig::new();
    /// config.input.file_input = Some(PathBuf::from("in.txt"));
    /// config.output.file_output = Some(PathBuf::from("out.txt"));
    /// let toml = config.to_toml_string()?;
    /// assert!(toml.contains("file_input = \"in.txt\""));
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_toml_string(&self) -> Result<String, DatapipeError> {
        toml::to_string_pretty(self).map_err(|error| {
            let error_message = format!("Error serializing configuration to TOML: {error}");
            error!("{error_message}");
            DatapipeError::ConfigurationError(error_message)
        })
    }

    /// Load and deserialize a `DatapipeConfig` from a TOML file on disk.
    ///
    /// # Errors
    /// Returns [`DatapipeError::InputOutputError`] if reading the file fails, or
    /// [`DatapipeError::ValidationError`] if parsing the TOML content fails.
    pub fn load_from_file(path: &Path) -> Result<Self, DatapipeError> {
        let content = std::fs::read_to_string(path).map_err(|error| {
            let error_message = format!(
                "Cannot read configuration file at {:?}: {}",
                path,
                error_root_cause(&error)
            );
            error!("{error_message}");
            DatapipeError::InputOutputError(error_message)
        })?;
        Self::from_toml_str(&content)
    }

    /// Serialize and save this configuration to a TOML file on disk.
    ///
    /// # Errors
    /// Returns [`DatapipeError::ConfigurationError`] if serialization fails, or
    /// [`DatapipeError::InputOutputError`] if writing the file fails.
    pub fn save_to_file(&self, path: &Path) -> Result<(), DatapipeError> {
        let toml_string = self.to_toml_string()?;
        std::fs::write(path, toml_string).map_err(|error| {
            let error_message = format!(
                "Cannot write configuration file to {:?}: {}",
                path,
                error_root_cause(&error)
            );
            error!("{error_message}");
            DatapipeError::InputOutputError(error_message)
        })?;
        Ok(())
    }

    /// Validate that this configuration satisfies all datapipe requirements:
    /// - Exactly one input source is defined.
    /// - At least one output destination is defined.
    /// - URL prefixes are valid for HTTP and HTTPS protocols.
    /// - TLS certificate and key pairs are consistent.
    /// - Encryption and decryption keys are valid 51-byte strings.
    ///
    /// # Errors
    /// Returns [`DatapipeError::ValidationError`] if any requirement is violated.
    ///
    /// # Example
    /// ```rust
    /// use datapipe::config::DatapipeConfig;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), datapipe::datapipe_types::DatapipeError> {
    /// let mut config = DatapipeConfig::new();
    /// config.input.file_input = Some(PathBuf::from("input.txt"));
    /// config.output.file_output = Some(PathBuf::from("output.txt"));
    /// config.validate()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self) -> Result<(), DatapipeError> {
        // Validate input: exactly one input source
        let mut input_count = 0;
        if self.input.file_input.is_some() {
            input_count += 1;
        }
        if self.input.http_input.is_some() {
            input_count += 1;
        }
        if self.input.https_input.is_some() {
            input_count += 1;
        }
        if self.input.stdin_input {
            input_count += 1;
        }
        if self.input.tcp_input.is_some() {
            input_count += 1;
        }
        if self.input.tcp_listen_input.is_some() {
            input_count += 1;
        }
        if self.input.tls_input.is_some() {
            input_count += 1;
        }
        if self.input.tls_listen_input.is_some() {
            input_count += 1;
        }
        if self.input.udp_input.is_some() {
            input_count += 1;
        }
        if self.input.udp_multicast_input.is_some() {
            input_count += 1;
        }

        if input_count == 0 {
            let error_message =
                "No input source provided! Exactly one input source must be configured."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        if input_count > 1 {
            let error_message = format!(
                "Multiple input sources configured ({input_count})! Exactly one input source must be configured."
            );
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate output: at least one output destination
        let mut output_count = 0;
        if self.output.file_output.is_some() {
            output_count += 1;
        }
        if self.output.http_output.is_some() {
            output_count += 1;
        }
        if self.output.https_output.is_some() {
            output_count += 1;
        }
        if self.output.stdout_output {
            output_count += 1;
        }
        if self.output.tcp_output.is_some() {
            output_count += 1;
        }
        if self.output.tls_output.is_some() {
            output_count += 1;
        }
        if self.output.udp_output.is_some() {
            output_count += 1;
        }

        if output_count == 0 {
            let error_message =
                "No output destination provided! At least one output destination must be configured."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate URL prefixes
        if let Some(ref url) = self.input.http_input
            && !url.starts_with("http://")
        {
            let error_message = format!("HTTP input URL '{url}' must start with 'http://'");
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        if let Some(ref url) = self.input.https_input
            && !url.starts_with("https://")
        {
            let error_message = format!("HTTPS input URL '{url}' must start with 'https://'");
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        if let Some(ref url) = self.output.http_output
            && !url.starts_with("http://")
        {
            let error_message = format!("HTTP output URL '{url}' must start with 'http://'");
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        if let Some(ref url) = self.output.https_output
            && !url.starts_with("https://")
        {
            let error_message = format!("HTTPS output URL '{url}' must start with 'https://'");
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate TLS input options consistency
        if self.input.tls_input.is_some()
            && self.tls_input.tls_input_cert_chain.is_some()
                != self.tls_input.tls_input_client_key.is_some()
        {
            let error_message =
                "Both TLS input certificate chain and client key must be specified together if either is used."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate TLS output options consistency
        if self.output.tls_output.is_some()
            && self.tls_output.tls_output_cert_chain.is_some()
                != self.tls_output.tls_output_client_key.is_some()
        {
            let error_message =
                "Both TLS output certificate chain and client key must be specified together if either is used."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate TLS listen input options consistency
        if self.input.tls_listen_input.is_some()
            && !self.tls_listen_input.tls_listen_input_generate_self_signed
            && (self.tls_listen_input.tls_listen_input_cert_chain.is_none()
                || self.tls_listen_input.tls_listen_input_server_key.is_none())
        {
            let error_message =
                "TLS listen input requires certificate chain and server key unless self-signed certificate generation is enabled."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate encryption key
        if let Some(ref key) = self.encryption_args.encryption_key {
            EncryptionKey::new(key)?;
        }
        if self.encryption_args.encryption_key.is_some()
            && self.encryption_args.generate_encryption_key
        {
            let error_message =
                "Cannot both provide an encryption key and request key generation simultaneously."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        // Validate decryption key
        if let Some(ref key) = self.decryption_args.decryption_key {
            EncryptionKey::new(key)?;
        }

        Ok(())
    }
}

impl From<&ProgramArgs> for DatapipeConfig {
    fn from(args: &ProgramArgs) -> Self {
        Self {
            input: args.input.clone(),
            http_input: args.http_input.clone(),
            https_input: args.https_input.clone(),
            tls_input: args.tls_input.clone(),
            tls_listen_input: args.tls_listen_input.clone(),
            decryption_args: args.decryption_args.clone(),
            encryption_args: args.encryption_args.clone(),
            output: args.output.clone(),
            http_output: args.http_output.clone(),
            https_output: args.https_output.clone(),
            tls_output: args.tls_output.clone(),
            logging_args: args.logging_args.clone(),
        }
    }
}

impl From<DatapipeConfig> for ProgramArgs {
    fn from(config: DatapipeConfig) -> Self {
        Self {
            use_config: None,
            save_to_config: None,
            verify_config: None,
            input: config.input,
            http_input: config.http_input,
            https_input: config.https_input,
            tls_input: config.tls_input,
            tls_listen_input: config.tls_listen_input,
            decryption_args: config.decryption_args,
            encryption_args: config.encryption_args,
            output: config.output,
            http_output: config.http_output,
            https_output: config.https_output,
            tls_output: config.tls_output,
            logging_args: config.logging_args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_roundtrip() -> Result<(), DatapipeError> {
        let mut config = DatapipeConfig::new();
        config.input.file_input = Some(PathBuf::from("source.dat"));
        config.output.file_output = Some(PathBuf::from("destination.dat"));
        config.logging_args.keep_logs = true;

        let toml_str = config.to_toml_string()?;
        let parsed_config = DatapipeConfig::from_toml_str(&toml_str)?;

        assert_eq!(config, parsed_config);
        config.validate()?;
        Ok(())
    }

    #[test]
    fn test_config_validate_no_input() {
        let mut config = DatapipeConfig::new();
        config.output.file_output = Some(PathBuf::from("destination.dat"));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_multiple_inputs() {
        let mut config = DatapipeConfig::new();
        config.input.file_input = Some(PathBuf::from("source.dat"));
        config.input.stdin_input = true;
        config.output.file_output = Some(PathBuf::from("destination.dat"));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_no_output() {
        let mut config = DatapipeConfig::new();
        config.input.file_input = Some(PathBuf::from("source.dat"));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_encryption_key() {
        let mut config = DatapipeConfig::new();
        config.input.file_input = Some(PathBuf::from("source.dat"));
        config.output.file_output = Some(PathBuf::from("destination.dat"));
        config.encryption_args.encryption_key = Some("short_key".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_http_url() {
        let mut config = DatapipeConfig::new();
        config.input.http_input = Some("ftp://bad-url.org".to_string());
        config.output.file_output = Some(PathBuf::from("destination.dat"));
        assert!(config.validate().is_err());
    }
}
