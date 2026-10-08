//! Data models and state structures for the datapipe GUI.

use crate::config::{DatapipeConfig, OutputDestinationConfig};
use crate::datapipe_types::EncryptionKey;
use std::path::PathBuf;

/// The active tab in the GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Input configuration tab
    #[default]
    Input,
    /// Decryption stage configuration tab
    Decrypt,
    /// Encryption stage configuration tab
    Encrypt,
    /// Output destination(s) configuration tab
    Output,
    /// Pipeline control and live metrics tab
    Run,
}

impl Tab {
    /// Returns the display title of the tab.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Input => "Input",
            Self::Decrypt => "Decrypt",
            Self::Encrypt => "Encrypt",
            Self::Output => "Output",
            Self::Run => "Run",
        }
    }
}

/// The available input source protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputType {
    /// Read from local file
    #[default]
    File,
    /// Poll HTTP URL
    Http,
    /// Poll HTTPS URL
    Https,
    /// Read from standard input
    Stdin,
    /// Connect to remote TCP address
    Tcp,
    /// Listen on local TCP port
    TcpListen,
    /// Connect to remote TLS address
    Tls,
    /// Listen on local TLS port
    TlsListen,
    /// Listen on UDP socket
    Udp,
    /// Join and listen on UDP multicast group
    UdpMulticast,
}

impl InputType {
    /// Returns all available input types.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::File,
            Self::Http,
            Self::Https,
            Self::Stdin,
            Self::Tcp,
            Self::TcpListen,
            Self::Tls,
            Self::TlsListen,
            Self::Udp,
            Self::UdpMulticast,
        ]
    }

    /// Returns the human-readable display name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Http => "HTTP",
            Self::Https => "HTTPS",
            Self::Stdin => "STDIN",
            Self::Tcp => "TCP",
            Self::TcpListen => "TCP Listen",
            Self::Tls => "TLS",
            Self::TlsListen => "TLS Listen",
            Self::Udp => "UDP",
            Self::UdpMulticast => "UDP Multicast",
        }
    }
}

/// GUI form state for the input stage.
#[derive(Debug, Clone, PartialEq)]
pub struct InputState {
    /// The currently selected input protocol
    pub input_type: InputType,
    /// Local file input path
    pub file_path: String,
    /// HTTP input URL
    pub http_url: String,
    /// HTTP polling rate in milliseconds
    pub http_rate: String,
    /// HTTPS input URL
    pub https_url: String,
    /// HTTPS polling rate in milliseconds
    pub https_rate: String,
    /// HTTPS custom root certificate bundle path
    pub https_root_certificates: String,
    /// HTTPS certificate revocation list path
    pub https_crl: String,
    /// HTTPS client identity certificate path
    pub https_client_identity: String,
    /// Allow invalid hostnames in HTTPS
    pub https_allow_invalid_hostnames: bool,
    /// Allow invalid certificates in HTTPS
    pub https_allow_invalid_certificates: bool,
    /// TCP remote address
    pub tcp_address: String,
    /// TCP listen local address
    pub tcp_listen_address: String,
    /// TLS remote address
    pub tls_address: String,
    /// TLS certificate chain path
    pub tls_cert_chain: String,
    /// TLS client private key path
    pub tls_client_key: String,
    /// TLS custom root CA path
    pub tls_root_ca: String,
    /// TLS skip server verification
    pub tls_skip_server_verify: bool,
    /// TLS listen local address
    pub tls_listen_address: String,
    /// TLS listen certificate chain path
    pub tls_listen_cert_chain: String,
    /// TLS listen server private key path
    pub tls_listen_server_key: String,
    /// TLS listen skip client verification
    pub tls_listen_skip_client_verify: bool,
    /// TLS listen generate self-signed certificate
    pub tls_listen_generate_self_signed: bool,
    /// UDP local bind address
    pub udp_address: String,
    /// UDP multicast bind address
    pub udp_multicast_address: String,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            input_type: InputType::File,
            file_path: String::new(),
            http_url: "http://".to_string(),
            http_rate: "1000".to_string(),
            https_url: "https://".to_string(),
            https_rate: "1000".to_string(),
            https_root_certificates: String::new(),
            https_crl: String::new(),
            https_client_identity: String::new(),
            https_allow_invalid_hostnames: false,
            https_allow_invalid_certificates: false,
            tcp_address: "127.0.0.1:8080".to_string(),
            tcp_listen_address: "0.0.0.0:8080".to_string(),
            tls_address: "127.0.0.1:8443".to_string(),
            tls_cert_chain: String::new(),
            tls_client_key: String::new(),
            tls_root_ca: String::new(),
            tls_skip_server_verify: false,
            tls_listen_address: "0.0.0.0:8443".to_string(),
            tls_listen_cert_chain: String::new(),
            tls_listen_server_key: String::new(),
            tls_listen_skip_client_verify: false,
            tls_listen_generate_self_signed: true,
            udp_address: "127.0.0.1:9000".to_string(),
            udp_multicast_address: "224.0.0.1:9000".to_string(),
        }
    }
}

impl InputState {
    /// Validates the input state and returns any validation errors.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        match self.input_type {
            InputType::File => {
                if self.file_path.trim().is_empty() {
                    errors.push("Input: File path is required.".to_string());
                }
            }
            InputType::Http => {
                if self.http_url.trim().is_empty() || self.http_url.trim() == "http://" {
                    errors.push("Input: HTTP URL is required.".to_string());
                } else if !self.http_url.starts_with("http://") {
                    errors.push("Input: HTTP URL must start with 'http://'.".to_string());
                }
                if self.http_rate.trim().parse::<u64>().is_err() {
                    errors.push(
                        "Input: HTTP poll rate must be a positive integer in milliseconds."
                            .to_string(),
                    );
                }
            }
            InputType::Https => {
                if self.https_url.trim().is_empty() || self.https_url.trim() == "https://" {
                    errors.push("Input: HTTPS URL is required.".to_string());
                } else if !self.https_url.starts_with("https://") {
                    errors.push("Input: HTTPS URL must start with 'https://'.".to_string());
                }
                if self.https_rate.trim().parse::<u64>().is_err() {
                    errors.push(
                        "Input: HTTPS poll rate must be a positive integer in milliseconds."
                            .to_string(),
                    );
                }
            }
            InputType::Stdin => {}
            InputType::Tcp => {
                if self.tcp_address.trim().is_empty() {
                    errors.push("Input: TCP address is required.".to_string());
                }
            }
            InputType::TcpListen => {
                if self.tcp_listen_address.trim().is_empty() {
                    errors.push("Input: TCP listen address is required.".to_string());
                }
            }
            InputType::Tls => {
                if self.tls_address.trim().is_empty() {
                    errors.push("Input: TLS address is required.".to_string());
                }
                let has_cert = !self.tls_cert_chain.trim().is_empty();
                let has_key = !self.tls_client_key.trim().is_empty();
                if has_cert != has_key {
                    errors.push("Input: TLS client certificate chain and client key must be provided together.".to_string());
                }
            }
            InputType::TlsListen => {
                if self.tls_listen_address.trim().is_empty() {
                    errors.push("Input: TLS listen address is required.".to_string());
                }
                if !self.tls_listen_generate_self_signed {
                    if self.tls_listen_cert_chain.trim().is_empty() {
                        errors.push("Input: TLS listen certificate chain is required unless self-signed certificate generation is enabled.".to_string());
                    }
                    if self.tls_listen_server_key.trim().is_empty() {
                        errors.push("Input: TLS listen server key is required unless self-signed certificate generation is enabled.".to_string());
                    }
                }
            }
            InputType::Udp => {
                if self.udp_address.trim().is_empty() {
                    errors.push("Input: UDP address is required.".to_string());
                }
            }
            InputType::UdpMulticast => {
                if self.udp_multicast_address.trim().is_empty() {
                    errors.push("Input: UDP multicast address is required.".to_string());
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// GUI state for the decryption stage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DecryptState {
    /// Whether decryption is enabled
    pub enabled: bool,
    /// 51-byte UTF-8 decryption key
    pub key: String,
}

impl DecryptState {
    /// Validates the decryption parameters.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.enabled {
            if self.key.trim().is_empty() {
                errors.push(
                    "Decrypt: 51-byte UTF-8 Decryption Key is required when decryption is enabled."
                        .to_string(),
                );
            } else if self.key.len() != 51 {
                errors.push(format!(
                    "Decrypt: Decryption key must be exactly 51 bytes long (currently {} bytes).",
                    self.key.len()
                ));
            } else if let Err(e) = EncryptionKey::new(&self.key) {
                errors.push(format!("Decrypt: Invalid decryption key: {e}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// GUI state for the encryption stage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncryptState {
    /// Whether encryption is enabled
    pub enabled: bool,
    /// 51-byte UTF-8 encryption key
    pub key: String,
}

impl EncryptState {
    /// Generates a random 51-byte encryption key.
    pub fn generate_key(&mut self) {
        let key = EncryptionKey::generate();
        self.key = key.to_string();
    }

    /// Validates the encryption parameters.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.enabled {
            if self.key.trim().is_empty() {
                errors.push(
                    "Encrypt: 51-byte UTF-8 Encryption Key is required when encryption is enabled."
                        .to_string(),
                );
            } else if self.key.len() != 51 {
                errors.push(format!(
                    "Encrypt: Encryption key must be exactly 51 bytes long (currently {} bytes).",
                    self.key.len()
                ));
            } else if let Err(e) = EncryptionKey::new(&self.key) {
                errors.push(format!("Encrypt: Invalid encryption key: {e}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Available output destination protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputType {
    /// Write to local file
    #[default]
    File,
    /// Write to standard output
    Stdout,
    /// Send over TCP socket
    Tcp,
    /// Send over UDP socket
    Udp,
    /// Send HTTP POST requests
    Http,
    /// Send HTTPS POST requests
    Https,
    /// Send over TLS socket
    Tls,
}

impl OutputType {
    /// Returns all available output types.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::File,
            Self::Stdout,
            Self::Tcp,
            Self::Udp,
            Self::Http,
            Self::Https,
            Self::Tls,
        ]
    }

    /// Returns the human-readable display name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Stdout => "STDOUT",
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::Http => "HTTP",
            Self::Https => "HTTPS",
            Self::Tls => "TLS",
        }
    }
}

/// GUI form state for a single output destination.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputState {
    /// Selected output destination protocol
    pub output_type: OutputType,
    /// Local file output path
    pub file_path: String,
    /// TCP remote address
    pub tcp_address: String,
    /// UDP remote address
    pub udp_address: String,
    /// HTTP destination URL
    pub http_url: String,
    /// HTTP transmission rate in milliseconds
    pub http_rate: String,
    /// HTTP delimiter string
    pub http_delimiter: String,
    /// Include delimiter in HTTP payload
    pub http_include_delimiter: bool,
    /// HTTPS destination URL
    pub https_url: String,
    /// HTTPS transmission rate in milliseconds
    pub https_rate: String,
    /// HTTPS delimiter string
    pub https_delimiter: String,
    /// Include delimiter in HTTPS payload
    pub https_include_delimiter: bool,
    /// HTTPS custom root certificate bundle path
    pub https_root_certificates: String,
    /// HTTPS certificate revocation list path
    pub https_crl: String,
    /// HTTPS client identity certificate path
    pub https_client_identity: String,
    /// Allow invalid hostnames in HTTPS
    pub https_allow_invalid_hostnames: bool,
    /// Allow invalid certificates in HTTPS
    pub https_allow_invalid_certificates: bool,
    /// TLS remote address
    pub tls_address: String,
    /// TLS client certificate chain path
    pub tls_cert_chain: String,
    /// TLS client private key path
    pub tls_client_key: String,
    /// TLS custom root CA path
    pub tls_root_ca: String,
    /// TLS skip server verification
    pub tls_skip_server_verify: bool,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            output_type: OutputType::File,
            file_path: String::new(),
            tcp_address: "127.0.0.1:8080".to_string(),
            udp_address: "127.0.0.1:9000".to_string(),
            http_url: "http://".to_string(),
            http_rate: "1000".to_string(),
            http_delimiter: "\\n".to_string(),
            http_include_delimiter: true,
            https_url: "https://".to_string(),
            https_rate: "1000".to_string(),
            https_delimiter: "\\n".to_string(),
            https_include_delimiter: true,
            https_root_certificates: String::new(),
            https_crl: String::new(),
            https_client_identity: String::new(),
            https_allow_invalid_hostnames: false,
            https_allow_invalid_certificates: false,
            tls_address: "127.0.0.1:8443".to_string(),
            tls_cert_chain: String::new(),
            tls_client_key: String::new(),
            tls_root_ca: String::new(),
            tls_skip_server_verify: false,
        }
    }
}

impl OutputState {
    /// Validates this output destination and returns any validation errors.
    pub fn validate(&self, index: usize) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let prefix = format!("Output #{}: ", index + 1);

        match self.output_type {
            OutputType::File => {
                if self.file_path.trim().is_empty() {
                    errors.push(format!("{prefix}File path is required."));
                }
            }
            OutputType::Stdout => {}
            OutputType::Tcp => {
                if self.tcp_address.trim().is_empty() {
                    errors.push(format!("{prefix}TCP address is required."));
                }
            }
            OutputType::Udp => {
                if self.udp_address.trim().is_empty() {
                    errors.push(format!("{prefix}UDP address is required."));
                }
            }
            OutputType::Http => {
                if self.http_url.trim().is_empty() || self.http_url.trim() == "http://" {
                    errors.push(format!("{prefix}HTTP URL is required."));
                } else if !self.http_url.starts_with("http://") {
                    errors.push(format!("{prefix}HTTP URL must start with 'http://'."));
                }
                if self.http_rate.trim().parse::<u64>().is_err() {
                    errors.push(format!(
                        "{prefix}HTTP send rate must be a positive integer in milliseconds."
                    ));
                }
            }
            OutputType::Https => {
                if self.https_url.trim().is_empty() || self.https_url.trim() == "https://" {
                    errors.push(format!("{prefix}HTTPS URL is required."));
                } else if !self.https_url.starts_with("https://") {
                    errors.push(format!("{prefix}HTTPS URL must start with 'https://'."));
                }
                if self.https_rate.trim().parse::<u64>().is_err() {
                    errors.push(format!(
                        "{prefix}HTTPS send rate must be a positive integer in milliseconds."
                    ));
                }
            }
            OutputType::Tls => {
                if self.tls_address.trim().is_empty() {
                    errors.push(format!("{prefix}TLS address is required."));
                }
                let has_cert = !self.tls_cert_chain.trim().is_empty();
                let has_key = !self.tls_client_key.trim().is_empty();
                if has_cert != has_key {
                    errors.push(format!("{prefix}TLS client certificate chain and client key must be provided together."));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// The runtime status of the pipeline.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PipelineStatus {
    /// Pipeline is idle
    #[default]
    Idle,
    /// Configuration is valid and ready to run
    Ready,
    /// Pipeline is running and transferring data
    Running,
    /// Pipeline was stopped by user
    Stopped,
    /// Pipeline completed successfully
    Completed,
    /// An error occurred during pipeline startup or execution
    Error(String),
}

impl PipelineStatus {
    /// Returns the human-readable status label.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Ready => "Ready",
            Self::Running => "Running",
            Self::Stopped => "Stopped",
            Self::Completed => "Completed",
            Self::Error(_) => "Error",
        }
    }
}

/// Asynchronous pipeline event message from background task to GUI.
#[derive(Debug)]
pub enum PipelineEvent {
    /// Pipeline has started
    Started,
    /// Pipeline finished normally or with error
    Finished(Result<(), String>),
}

/// Helper function to convert GUI states to a [`DatapipeConfig`].
pub fn states_to_config(
    input: &InputState,
    decrypt: &DecryptState,
    encrypt: &EncryptState,
    outputs: &[OutputState],
) -> Result<DatapipeConfig, Vec<String>> {
    let mut errors = Vec::new();
    if let Err(errs) = input.validate() {
        errors.extend(errs);
    }
    if let Err(errs) = decrypt.validate() {
        errors.extend(errs);
    }
    if let Err(errs) = encrypt.validate() {
        errors.extend(errs);
    }
    if outputs.is_empty() {
        errors.push("At least one output destination is required.".to_string());
    } else {
        for (i, out) in outputs.iter().enumerate() {
            if let Err(errs) = out.validate(i) {
                errors.extend(errs);
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut config = DatapipeConfig::new();

    // Input configuration
    match input.input_type {
        InputType::File => {
            config.input.file_input = Some(PathBuf::from(input.file_path.trim()));
        }
        InputType::Http => {
            config.input.http_input = Some(input.http_url.trim().to_string());
            if let Ok(rate) = input.http_rate.trim().parse::<u64>() {
                config.http_input.http_input_rate = Some(rate);
            }
        }
        InputType::Https => {
            config.input.https_input = Some(input.https_url.trim().to_string());
            if let Ok(rate) = input.https_rate.trim().parse::<u64>() {
                config.https_input.https_input_rate = Some(rate);
            }
            if !input.https_root_certificates.trim().is_empty() {
                config.https_input.https_input_root_certificates =
                    Some(PathBuf::from(input.https_root_certificates.trim()));
            }
            if !input.https_crl.trim().is_empty() {
                config.https_input.https_input_certificate_revocation_list =
                    Some(PathBuf::from(input.https_crl.trim()));
            }
            if !input.https_client_identity.trim().is_empty() {
                config.https_input.https_input_client_identity =
                    Some(PathBuf::from(input.https_client_identity.trim()));
            }
            config.https_input.https_input_allow_invalid_hostnames =
                input.https_allow_invalid_hostnames;
            config.https_input.https_input_allow_invalid_certificates =
                input.https_allow_invalid_certificates;
        }
        InputType::Stdin => {
            config.input.stdin_input = true;
        }
        InputType::Tcp => {
            config.input.tcp_input = Some(input.tcp_address.trim().to_string());
        }
        InputType::TcpListen => {
            config.input.tcp_listen_input = Some(input.tcp_listen_address.trim().to_string());
        }
        InputType::Tls => {
            config.input.tls_input = Some(input.tls_address.trim().to_string());
            if !input.tls_cert_chain.trim().is_empty() {
                config.tls_input.tls_input_cert_chain =
                    Some(PathBuf::from(input.tls_cert_chain.trim()));
            }
            if !input.tls_client_key.trim().is_empty() {
                config.tls_input.tls_input_client_key =
                    Some(PathBuf::from(input.tls_client_key.trim()));
            }
            if !input.tls_root_ca.trim().is_empty() {
                config.tls_input.tls_input_root_ca = Some(PathBuf::from(input.tls_root_ca.trim()));
            }
            config.tls_input.tls_input_skip_server_verify = input.tls_skip_server_verify;
        }
        InputType::TlsListen => {
            config.input.tls_listen_input = Some(input.tls_listen_address.trim().to_string());
            if !input.tls_listen_cert_chain.trim().is_empty() {
                config.tls_listen_input.tls_listen_input_cert_chain =
                    Some(PathBuf::from(input.tls_listen_cert_chain.trim()));
            }
            if !input.tls_listen_server_key.trim().is_empty() {
                config.tls_listen_input.tls_listen_input_server_key =
                    Some(PathBuf::from(input.tls_listen_server_key.trim()));
            }
            config.tls_listen_input.tls_listen_input_skip_client_verify =
                input.tls_listen_skip_client_verify;
            config
                .tls_listen_input
                .tls_listen_input_generate_self_signed = input.tls_listen_generate_self_signed;
        }
        InputType::Udp => {
            config.input.udp_input = Some(input.udp_address.trim().to_string());
        }
        InputType::UdpMulticast => {
            config.input.udp_multicast_input = Some(input.udp_multicast_address.trim().to_string());
        }
    }

    // Decrypt configuration
    if decrypt.enabled {
        config.decryption_args.decryption_key = Some(decrypt.key.clone());
    }

    // Encrypt configuration
    if encrypt.enabled {
        config.encryption_args.encryption_key = Some(encrypt.key.clone());
    }

    // Outputs configuration
    let mut output_configs = Vec::new();
    for out in outputs {
        let mut dest = OutputDestinationConfig::new();
        match out.output_type {
            OutputType::File => {
                dest.output.file_output = Some(PathBuf::from(out.file_path.trim()));
            }
            OutputType::Stdout => {
                dest.output.stdout_output = true;
            }
            OutputType::Tcp => {
                dest.output.tcp_output = Some(out.tcp_address.trim().to_string());
            }
            OutputType::Udp => {
                dest.output.udp_output = Some(out.udp_address.trim().to_string());
            }
            OutputType::Http => {
                dest.output.http_output = Some(out.http_url.trim().to_string());
                if let Ok(rate) = out.http_rate.trim().parse::<u64>() {
                    dest.http_output.http_output_rate = Some(rate);
                }
                dest.http_output.http_output_include_delimiter = out.http_include_delimiter;
            }
            OutputType::Https => {
                dest.output.https_output = Some(out.https_url.trim().to_string());
                if let Ok(rate) = out.https_rate.trim().parse::<u64>() {
                    dest.https_output.https_output_rate = Some(rate);
                }
                dest.https_output.https_output_include_delimiter =
                    Some(out.https_include_delimiter);
                if !out.https_root_certificates.trim().is_empty() {
                    dest.https_output.https_output_root_certificates =
                        Some(PathBuf::from(out.https_root_certificates.trim()));
                }
                if !out.https_crl.trim().is_empty() {
                    dest.https_output.https_output_certificate_revocation_list =
                        Some(PathBuf::from(out.https_crl.trim()));
                }
                if !out.https_client_identity.trim().is_empty() {
                    dest.https_output.https_output_client_identity =
                        Some(PathBuf::from(out.https_client_identity.trim()));
                }
                dest.https_output.https_output_allow_invalid_hostnames =
                    out.https_allow_invalid_hostnames;
                dest.https_output.https_output_allow_invalid_certificates =
                    out.https_allow_invalid_certificates;
            }
            OutputType::Tls => {
                dest.output.tls_output = Some(out.tls_address.trim().to_string());
                if !out.tls_cert_chain.trim().is_empty() {
                    dest.tls_output.tls_output_cert_chain =
                        Some(PathBuf::from(out.tls_cert_chain.trim()));
                }
                if !out.tls_client_key.trim().is_empty() {
                    dest.tls_output.tls_output_client_key =
                        Some(PathBuf::from(out.tls_client_key.trim()));
                }
                if !out.tls_root_ca.trim().is_empty() {
                    dest.tls_output.tls_output_root_ca =
                        Some(PathBuf::from(out.tls_root_ca.trim()));
                }
                dest.tls_output.tls_output_skip_server_verify = out.tls_skip_server_verify;
            }
        }
        output_configs.push(dest);
    }

    if let Some(first) = output_configs.first() {
        config.output = first.output.clone();
        config.http_output = first.http_output.clone();
        config.https_output = first.https_output.clone();
        config.tls_output = first.tls_output.clone();
    }
    config.outputs = Some(output_configs);

    Ok(config)
}

/// Helper function to load a [`DatapipeConfig`] into GUI states.
pub fn load_config_into_states(
    config: &DatapipeConfig,
    input: &mut InputState,
    decrypt: &mut DecryptState,
    encrypt: &mut EncryptState,
    outputs: &mut Vec<OutputState>,
) {
    // Populate Input
    if let Some(ref path) = config.input.file_input {
        input.input_type = InputType::File;
        input.file_path = path.display().to_string();
    } else if let Some(ref url) = config.input.http_input {
        input.input_type = InputType::Http;
        input.http_url = url.clone();
        if let Some(rate) = config.http_input.http_input_rate {
            input.http_rate = rate.to_string();
        }
    } else if let Some(ref url) = config.input.https_input {
        input.input_type = InputType::Https;
        input.https_url = url.clone();
        if let Some(rate) = config.https_input.https_input_rate {
            input.https_rate = rate.to_string();
        }
        if let Some(ref path) = config.https_input.https_input_root_certificates {
            input.https_root_certificates = path.display().to_string();
        }
        if let Some(ref path) = config.https_input.https_input_certificate_revocation_list {
            input.https_crl = path.display().to_string();
        }
        if let Some(ref path) = config.https_input.https_input_client_identity {
            input.https_client_identity = path.display().to_string();
        }
        input.https_allow_invalid_hostnames =
            config.https_input.https_input_allow_invalid_hostnames;
        input.https_allow_invalid_certificates =
            config.https_input.https_input_allow_invalid_certificates;
    } else if config.input.stdin_input {
        input.input_type = InputType::Stdin;
    } else if let Some(ref addr) = config.input.tcp_input {
        input.input_type = InputType::Tcp;
        input.tcp_address = addr.clone();
    } else if let Some(ref addr) = config.input.tcp_listen_input {
        input.input_type = InputType::TcpListen;
        input.tcp_listen_address = addr.clone();
    } else if let Some(ref addr) = config.input.tls_input {
        input.input_type = InputType::Tls;
        input.tls_address = addr.clone();
        if let Some(ref path) = config.tls_input.tls_input_cert_chain {
            input.tls_cert_chain = path.display().to_string();
        }
        if let Some(ref path) = config.tls_input.tls_input_client_key {
            input.tls_client_key = path.display().to_string();
        }
        if let Some(ref path) = config.tls_input.tls_input_root_ca {
            input.tls_root_ca = path.display().to_string();
        }
        input.tls_skip_server_verify = config.tls_input.tls_input_skip_server_verify;
    } else if let Some(ref addr) = config.input.tls_listen_input {
        input.input_type = InputType::TlsListen;
        input.tls_listen_address = addr.clone();
        if let Some(ref path) = config.tls_listen_input.tls_listen_input_cert_chain {
            input.tls_listen_cert_chain = path.display().to_string();
        }
        if let Some(ref path) = config.tls_listen_input.tls_listen_input_server_key {
            input.tls_listen_server_key = path.display().to_string();
        }
        input.tls_listen_skip_client_verify =
            config.tls_listen_input.tls_listen_input_skip_client_verify;
        input.tls_listen_generate_self_signed = config
            .tls_listen_input
            .tls_listen_input_generate_self_signed;
    } else if let Some(ref addr) = config.input.udp_input {
        input.input_type = InputType::Udp;
        input.udp_address = addr.clone();
    } else if let Some(ref addr) = config.input.udp_multicast_input {
        input.input_type = InputType::UdpMulticast;
        input.udp_multicast_address = addr.clone();
    }

    // Populate Decrypt
    if let Some(ref key) = config.decryption_args.decryption_key {
        decrypt.enabled = true;
        decrypt.key = key.clone();
    } else {
        decrypt.enabled = false;
        decrypt.key.clear();
    }

    // Populate Encrypt
    if let Some(ref key) = config.encryption_args.encryption_key {
        encrypt.enabled = true;
        encrypt.key = key.clone();
    } else {
        encrypt.enabled = false;
        encrypt.key.clear();
    }

    // Populate Outputs
    outputs.clear();
    if let Some(ref output_list) = config.outputs
        && !output_list.is_empty()
    {
        for dest in output_list {
            let mut state = OutputState::default();
            populate_single_output(dest, &mut state);
            outputs.push(state);
        }
    } else {
        let mut state = OutputState::default();
        let dest = OutputDestinationConfig {
            output: config.output.clone(),
            http_output: config.http_output.clone(),
            https_output: config.https_output.clone(),
            tls_output: config.tls_output.clone(),
        };
        populate_single_output(&dest, &mut state);
        outputs.push(state);
    }
}

fn populate_single_output(dest: &OutputDestinationConfig, state: &mut OutputState) {
    if let Some(ref path) = dest.output.file_output {
        state.output_type = OutputType::File;
        state.file_path = path.display().to_string();
    } else if dest.output.stdout_output {
        state.output_type = OutputType::Stdout;
    } else if let Some(ref addr) = dest.output.tcp_output {
        state.output_type = OutputType::Tcp;
        state.tcp_address = addr.clone();
    } else if let Some(ref addr) = dest.output.udp_output {
        state.output_type = OutputType::Udp;
        state.udp_address = addr.clone();
    } else if let Some(ref url) = dest.output.http_output {
        state.output_type = OutputType::Http;
        state.http_url = url.clone();
        if let Some(rate) = dest.http_output.http_output_rate {
            state.http_rate = rate.to_string();
        }
        state.http_include_delimiter = dest.http_output.http_output_include_delimiter;
    } else if let Some(ref url) = dest.output.https_output {
        state.output_type = OutputType::Https;
        state.https_url = url.clone();
        if let Some(rate) = dest.https_output.https_output_rate {
            state.https_rate = rate.to_string();
        }
        if let Some(include) = dest.https_output.https_output_include_delimiter {
            state.https_include_delimiter = include;
        }
        if let Some(ref path) = dest.https_output.https_output_root_certificates {
            state.https_root_certificates = path.display().to_string();
        }
        if let Some(ref path) = dest.https_output.https_output_certificate_revocation_list {
            state.https_crl = path.display().to_string();
        }
        if let Some(ref path) = dest.https_output.https_output_client_identity {
            state.https_client_identity = path.display().to_string();
        }
        state.https_allow_invalid_hostnames =
            dest.https_output.https_output_allow_invalid_hostnames;
        state.https_allow_invalid_certificates =
            dest.https_output.https_output_allow_invalid_certificates;
    } else if let Some(ref addr) = dest.output.tls_output {
        state.output_type = OutputType::Tls;
        state.tls_address = addr.clone();
        if let Some(ref path) = dest.tls_output.tls_output_cert_chain {
            state.tls_cert_chain = path.display().to_string();
        }
        if let Some(ref path) = dest.tls_output.tls_output_client_key {
            state.tls_client_key = path.display().to_string();
        }
        if let Some(ref path) = dest.tls_output.tls_output_root_ca {
            state.tls_root_ca = path.display().to_string();
        }
        state.tls_skip_server_verify = dest.tls_output.tls_output_skip_server_verify;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_titles() {
        assert_eq!(Tab::Input.title(), "Input");
        assert_eq!(Tab::Decrypt.title(), "Decrypt");
        assert_eq!(Tab::Encrypt.title(), "Encrypt");
        assert_eq!(Tab::Output.title(), "Output");
        assert_eq!(Tab::Run.title(), "Run");
    }

    #[test]
    fn test_input_type_all() {
        assert_eq!(InputType::all().len(), 10);
        assert_eq!(InputType::File.name(), "File");
        assert_eq!(InputType::Http.name(), "HTTP");
        assert_eq!(InputType::Https.name(), "HTTPS");
        assert_eq!(InputType::Stdin.name(), "STDIN");
    }

    #[test]
    fn test_input_validation_file() {
        let mut state = InputState {
            input_type: InputType::File,
            ..Default::default()
        };
        assert!(state.validate().is_err());

        state.file_path = "valid_input.txt".to_string();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_input_validation_http() {
        let mut state = InputState {
            input_type: InputType::Http,
            http_url: "ftp://bad.com".to_string(),
            ..Default::default()
        };
        assert!(state.validate().is_err());

        state.http_url = "http://valid.com/api".to_string();
        state.http_rate = "not_a_number".to_string();
        assert!(state.validate().is_err());

        state.http_rate = "500".to_string();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_decrypt_validation() {
        let mut state = DecryptState::default();
        assert!(state.validate().is_ok()); // disabled, ok

        state.enabled = true;
        state.key = "too_short".to_string();
        assert!(state.validate().is_err());

        let key = EncryptionKey::generate();
        state.key = key.to_string();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_encrypt_validation_and_generate() {
        let mut state = EncryptState::default();
        assert!(state.validate().is_ok());

        state.enabled = true;
        state.key = String::new();
        assert!(state.validate().is_err());

        state.generate_key();
        assert_eq!(state.key.len(), 51);
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_output_validation() {
        let mut state = OutputState {
            output_type: OutputType::File,
            ..Default::default()
        };
        assert!(state.validate(0).is_err());

        state.file_path = "output.dat".to_string();
        assert!(state.validate(0).is_ok());

        state.output_type = OutputType::Stdout;
        assert!(state.validate(0).is_ok());
    }

    #[test]
    fn test_states_to_config_roundtrip() -> Result<(), String> {
        let input = InputState {
            input_type: InputType::File,
            file_path: "source.dat".to_string(),
            ..Default::default()
        };

        let decrypt = DecryptState {
            enabled: true,
            key: EncryptionKey::generate().to_string(),
        };

        let mut encrypt = EncryptState {
            enabled: true,
            ..Default::default()
        };
        encrypt.generate_key();

        let out1 = OutputState {
            output_type: OutputType::File,
            file_path: "dest1.dat".to_string(),
            ..Default::default()
        };

        let out2 = OutputState {
            output_type: OutputType::Stdout,
            ..Default::default()
        };

        let outputs = vec![out1, out2];

        let config = states_to_config(&input, &decrypt, &encrypt, &outputs)
            .map_err(|errs| errs.join("; "))?;
        assert!(config.validate().is_ok());

        let mut loaded_input = InputState::default();
        let mut loaded_decrypt = DecryptState::default();
        let mut loaded_encrypt = EncryptState::default();
        let mut loaded_outputs = Vec::new();

        load_config_into_states(
            &config,
            &mut loaded_input,
            &mut loaded_decrypt,
            &mut loaded_encrypt,
            &mut loaded_outputs,
        );

        assert_eq!(loaded_input.input_type, InputType::File);
        assert_eq!(loaded_input.file_path, "source.dat");
        assert!(loaded_decrypt.enabled);
        assert_eq!(loaded_decrypt.key, decrypt.key);
        assert!(loaded_encrypt.enabled);
        assert_eq!(loaded_encrypt.key, encrypt.key);
        assert_eq!(loaded_outputs.len(), 2);
        assert_eq!(loaded_outputs[0].output_type, OutputType::File);
        assert_eq!(loaded_outputs[0].file_path, "dest1.dat");
        assert_eq!(loaded_outputs[1].output_type, OutputType::Stdout);

        Ok(())
    }
}
