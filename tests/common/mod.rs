//! Shared test utilities for datapipe integration tests.
//!
//! Provides mock HTTP and HTTPS servers, temporary file management,
//! CLI subprocess execution helpers, and network port utilities.

#![allow(dead_code)]

use datapipe::datapipe_types::{DatapipeError, generate_random_string};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::{PemObject, SectionKind};
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;

/// Initialize rustls crypto provider globally (safe to call multiple times)
pub fn init_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Returns the path to the compiled datapipe binary
pub fn get_datapipe_binary() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir).join("target/debug/datapipe")
}

/// Returns the path to the static test PDF document
pub fn get_test_document() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir).join("tests/data/document.pdf")
}

/// Returns the system temporary directory
pub fn get_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// RAII temporary file that deletes itself upon drop
#[derive(Debug)]
pub struct TempFileGuard {
    pub path: PathBuf,
}

impl TempFileGuard {
    pub fn new(extension: &str) -> Self {
        let filename = format!("test_dp_{}.{}", generate_random_string(16), extension);
        let path = get_temp_dir().join(filename);
        Self { path }
    }

    pub fn with_content(extension: &str, content: &[u8]) -> Self {
        let guard = Self::new(extension);
        let mut file = File::create(&guard.path).expect("Failed to create temp file");
        file.write_all(content)
            .expect("Failed to write to temp file");
        guard
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// RAII guard for a spawned child process that guarantees the process is killed on drop.
#[derive(Debug)]
pub struct ChildProcessGuard {
    pub child: tokio::process::Child,
}

impl ChildProcessGuard {
    pub fn new(child: tokio::process::Child) -> Self {
        Self { child }
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }
}

impl Drop for ChildProcessGuard {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Spawns a datapipe CLI child process with `kill_on_drop(true)` and wrapped in `ChildProcessGuard`.
pub fn spawn_cli_child(args: &[String]) -> ChildProcessGuard {
    let binary = get_datapipe_binary();
    let mut cmd = Command::new(binary);
    cmd.args(args);
    cmd.kill_on_drop(true);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let child = cmd.spawn().expect("Failed to spawn datapipe child");
    ChildProcessGuard::new(child)
}

/// Run datapipe binary and return its exit status
pub async fn run_cli_status(args: &[String]) -> std::process::ExitStatus {
    let binary = get_datapipe_binary();
    let mut cmd = Command::new(binary);
    cmd.args(args);
    cmd.kill_on_drop(true);
    let mut child = cmd.spawn().expect("Failed to spawn datapipe");
    child.wait().await.expect("Failed to wait on datapipe")
}

/// Run datapipe binary and capture stdout, stderr, and exit status
pub async fn run_cli_output(args: &[String]) -> std::process::Output {
    let binary = get_datapipe_binary();
    let mut cmd = Command::new(binary);
    cmd.args(args);
    cmd.kill_on_drop(true);
    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn datapipe");
    child
        .wait_with_output()
        .await
        .expect("Failed to execute datapipe")
}

/// Run datapipe binary with piped stdin input
pub async fn run_cli_with_stdin(args: &[String], input_data: &[u8]) -> std::process::Output {
    let binary = get_datapipe_binary();
    let mut cmd = Command::new(binary);
    cmd.args(args);
    cmd.kill_on_drop(true);
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn datapipe with stdin");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input_data)
            .await
            .expect("Failed to write to stdin");
        drop(stdin); // Send EOF
    }

    child
        .wait_with_output()
        .await
        .expect("Failed to wait with output")
}

/// Compare if two files have identical content
pub async fn identical_contents(
    file1: impl AsRef<Path>,
    file2: impl AsRef<Path>,
) -> Result<bool, DatapipeError> {
    let c1 = tokio::fs::read(file1).await?;
    let c2 = tokio::fs::read(file2).await?;
    Ok(c1 == c2)
}

/// Mock HTTP/1.1 Server for testing HttpReader and HttpWriter
pub struct MockHttpServer {
    pub addr: SocketAddr,
    pub received_payloads: Arc<Mutex<Vec<Vec<u8>>>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockHttpServer {
    /// Start a mock HTTP server responding to GET with `response_body` (and specified status),
    /// and accepting PUT requests into `received_payloads`.
    pub async fn start(response_body: Vec<u8>, status_code: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind mock HTTP listener");
        let addr = listener.local_addr().expect("Failed to get local addr");

        let received_payloads = Arc::new(Mutex::new(Vec::new()));
        let payloads_clone = Arc::clone(&received_payloads);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    conn = listener.accept() => {
                        let Ok((mut socket, _)) = conn else { break; };
                        let payloads = Arc::clone(&payloads_clone);
                        let body = response_body.clone();

                        tokio::spawn(async move {
                            let mut buffer = [0u8; 8192];
                            let Ok(n) = socket.read(&mut buffer).await else { return; };
                            if n == 0 { return; }

                            let request = String::from_utf8_lossy(&buffer[..n]);
                            if request.starts_with("GET") {
                                let status_text = match status_code {
                                    200 => "OK",
                                    404 => "Not Found",
                                    500 => "Internal Server Error",
                                    _ => "Unknown",
                                };
                                let response = format!(
                                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    status_code, status_text, body.len()
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                                let _ = socket.write_all(&body).await;
                                let _ = socket.flush().await;
                            } else if request.starts_with("PUT") {
                                // Extract body from request
                                if let Some(header_end) = request.find("\r\n\r\n") {
                                    let body_start = header_end + 4;
                                    let mut received = buffer[body_start..n].to_vec();

                                    // Check content-length
                                    let mut content_length = 0;
                                    for line in request.lines() {
                                        let lower = line.to_lowercase();
                                        if let Some(val) = lower.strip_prefix("content-length:") {
                                            content_length = val.trim().parse::<usize>().unwrap_or(0);
                                        }
                                    }

                                    // Read remaining body if needed
                                    while received.len() < content_length {
                                        let Ok(m) = socket.read(&mut buffer).await else { break; };
                                        if m == 0 { break; }
                                        received.extend_from_slice(&buffer[..m]);
                                    }

                                    payloads.lock().await.push(received);
                                }

                                let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                                let _ = socket.write_all(response.as_bytes()).await;
                                let _ = socket.flush().await;
                            }
                        });
                    }
                }
            }
        });

        Self {
            addr,
            received_payloads,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for MockHttpServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Mock HTTPS Server for testing HttpsReader and HttpsWriter
pub struct MockHttpsServer {
    pub addr: SocketAddr,
    pub cert_pem_path: PathBuf,
    pub received_payloads: Arc<Mutex<Vec<Vec<u8>>>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    _cert_guard: TempFileGuard,
}

impl MockHttpsServer {
    /// Start a mock HTTPS server with self-signed certificate
    pub async fn start(response_body: Vec<u8>, status_code: u16) -> Self {
        init_crypto();
        let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let cert_key = generate_simple_self_signed(subject_alt_names)
            .expect("Failed to generate self-signed cert");

        let cert_der = cert_key.cert.der().clone();
        let cert_pem = cert_key.cert.pem();
        let key_der =
            PrivateKeyDer::from_pem(SectionKind::PrivateKey, cert_key.key_pair.serialize_der())
                .expect("Failed to parse private key DER");

        let cert_guard = TempFileGuard::with_content("pem", cert_pem.as_bytes());
        let cert_pem_path = cert_guard.path.clone();

        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("Failed to build server config");

        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind mock HTTPS listener");
        let addr = listener.local_addr().expect("Failed to get local addr");

        let received_payloads = Arc::new(Mutex::new(Vec::new()));
        let payloads_clone = Arc::clone(&received_payloads);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    conn = listener.accept() => {
                        let Ok((tcp_stream, _)) = conn else { break; };
                        let acceptor = acceptor.clone();
                        let payloads = Arc::clone(&payloads_clone);
                        let body = response_body.clone();

                        tokio::spawn(async move {
                            let Ok(mut tls_stream) = acceptor.accept(tcp_stream).await else { return; };
                            let mut buffer = [0u8; 8192];
                            let Ok(n) = tls_stream.read(&mut buffer).await else { return; };
                            if n == 0 { return; }

                            let request = String::from_utf8_lossy(&buffer[..n]);
                            if request.starts_with("GET") {
                                let status_text = match status_code {
                                    200 => "OK",
                                    404 => "Not Found",
                                    500 => "Internal Server Error",
                                    _ => "Unknown",
                                };
                                let response = format!(
                                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    status_code, status_text, body.len()
                                );
                                let _ = tls_stream.write_all(response.as_bytes()).await;
                                let _ = tls_stream.write_all(&body).await;
                                let _ = tls_stream.flush().await;
                            } else if request.starts_with("PUT") {
                                if let Some(header_end) = request.find("\r\n\r\n") {
                                    let body_start = header_end + 4;
                                    let mut received = buffer[body_start..n].to_vec();

                                    let mut content_length = 0;
                                    for line in request.lines() {
                                        let lower = line.to_lowercase();
                                        if let Some(val) = lower.strip_prefix("content-length:") {
                                            content_length = val.trim().parse::<usize>().unwrap_or(0);
                                        }
                                    }

                                    while received.len() < content_length {
                                        let Ok(m) = tls_stream.read(&mut buffer).await else { break; };
                                        if m == 0 { break; }
                                        received.extend_from_slice(&buffer[..m]);
                                    }

                                    payloads.lock().await.push(received);
                                }

                                let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                                let _ = tls_stream.write_all(response.as_bytes()).await;
                                let _ = tls_stream.flush().await;
                            }
                        });
                    }
                }
            }
        });

        Self {
            addr,
            cert_pem_path,
            received_payloads,
            shutdown_tx: Some(shutdown_tx),
            _cert_guard: cert_guard,
        }
    }

    pub fn url(&self) -> String {
        format!("https://localhost:{}", self.addr.port())
    }
}

impl Drop for MockHttpsServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
