use clap::{Args, Parser};
use std::path::PathBuf;
use crate::engine::Parameters;

/// Choose one input source
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct InputArgs {
    /// --stdin-input  read input from STDIN
    #[arg(long = "stdin-input")]
    pub stdin_input: bool,
    /// --file-input  read input from file
    #[arg(long = "file-input")]
    pub file_input: Option<PathBuf>,
    /// --udp-input  read input from UDP address
    #[arg(long = "udp-input")]
    pub udp_input: Option<String>,
    /// --tcp-input  read input from TCP address
    #[arg(long = "tcp-input")]
    pub tcp_input: Option<String>,
    /// --tcp-listen-input  open a local port to receive a TCP connection
    #[arg(long = "tcp-listen-input")]
    pub tcp_listen_input: Option<String>,
    /// --tls-input  read input from TLS address
    #[arg(long = "tls-input")]
    pub tls_input: Option<String>,
    /// --http-input  read input from HTTP URL
    #[arg(long = "http-input")]
    pub http_input: Option<String>,
    /// --https-input  read input from HTTPS URL
    #[arg(long = "https-input")]
    pub https_input: Option<String>,
}

/// Additional parameters for HTTP input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpInputArgs {
    /// --http-input-rate  in milliseconds, how often should the input web address be polled?
    #[arg(long = "http-input-rate")]
    pub http_input_rate: Option<u64>,    
}

/// Additional parameters for decrypting input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct DecryptionInputArgs {
    /// --decrypt  decryption key to use
    #[arg(long = "decrypt")]
    pub decryption_key: Option<String>,
}

/// Choose one or more output destinations
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = true)]
pub struct OutputArgs {
    /// --stdout-output  write output to STDOUT
    #[arg(long = "stdout-output")]
    pub stdout_output: bool,
    /// --file-output  write output to file
    #[arg(long = "file-output")]
    pub file_output: Option<Vec<PathBuf>>,
    /// --udp-output  write output to UDP address
    #[arg(long = "udp-output")]
    pub udp_output: Option<Vec<String>>,
    /// --tcp-output  write output to TCP address
    #[arg(long = "tcp-output")]
    pub tcp_output: Option<Vec<String>>,
    /// --tls-output  write output to TCP/TLS URL
    #[arg(long = "tls-output")]
    pub tls_output: Option<Vec<String>>,
    /// --http-output  write output to HTTP URL
    #[arg(long = "http-output")]
    pub http_output: Option<String>,
    /// --https-output  write output to HTTPS URL
    #[arg(long = "https-output")]
    pub https_output: Option<String>,
}

/// Additional parameters needed for HTTP output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpOutputArgs {
    /// --http-output-rate  in milliseconds, how often should data be sent to the output web address?
    #[arg(long = "http-output-rate")]
    pub http_output_rate: Option<u64>,    
    /// --http-output-delimiter  this delimiter is used to group the output into one or more 'segments' that will be sent in each request
    #[arg(long = "http-output-delimiter")]
    pub http_output_delimiter: Option<Vec<u8>>,
    /// --http-output-include-delimiter  should the delimiter be included with the segment that preceeds it?
    #[arg(long = "http-output-include-delimiter")]
    pub http_output_include_delimiter: Option<bool>,
}

/// Additional parameters needed for TLS output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct TlsOutputArgs {
    /// --tls-cert-chain  TLS certificate chain file to use
    #[arg(long = "tls-cert-chain")]
    pub tls_cert_chain: Option<PathBuf>,
    /// --tls-client-key  TLS client certificate to use
    #[arg(long = "tls-client-key")]
    pub tls_client_key: Option<PathBuf>,
    /// --tls-root-ca  use these CAs instead of web root CAs
    #[arg(long = "tls-root-ca")]
    pub tls_root_ca: Option<PathBuf>,
    /// --tls-skip-server-verify  do not verify the server identity; for testing or closed, self-signed networks
    #[arg(long = "tls-skip-server-verify")]
    pub tls_skip_server_verify: Option<bool>, 
}

/// Additional parameters for encrypting output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct EncryptionOutputArgs {
    /// --encrypt  encryption key to use
    #[arg(long = "encrypt")]
    pub encryption_key: Option<String>,
}

/// Overall command line args
#[derive(Parser, Debug, Clone)]
pub struct ProgramArgs {
    #[command(flatten)]
    pub input: InputArgs,
    #[command(flatten)]
    pub http_input: HttpInputArgs,
    #[command(flatten)]
    pub output: OutputArgs,
    #[command(flatten)]
    pub http_output: HttpOutputArgs,
    #[command(flatten)]
    pub tls_output: TlsOutputArgs,
}

impl ProgramArgs {

    pub async fn to_parameters(&self) -> Result<Parameters, std::io::Error> {
        /*
        // parse input UDP address if present
        if args.input.udp_input.is_some()
            && !good_tcp_udp_address(args.input.udp_input.clone().unwrap().as_str())
        {
            eprintln!(
                "Invalid UDP input address: {}",
                args.input.udp_input.unwrap()
            );
            return ExitCode::FAILURE;
        }
        // parse HTTP input URL if present
        if args.input.http_input.is_some() && !good_url(args.input.http_input.clone().unwrap().as_str())
        {
            eprintln!("Invalid HTTP input URL: {}", args.input.http_input.unwrap());
            return ExitCode::FAILURE;
        }
        // parse output UDP address if present
        if args.output.udp_output.is_some() {
            let addresses = args.output.udp_output.clone().unwrap();
            for address in addresses {
                if !good_tcp_udp_address(&address) {
                    eprintln!("Invalid UDP output address: {}", address);
                    return ExitCode::FAILURE;
                }
            }
        }
        // parse HTTP output URL if present
        if args.output.http_output.is_some()
            && !good_url(args.output.http_output.clone().unwrap().as_str())
        {
            eprintln!(
                "Invalid HTTP output URL: {}",
                args.output.http_output.unwrap()
            );
            return ExitCode::FAILURE;
        }
        // parse TLS (TCP) address if present
        if args.output.tls_output.is_some() {
            let addresses = args.output.tls_output.clone().unwrap();
            for address in addresses {
                if !good_tcp_udp_address(&address) {
                    eprintln!("Invalid TLS output address: {}", address);
                    return ExitCode::FAILURE;
                }
            }
        }
        */

        Ok(Parameters::default())
    }
}

