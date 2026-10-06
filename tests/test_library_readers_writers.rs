//! Comprehensive tests exercising the datapipe library API (`ParametersBuilder`,
//! `run_data_pipe`, and reader/writer component traits) across all reader and writer types
//! in cross-product combinations.

mod common;

use common::{
    MockHttpServer, MockHttpsServer, TempFileGuard, get_test_document, identical_contents,
    init_crypto,
};
use datapipe::datapipe_types::{EncryptionKey, InputReader, OutputWriter};
use datapipe::encryption::{StreamDecryptor, StreamEncryptor};
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::http_reader::HttpReader;
use datapipe::http_writer::HttpWriter;
use datapipe::https_reader::HttpsReader;
use datapipe::https_writer::HttpsWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::stdin_reader::StdinReader;
use datapipe::stdout_writer::StdoutWriter;
use datapipe::tcp_listen_reader::TcpListenReader;
use datapipe::tcp_reader_writer::TcpReaderWriter;
use datapipe::tls_listen_reader::TlsListenReader;
use datapipe::tls_reader_writer::TlsReaderWriter;
use datapipe::udp_reader::UdpReader;
use datapipe::udp_writer::UdpWriter;
use datapipe::utilities::get_unused_port;
use datapipe::writer::Writer;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::{ClientConfig, DigitallySignedStruct, ServerConfig, SignatureScheme};

/// Helper: custom no-op TLS certificate verifier for library test connections
#[derive(Debug)]
struct TestNoCertVerifier;

impl ServerCertVerifier for TestNoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ED25519,
        ]
    }
}

fn create_test_tls_client_config() -> ClientConfig {
    init_crypto();
    ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(TestNoCertVerifier))
        .with_no_client_auth()
}

fn create_test_tls_server_config() -> ServerConfig {
    init_crypto();
    let cert_key =
        generate_simple_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .expect("Failed to generate self-signed cert");
    let cert_der = cert_key.cert.der().clone();
    let key_der = PrivateKeyDer::from(cert_key.signing_key);

    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("Failed to create server config")
}

// =========================================================================
// 1. Core Cross-Product Tests (Library API)
// =========================================================================

#[tokio::test]
async fn test_lib_file_to_file() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let reader = FileReader::new(&test_doc).await.unwrap();
    let writer = FileWriter::new(&output_file.path).await.unwrap();

    let parameters = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()
        .unwrap();

    run_datapipe(parameters).await.unwrap();

    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_lib_file_to_tcp_to_file() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let tcp_listener = TcpListenReader::new("127.0.0.1:0").await.unwrap();
    let listen_addr = tcp_listener.get_listen_port().unwrap();

    // Receiving pipeline: TcpListen -> FileWriter
    let output_path = output_file.path.clone();
    let receiver = tokio::spawn(async move {
        let writer = FileWriter::new(&output_path).await.unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(tcp_listener))
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Sending pipeline: FileReader -> TcpReaderWriter
    let test_doc_path = test_doc.clone();
    let sender = tokio::spawn(async move {
        let reader = FileReader::new(&test_doc_path).await.unwrap();
        let writer = TcpReaderWriter::new(&listen_addr.to_string())
            .await
            .unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    sender.await.unwrap();
    receiver.await.unwrap();

    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_lib_file_to_tls_to_file() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let server_config = create_test_tls_server_config();
    let tls_listener = TlsListenReader::new("127.0.0.1:0", server_config)
        .await
        .unwrap();
    let listen_addr = tls_listener.get_listen_port().unwrap();

    // Receiver: TlsListenReader -> FileWriter
    let out_path = output_file.path.clone();
    let receiver = tokio::spawn(async move {
        let writer = FileWriter::new(&out_path).await.unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(tls_listener))
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Sender: FileReader -> TlsReaderWriter
    let test_doc_path = test_doc.clone();
    let sender = tokio::spawn(async move {
        let reader = FileReader::new(&test_doc_path).await.unwrap();
        let client_config = create_test_tls_client_config();
        let writer = TlsReaderWriter::new(&listen_addr.to_string(), client_config)
            .await
            .unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    sender.await.unwrap();
    receiver.await.unwrap();

    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_lib_file_to_udp_to_file() {
    let payload = b"Hello UDP stream from datapipe library!";
    let input_file = TempFileGuard::with_content("dat", payload);
    let output_file = TempFileGuard::new("dat");

    let port = get_unused_port().await.expect("Failed to get unused port");
    let udp_addr = format!("127.0.0.1:{}", port);

    // Receiver: UdpReader -> FileWriter
    let receiver_addr = udp_addr.clone();
    let out_path = output_file.path.clone();
    let receiver = tokio::spawn(async move {
        let reader = UdpReader::new(&receiver_addr).await.unwrap();
        let writer = FileWriter::new(&out_path).await.unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .writer(Writer::from(writer))
            .build()
            .unwrap();

        // Run receiver with timeout since UDP waits indefinitely
        let _ = tokio::time::timeout(Duration::from_millis(1500), run_datapipe(params)).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Sender: FileReader -> UdpWriter
    let in_path = input_file.path.clone();
    let target_addr = udp_addr.clone();
    let sender = tokio::spawn(async move {
        let reader = FileReader::new(&in_path).await.unwrap();
        let writer = UdpWriter::new(&target_addr).await.unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    sender.await.unwrap();
    let _ = receiver.await;

    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert_eq!(written, payload);
}

#[tokio::test]
async fn test_lib_http_to_file() {
    let expected_data = b"HTTP response body content from mock server";
    let mock_server = MockHttpServer::start(expected_data.to_vec(), 200).await;
    let output_file = TempFileGuard::new("txt");

    let reader = HttpReader::new(&mock_server.url(), 10).unwrap();
    let writer = FileWriter::new(&output_file.path).await.unwrap();

    let params = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()
        .unwrap();

    // Run pipeline with a timeout since HTTP reader periodically polls
    let _ = tokio::time::timeout(Duration::from_millis(500), run_datapipe(params)).await;

    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert!(written.starts_with(expected_data));
}

#[tokio::test]
async fn test_lib_file_to_http() {
    let line_data = b"Line 1 for HTTP PUT upload\n";
    let input_file = TempFileGuard::with_content("txt", line_data);
    let mock_server = MockHttpServer::start(Vec::new(), 200).await;

    let reader = FileReader::new(&input_file.path).await.unwrap();
    let writer = HttpWriter::new(
        &mock_server.url(),
        vec![b'\n'],
        true,
        Duration::from_millis(10),
    )
    .unwrap();

    let params = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()
        .unwrap();

    run_datapipe(params).await.unwrap();

    // Verify mock server received the payload
    let received = mock_server.received_payloads.lock().await;
    assert!(!received.is_empty());
    assert_eq!(received[0], line_data);
}

#[tokio::test]
async fn test_lib_https_to_file() {
    let expected_data = b"Secure HTTPS response body content";
    let mock_server = MockHttpsServer::start(expected_data.to_vec(), 200).await;
    let output_file = TempFileGuard::new("txt");

    let reader = HttpsReader::new(
        &mock_server.url(),
        Duration::from_millis(10),
        None,
        None,
        None,
        true, // allow invalid hostnames
        true, // allow invalid certs
    )
    .unwrap();

    let writer = FileWriter::new(&output_file.path).await.unwrap();
    let params = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_millis(500), run_datapipe(params)).await;

    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert!(written.starts_with(expected_data));
}

#[tokio::test]
async fn test_lib_file_to_https() {
    let line_data = b"Encrypted HTTPS line payload\n";
    let input_file = TempFileGuard::with_content("txt", line_data);
    let mock_server = MockHttpsServer::start(Vec::new(), 200).await;

    let reader = FileReader::new(&input_file.path).await.unwrap();
    let writer = HttpsWriter::new(
        &mock_server.url(),
        vec![b'\n'],
        true,
        Duration::from_millis(10),
        None,
        None,
        None,
        true,
        true,
    )
    .unwrap();

    let params = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()
        .unwrap();

    run_datapipe(params).await.unwrap();

    let received = mock_server.received_payloads.lock().await;
    assert!(!received.is_empty());
    assert_eq!(received[0], line_data);
}

#[tokio::test]
async fn test_lib_multi_output_fan_out() {
    let payload = b"Broadcast this line to multiple destinations\n";
    let input_file = TempFileGuard::with_content("txt", payload);
    let out_file_1 = TempFileGuard::new("txt");
    let out_file_2 = TempFileGuard::new("txt");

    let port = get_unused_port().await.unwrap();
    let udp_addr = format!("127.0.0.1:{}", port);

    let reader = FileReader::new(&input_file.path).await.unwrap();
    let writer_1 = FileWriter::new(&out_file_1.path).await.unwrap();
    let writer_2 = FileWriter::new(&out_file_2.path).await.unwrap();
    let writer_udp = UdpWriter::new(&udp_addr).await.unwrap();
    let writer_stdout = StdoutWriter::new();

    let params = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer_1))
        .writer(Writer::from(writer_2))
        .writer(Writer::from(writer_udp))
        .writer(Writer::from(writer_stdout))
        .build()
        .unwrap();

    run_datapipe(params).await.unwrap();

    // Both files must have received the identical payload
    assert_eq!(tokio::fs::read(&out_file_1.path).await.unwrap(), payload);
    assert_eq!(tokio::fs::read(&out_file_2.path).await.unwrap(), payload);
}

// =========================================================================
// 2. Inline Encryption & Decryption Combinations
// =========================================================================

#[tokio::test]
async fn test_lib_file_encryption_decryption_pipeline() {
    let test_doc = get_test_document();
    let encrypted_file = TempFileGuard::new("enc");
    let decrypted_file = TempFileGuard::new("pdf");

    let key = EncryptionKey::generate();

    // Stage 1: File -> Encryptor -> File
    {
        let reader = FileReader::new(&test_doc).await.unwrap();
        let encryptor = StreamEncryptor::new(key.clone()).unwrap();
        let writer = FileWriter::new(&encrypted_file.path).await.unwrap();

        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .encryptor(encryptor)
            .writer(Writer::from(writer))
            .build()
            .unwrap();

        run_datapipe(params).await.unwrap();
    }

    // Encrypted file should exist and NOT be identical to the original plaintext
    assert!(encrypted_file.path.exists());
    assert!(
        !identical_contents(&test_doc, &encrypted_file.path)
            .await
            .unwrap()
    );

    // Stage 2: Encrypted File -> Decryptor -> File
    {
        let reader = FileReader::new(&encrypted_file.path).await.unwrap();
        let decryptor = StreamDecryptor::new(key).unwrap();
        let writer = FileWriter::new(&decrypted_file.path).await.unwrap();

        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .decryptor(decryptor)
            .writer(Writer::from(writer))
            .build()
            .unwrap();

        run_datapipe(params).await.unwrap();
    }

    assert!(decrypted_file.path.exists());
    assert!(
        identical_contents(&test_doc, &decrypted_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_lib_tcp_stream_encryption() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let key = EncryptionKey::generate();
    let tcp_listener = TcpListenReader::new("127.0.0.1:0").await.unwrap();
    let listen_addr = tcp_listener.get_listen_port().unwrap();

    // Receiver: TcpListenReader -> Decryptor -> FileWriter
    let out_path = output_file.path.clone();
    let dec_key = key.clone();
    let receiver = tokio::spawn(async move {
        let decryptor = StreamDecryptor::new(dec_key).unwrap();
        let writer = FileWriter::new(&out_path).await.unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(tcp_listener))
            .decryptor(decryptor)
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Sender: FileReader -> Encryptor -> TcpReaderWriter
    let test_doc_path = test_doc.clone();
    let sender = tokio::spawn(async move {
        let reader = FileReader::new(&test_doc_path).await.unwrap();
        let encryptor = StreamEncryptor::new(key).unwrap();
        let writer = TcpReaderWriter::new(&listen_addr.to_string())
            .await
            .unwrap();
        let params = ParametersBuilder::new()
            .reader(Reader::from(reader))
            .encryptor(encryptor)
            .writer(Writer::from(writer))
            .build()
            .unwrap();
        run_datapipe(params).await.unwrap();
    });

    sender.await.unwrap();
    receiver.await.unwrap();

    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

// =========================================================================
// 3. Direct Trait Implementation Tests
// =========================================================================

#[tokio::test]
async fn test_lib_direct_reader_writer_traits() {
    let data = b"Direct trait write test content";
    let temp_file = TempFileGuard::new("dat");

    // Test OutputWriter on FileWriter
    let mut file_writer = FileWriter::new(&temp_file.path).await.unwrap();
    file_writer.write(data).await.unwrap();

    // Test InputReader on FileReader
    let mut file_reader = FileReader::new(&temp_file.path).await.unwrap();
    let read_bytes = file_reader.read().await.unwrap();
    assert_eq!(&read_bytes[..], data);

    // Test OutputWriter on StdoutWriter
    let mut stdout_writer = StdoutWriter::new();
    stdout_writer
        .write(b"Stdout writer trait test\n")
        .await
        .unwrap();

    // Test StdinReader instantiation
    let _stdin_reader = StdinReader::new();
}
