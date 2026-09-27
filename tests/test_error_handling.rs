//! Comprehensive error handling and failure scenario tests for datapipe,
//! covering both the library API and the CLI binary.

mod common;

use common::{MockHttpServer, TempFileGuard, run_cli_output, run_cli_status};
use datapipe::config::DatapipeConfig;
use datapipe::datapipe_types::{DatapipeError, EncryptionKey, InputReader};
use datapipe::encryption::{StreamDecryptor, StreamEncryptor};
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::http_reader::HttpReader;
use datapipe::http_writer::HttpWriter;
use datapipe::https_reader::HttpsReader;
use datapipe::https_writer::HttpsWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::stdout_writer::StdoutWriter;
use datapipe::tcp_listen_reader::TcpListenReader;
use datapipe::tcp_reader_writer::TcpReaderWriter;
use datapipe::tls_reader_writer::TlsReaderWriter;
use datapipe::udp_reader::UdpReader;
use datapipe::udp_writer::UdpWriter;
use datapipe::utilities::get_unused_port;
use datapipe::writer::Writer;
use std::path::PathBuf;
use std::time::Duration;
use tokio_rustls::rustls::ClientConfig;

// =========================================================================
// 1. Library API Validation & Configuration Failures
// =========================================================================

#[test]
fn test_lib_parameters_builder_missing_reader() {
    let writer = StdoutWriter::new();
    let result = ParametersBuilder::new()
        .writer(Writer::from(writer))
        .build();

    match result {
        Err(DatapipeError::ValidationError(msg)) => {
            assert!(msg.contains("No input source"));
        }
        Err(other) => panic!("Expected ValidationError, got {other:?}"),
        Ok(_) => panic!("Expected build() to fail without reader"),
    }
}

#[tokio::test]
async fn test_lib_parameters_builder_missing_writer() {
    let temp_file = TempFileGuard::with_content("txt", b"test");
    let reader = FileReader::new(&temp_file.path).await.unwrap();

    let result = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .build();

    match result {
        Err(DatapipeError::ValidationError(msg)) => {
            assert!(msg.contains("No output destination"));
        }
        Err(other) => panic!("Expected ValidationError, got {other:?}"),
        Ok(_) => panic!("Expected build() to fail without writer"),
    }
}

#[tokio::test]
async fn test_lib_file_reader_nonexistent_file() {
    let nonexistent = PathBuf::from("/nonexistent/path/to/never_existed_datapipe_file.dat");
    let result = FileReader::new(&nonexistent).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lib_file_writer_invalid_directory() {
    let invalid_path = PathBuf::from("/nonexistent_dir_12345/unwritable_subdir/output.dat");
    let result = FileWriter::new(&invalid_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lib_tcp_connection_refused() {
    let port: u16 = get_unused_port().await.expect("Failed to get port");
    let addr = format!("127.0.0.1:{}", port);

    // Connecting to unbound port must return an IO error
    let result = TcpReaderWriter::new(&addr).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lib_tcp_listen_port_already_in_use() {
    let listener = TcpListenReader::new("127.0.0.1:0").await.unwrap();
    let port = listener.get_listen_port().unwrap().port();

    // Trying to bind the same port again must error
    let addr = format!("127.0.0.1:{}", port);
    let second_bind = TcpListenReader::new(&addr).await;
    assert!(second_bind.is_err());
}

#[tokio::test]
async fn test_lib_tls_connection_refused() {
    let port: u16 = get_unused_port().await.expect("Failed to get port");
    let addr = format!("127.0.0.1:{}", port);
    let client_config = ClientConfig::builder()
        .with_root_certificates(tokio_rustls::rustls::RootCertStore::empty())
        .with_no_client_auth();

    let result = TlsReaderWriter::new(&addr, client_config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lib_udp_multicast_invalid_address() {
    // 127.0.0.1 is unicast loopback, not a valid multicast address
    let result = UdpReader::new_multicast("127.0.0.1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lib_udp_writer_invalid_address() {
    let result = UdpWriter::new("invalid_host_name_definitely_does_not_exist:9999").await;
    assert!(result.is_err());
}

#[test]
fn test_lib_http_reader_invalid_url_scheme() {
    let result = HttpReader::new("ftp://example.com/data", 100);
    assert!(result.is_err());

    let result_https = HttpReader::new("https://example.com/data", 100);
    assert!(result_https.is_err());

    let result_not_url = HttpReader::new("not_a_valid_url", 100);
    assert!(result_not_url.is_err());
}

#[test]
fn test_lib_http_writer_invalid_url_scheme() {
    let result = HttpWriter::new(
        "ftp://example.com/data",
        vec![b'\n'],
        true,
        Duration::from_millis(100),
    );
    assert!(result.is_err());
}

#[test]
fn test_lib_https_reader_invalid_url_scheme() {
    let result = HttpsReader::new(
        "http://example.com/data", // http instead of https
        Duration::from_millis(100),
        None,
        None,
        None,
        false,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_lib_https_writer_invalid_url_scheme() {
    let result = HttpsWriter::new(
        "http://example.com/data", // http instead of https
        vec![b'\n'],
        true,
        Duration::from_millis(100),
        None,
        None,
        None,
        false,
        false,
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lib_http_reader_server_error_responses() {
    // 1. HTTP 404 Not Found
    {
        let mock_server_404 = MockHttpServer::start(b"Not found".to_vec(), 404).await;
        let mut reader = HttpReader::new(&mock_server_404.url(), 10).unwrap();
        let result = reader.read().await;
        assert!(result.is_err(), "Expected error on HTTP 404 response");
    }

    // 2. HTTP 500 Internal Server Error
    {
        let mock_server_500 = MockHttpServer::start(b"Internal Error".to_vec(), 500).await;
        let mut reader = HttpReader::new(&mock_server_500.url(), 10).unwrap();
        let result = reader.read().await;
        assert!(result.is_err(), "Expected error on HTTP 500 response");
    }
}

// =========================================================================
// 2. Encryption Key & Decryption Failure Scenarios
// =========================================================================

#[test]
fn test_lib_encryption_key_invalid_lengths() {
    // Exactly 51 bytes required
    assert!(EncryptionKey::new("").is_err());
    assert!(EncryptionKey::new("short_key").is_err());
    assert!(EncryptionKey::new("12345678901234567890123456789012345678901234567890").is_err()); // 50 chars
    assert!(EncryptionKey::new("1234567890123456789012345678901234567890123456789012").is_err()); // 52 chars

    // 51 characters must succeed
    assert!(EncryptionKey::new("123456789012345678901234567890123456789012345678901").is_ok());
}

#[test]
fn test_lib_decryption_corrupted_data() {
    let key = EncryptionKey::generate();
    let mut decryptor = StreamDecryptor::new(key.clone()).unwrap();

    // 1. Corrupted magic header
    let mut corrupted_magic = vec![0x00, 0x00, 0x00, 0x00, 0x05, 0x01, 0x02, 0x03, 0x04, 0x05];
    let result = decryptor.decrypt(&mut corrupted_magic);
    assert!(result.is_err());

    // 2. Valid ciphertext with corrupted payload
    let mut encryptor = StreamEncryptor::new(key).unwrap();
    let mut plaintext = b"Sensitive information payload".to_vec();
    let mut ciphertext = encryptor.encrypt(&mut plaintext).unwrap();

    // Corrupt one byte in the ciphertext body
    let last_idx = ciphertext.len() - 1;
    ciphertext[last_idx] ^= 0xFF;

    let mut decryptor2 = StreamDecryptor::new(EncryptionKey::generate()).unwrap();
    let dec_result = decryptor2.decrypt(&mut ciphertext);
    assert!(
        dec_result.is_err(),
        "Decryption must fail with wrong key or tampered ciphertext"
    );
}

// =========================================================================
// 3. Configuration File Validation Failures
// =========================================================================

#[test]
fn test_config_validation_failures() {
    // 1. No input configured
    let mut config = DatapipeConfig::default();
    config.output.file_output = Some(PathBuf::from("/tmp/out.dat"));
    assert!(config.validate().is_err());

    // 2. Multiple inputs configured
    let mut config_multi = DatapipeConfig::default();
    config_multi.input.file_input = Some(PathBuf::from("/tmp/in.dat"));
    config_multi.input.stdin_input = true;
    config_multi.output.file_output = Some(PathBuf::from("/tmp/out.dat"));
    assert!(config_multi.validate().is_err());

    // 3. No output configured
    let mut config_no_out = DatapipeConfig::default();
    config_no_out.input.file_input = Some(PathBuf::from("/tmp/in.dat"));
    assert!(config_no_out.validate().is_err());

    // 4. Invalid encryption key
    let mut config_bad_key = DatapipeConfig::default();
    config_bad_key.input.file_input = Some(PathBuf::from("/tmp/in.dat"));
    config_bad_key.output.file_output = Some(PathBuf::from("/tmp/out.dat"));
    config_bad_key.encryption_args.encryption_key = Some("short_key".to_string());
    assert!(config_bad_key.validate().is_err());

    // 5. Invalid HTTP URL in config
    let mut config_bad_url = DatapipeConfig::default();
    config_bad_url.input.http_input = Some("bad_url".to_string());
    config_bad_url.output.file_output = Some(PathBuf::from("/tmp/out.dat"));
    assert!(config_bad_url.validate().is_err());
}

#[test]
fn test_config_load_nonexistent_or_malformed() {
    let nonexistent = PathBuf::from("/nonexistent/datapipe_config.toml");
    assert!(DatapipeConfig::load_from_file(&nonexistent).is_err());

    let malformed_guard = TempFileGuard::with_content("toml", b"[[invalid toml syntax [[[");
    assert!(DatapipeConfig::load_from_file(&malformed_guard.path).is_err());
}

// =========================================================================
// 4. CLI Binary Failure & Error Scenarios
// =========================================================================

#[tokio::test]
async fn test_cli_no_arguments() {
    let output = run_cli_output(&[]).await;
    assert!(!output.status.success());
}

#[tokio::test]
async fn test_cli_missing_output() {
    let test_doc = common::get_test_document();
    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_missing_input() {
    let temp_out = TempFileGuard::new("dat");
    let args = vec![
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_multiple_inputs_conflict() {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("dat");

    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--stdin-input".to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_nonexistent_input_file() {
    let temp_out = TempFileGuard::new("dat");
    let args = vec![
        "--file-input".to_string(),
        "/nonexistent/datapipe/input_file.dat".to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_tcp_connection_refused() {
    let port: u16 = get_unused_port().await.expect("Failed to get port");
    let temp_out = TempFileGuard::new("dat");

    let args = vec![
        "--tcp-input".to_string(),
        format!("127.0.0.1:{}", port),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_http_input_missing_rate() {
    let temp_out = TempFileGuard::new("dat");
    let args = vec![
        "--http-input".to_string(),
        "http://localhost:8080".to_string(),
        // missing --http-input-rate
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_https_input_missing_rate() {
    let temp_out = TempFileGuard::new("dat");
    let args = vec![
        "--https-input".to_string(),
        "https://localhost:8080".to_string(),
        // missing --https-input-rate
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_invalid_encryption_key_length() {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("dat");

    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--encrypt".to_string(),
        "invalid_short_key".to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_invalid_decryption_key_length() {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("dat");

    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--decrypt".to_string(),
        "invalid_short_key".to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_use_config_cli_io_conflict() {
    let valid_toml = r#"
[input]
file_input = "/tmp/in.dat"
[output]
file_output = "/tmp/out.dat"
"#;
    let cfg = TempFileGuard::with_content("toml", valid_toml.as_bytes());

    let args = vec![
        "--use-config".to_string(),
        cfg.path.to_str().unwrap().to_string(),
        "--file-input".to_string(),
        "/tmp/conflict.dat".to_string(),
    ];
    let status = run_cli_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn test_cli_verify_config_failures() {
    // 1. Non-existent file
    let args_nonexistent = vec![
        "--verify-config".to_string(),
        "/nonexistent/config_path.toml".to_string(),
    ];
    assert!(!run_cli_status(&args_nonexistent).await.success());

    // 2. Malformed TOML syntax
    let malformed = TempFileGuard::with_content("toml", b"invalid [[ [ toml");
    let args_malformed = vec![
        "--verify-config".to_string(),
        malformed.path.to_str().unwrap().to_string(),
    ];
    assert!(!run_cli_status(&args_malformed).await.success());

    // 3. Valid TOML syntax but invalid configuration (no output)
    let invalid_config =
        TempFileGuard::with_content("toml", b"[input]\nfile_input = '/tmp/in.dat'\n");
    let args_invalid = vec![
        "--verify-config".to_string(),
        invalid_config.path.to_str().unwrap().to_string(),
    ];
    assert!(!run_cli_status(&args_invalid).await.success());
}

#[tokio::test]
async fn test_cli_save_to_config_missing_io() {
    let temp_cfg = TempFileGuard::new("toml");
    let args = vec![
        "--save-to-config".to_string(),
        temp_cfg.path.to_str().unwrap().to_string(),
    ];
    assert!(!run_cli_status(&args).await.success());
}
