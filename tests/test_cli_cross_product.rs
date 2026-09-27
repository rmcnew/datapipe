//! Comprehensive tests exercising the datapipe CLI binary across all reader and writer types
//! in cross-product combinations.

mod common;

use common::{
    MockHttpServer, MockHttpsServer, TempFileGuard, get_test_document, identical_contents,
    run_cli_output, run_cli_status, run_cli_with_stdin,
};
use datapipe::datapipe_types::EncryptionKey;
use datapipe::utilities::get_unused_port;
use std::time::Duration;

#[tokio::test]
async fn test_cli_file_to_file() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let status = run_cli_status(&args).await;
    assert!(status.success());
    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_cli_file_to_tcp_to_file() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let port = get_unused_port().await.expect("Failed to get port");
    let tcp_addr = format!("127.0.0.1:{}", port);

    let listen_args = vec![
        "--tcp-listen-input".to_string(),
        tcp_addr.clone(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let send_args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--tcp-output".to_string(),
        tcp_addr.clone(),
    ];

    let receiver = tokio::spawn(async move { run_cli_status(&listen_args).await });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let sender = tokio::spawn(async move { run_cli_status(&send_args).await });

    let send_status = sender.await.unwrap();
    let recv_status = receiver.await.unwrap();

    assert!(send_status.success());
    assert!(recv_status.success());
    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_cli_file_to_tls_to_file() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let port = get_unused_port().await.expect("Failed to get port");
    let tls_addr = format!("localhost:{}", port);

    let listen_args = vec![
        "--tls-listen-input".to_string(),
        tls_addr.clone(),
        "--tls-listen-input-generate-self-signed".to_string(),
        "--tls-listen-input-skip-client-verify".to_string(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let send_args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--tls-output".to_string(),
        tls_addr.clone(),
        "--tls-output-skip-server-verify".to_string(),
    ];

    let receiver = tokio::spawn(async move { run_cli_status(&listen_args).await });

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let sender = tokio::spawn(async move { run_cli_status(&send_args).await });

    let send_status = sender.await.unwrap();
    let recv_status = receiver.await.unwrap();

    assert!(send_status.success());
    assert!(recv_status.success());
    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_cli_file_to_udp_to_file() {
    let payload = b"CLI UDP packet stream payload verification";
    let input_file = TempFileGuard::with_content("dat", payload);
    let output_file = TempFileGuard::new("dat");

    let port = get_unused_port().await.expect("Failed to get port");
    let udp_addr = format!("127.0.0.1:{}", port);

    let listen_args = vec![
        "--udp-input".to_string(),
        udp_addr.clone(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let send_args = vec![
        "--file-input".to_string(),
        input_file.path.to_str().unwrap().to_string(),
        "--udp-output".to_string(),
        udp_addr.clone(),
    ];

    let receiver = tokio::spawn(async move {
        let _ =
            tokio::time::timeout(Duration::from_millis(1500), run_cli_status(&listen_args)).await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let sender = tokio::spawn(async move { run_cli_status(&send_args).await });

    let send_status = sender.await.unwrap();
    assert!(send_status.success());
    let _ = receiver.await;

    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert_eq!(written, payload);
}

#[tokio::test]
async fn test_cli_stdin_to_file() {
    let input_bytes = b"Streaming bytes directly from stdin into a destination file!";
    let output_file = TempFileGuard::new("dat");

    let args = vec![
        "--stdin-input".to_string(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let output = run_cli_with_stdin(&args, input_bytes).await;
    assert!(output.status.success());
    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert_eq!(written, input_bytes);
}

#[tokio::test]
async fn test_cli_file_to_stdout() {
    let payload = b"Streaming bytes from a file directly to stdout!";
    let input_file = TempFileGuard::with_content("dat", payload);

    let args = vec![
        "--file-input".to_string(),
        input_file.path.to_str().unwrap().to_string(),
        "--stdout-output".to_string(),
    ];

    let output = run_cli_output(&args).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, payload);
}

#[tokio::test]
async fn test_cli_stdin_to_stdout() {
    let payload = b"Pure pipeline pipe: stdin straight to stdout!";

    let args = vec!["--stdin-input".to_string(), "--stdout-output".to_string()];

    let output = run_cli_with_stdin(&args, payload).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, payload);
}

#[tokio::test]
async fn test_cli_http_input_to_file() {
    let expected = b"Mock server HTTP body via CLI --http-input";
    let mock_server = MockHttpServer::start(expected.to_vec(), 200).await;
    let output_file = TempFileGuard::new("txt");

    let args = vec![
        "--http-input".to_string(),
        mock_server.url(),
        "--http-input-rate".to_string(),
        "10".to_string(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    // Run CLI with timeout because HTTP polling keeps running
    let _ = tokio::time::timeout(Duration::from_millis(600), run_cli_status(&args)).await;

    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert!(written.starts_with(expected));
}

#[tokio::test]
async fn test_cli_file_to_http_output() {
    let line_data = b"HTTP PUT test payload from CLI\n";
    let input_file = TempFileGuard::with_content("txt", line_data);
    let mock_server = MockHttpServer::start(Vec::new(), 200).await;

    let args = vec![
        "--file-input".to_string(),
        input_file.path.to_str().unwrap().to_string(),
        "--http-output".to_string(),
        mock_server.url(),
        "--http-output-rate".to_string(),
        "10".to_string(),
    ];

    let status = run_cli_status(&args).await;
    assert!(status.success());

    let received = mock_server.received_payloads.lock().await;
    assert!(!received.is_empty());
    assert_eq!(received[0], line_data);
}

#[tokio::test]
async fn test_cli_https_input_to_file() {
    let expected = b"Mock server HTTPS body via CLI --https-input";
    let mock_server = MockHttpsServer::start(expected.to_vec(), 200).await;
    let output_file = TempFileGuard::new("txt");

    let args = vec![
        "--https-input".to_string(),
        mock_server.url(),
        "--https-input-rate".to_string(),
        "10".to_string(),
        "--https-input-allow-invalid-certificates".to_string(),
        "--https-input-allow-invalid-hostnames".to_string(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let _ = tokio::time::timeout(Duration::from_millis(600), run_cli_status(&args)).await;

    assert!(output_file.path.exists());
    let written = tokio::fs::read(&output_file.path).await.unwrap();
    assert!(written.starts_with(expected));
}

#[tokio::test]
async fn test_cli_file_to_https_output() {
    let line_data = b"HTTPS PUT test payload from CLI\n";
    let input_file = TempFileGuard::with_content("txt", line_data);
    let mock_server = MockHttpsServer::start(Vec::new(), 200).await;

    let args = vec![
        "--file-input".to_string(),
        input_file.path.to_str().unwrap().to_string(),
        "--https-output".to_string(),
        mock_server.url(),
        "--https-output-rate".to_string(),
        "10".to_string(),
        "--https-output-allow-invalid-certificates".to_string(),
        "--https-output-allow-invalid-hostnames".to_string(),
    ];

    let status = run_cli_status(&args).await;
    assert!(status.success());

    let received = mock_server.received_payloads.lock().await;
    assert!(!received.is_empty());
    assert_eq!(received[0], line_data);
}

#[tokio::test]
async fn test_cli_multi_output_fan_out() {
    let payload = b"Testing CLI multi-output fan-out capability!";
    let input_file = TempFileGuard::with_content("dat", payload);
    let out_file = TempFileGuard::new("dat");

    let args = vec![
        "--file-input".to_string(),
        input_file.path.to_str().unwrap().to_string(),
        "--file-output".to_string(),
        out_file.path.to_str().unwrap().to_string(),
        "--stdout-output".to_string(),
    ];

    let output = run_cli_output(&args).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, payload);
    assert_eq!(tokio::fs::read(&out_file.path).await.unwrap(), payload);
}

#[tokio::test]
async fn test_cli_inline_encryption_pipeline() {
    let test_doc = get_test_document();
    let encrypted_file = TempFileGuard::new("enc");
    let decrypted_file = TempFileGuard::new("pdf");

    let key = EncryptionKey::generate();
    let key_str = key.to_string();

    // 1. Encrypt: File -> --encrypt -> File
    let enc_args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--encrypt".to_string(),
        key_str.clone(),
        "--file-output".to_string(),
        encrypted_file.path.to_str().unwrap().to_string(),
    ];

    let enc_status = run_cli_status(&enc_args).await;
    assert!(enc_status.success());
    assert!(encrypted_file.path.exists());
    assert!(
        !identical_contents(&test_doc, &encrypted_file.path)
            .await
            .unwrap()
    );

    // 2. Decrypt: File -> --decrypt -> File
    let dec_args = vec![
        "--file-input".to_string(),
        encrypted_file.path.to_str().unwrap().to_string(),
        "--decrypt".to_string(),
        key_str,
        "--file-output".to_string(),
        decrypted_file.path.to_str().unwrap().to_string(),
    ];

    let dec_status = run_cli_status(&dec_args).await;
    assert!(dec_status.success());
    assert!(decrypted_file.path.exists());
    assert!(
        identical_contents(&test_doc, &decrypted_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_cli_tcp_encrypted_stream() {
    let test_doc = get_test_document();
    let output_file = TempFileGuard::new("pdf");

    let key = EncryptionKey::generate();
    let key_str = key.to_string();
    let port = get_unused_port().await.expect("Failed to get port");
    let tcp_addr = format!("127.0.0.1:{}", port);

    let listen_args = vec![
        "--tcp-listen-input".to_string(),
        tcp_addr.clone(),
        "--decrypt".to_string(),
        key_str.clone(),
        "--file-output".to_string(),
        output_file.path.to_str().unwrap().to_string(),
    ];

    let send_args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--encrypt".to_string(),
        key_str,
        "--tcp-output".to_string(),
        tcp_addr.clone(),
    ];

    let receiver = tokio::spawn(async move { run_cli_status(&listen_args).await });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let sender = tokio::spawn(async move { run_cli_status(&send_args).await });

    let send_status = sender.await.unwrap();
    let recv_status = receiver.await.unwrap();

    assert!(send_status.success());
    assert!(recv_status.success());
    assert!(output_file.path.exists());
    assert!(
        identical_contents(&test_doc, &output_file.path)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_cli_encrypt_generate_key() {
    let test_doc = get_test_document();
    let temp_out = TempFileGuard::new("enc");
    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--encrypt-generate-key".to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];
    let output = run_cli_output(&args).await;

    assert!(output.status.success());
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(stdout_str.contains("Generated encryption key:"));
    let key_part = stdout_str
        .lines()
        .find(|l| l.contains("Generated encryption key:"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim())
        .expect("Failed to parse generated key from stdout");
    assert_eq!(key_part.len(), 51);
    assert!(EncryptionKey::new(key_part).is_ok());
    assert!(temp_out.path.exists());
}
