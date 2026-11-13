# Agent Guide for datapipe

This document provides an overview of the `datapipe` codebase and outlines mandatory instructions, conventions, and verification workflows for AI coding agents and automated tools working in this repository.

---

## 1. Mandatory Compliance with `rust-instructions.md`

> [!IMPORTANT]
> **ALL Rust code written, modified, refactored, or reviewed in this repository MUST strictly comply with the requirements, conventions, and patterns defined in `rust-instructions.md`**

Before implementing features, fixing bugs, or refactoring code, agents must review `rust-instructions.md`. Key mandates include:

1. **Zero-Warning Policy**: All code must compile cleanly without warnings. Warnings are treated as errors (`-D warnings`).
2. **Error Handling**:
   - Do **NOT** use `.unwrap()` or `.expect()` in production/library code. Use idiomatic error propagation via `?`, `match`, or `if let`.
   - Use custom error types leveraging `thiserror` (see [`DatapipeError`](file:///workspaces/datapipe/src/datapipe_types.rs)).
   - Do not panic in library code; always return a `Result<T, E>`.
3. **Memory & Borrow Checker**:
   - Prefer borrowing (`&T`, `&str`) over cloning (`.clone()`) unless ownership transfer is explicitly required.
   - Keep iterator chains lazy; avoid premature `.collect()`.
   - Minimize allocations and leverage zero-copy operations where possible.
4. **Safety**:
   - `unsafe` code is strictly prohibited unless absolutely necessary and accompanied by an explicit `// SAFETY:` explanatory comment.
5. **Documentation & Formatting**:
   - Follow the Rust Style Guide and enforce with `cargo fmt`.
   - Provide comprehensive `///` rustdoc comments on all public structs, enums, traits, functions, and methods, including example code utilizing `?` rather than `unwrap()`.
   - All public types must derive/implement `Debug`. Implement standard traits (`Clone`, `PartialEq`, `Default`, `Display`) wherever applicable.
6. **Mandatory Quality & Verification Checks**:
   Agents must run and pass all verification commands prior to finishing a task:
   ```bash
   # 1. Run unit and integration tests (include --all-features to also test GUI components)
   cargo test --locked --all-targets --all-features

   # 2. Run clippy linter (warnings treated as errors)
   cargo clippy --locked --all-targets --all-features -- -D warnings

   # 3. Verify code formatting
   cargo fmt -- --check

   # 4. Validate documentation builds without errors
   cargo doc --locked --no-deps --all-features
   ```

---

## 2. Project Overview

### Purpose
`datapipe` is an asynchronous data streaming CLI utility, desktop GUI application, and Rust library. It enables streaming raw bytes from an input source to one or more output destinations across multiple network, peer-to-peer, and system protocols, with optional in-line streaming encryption and decryption.

- **Crate Name**: `datapipe`
- **Version**: `0.3.0`
- **Rust Edition**: `2024`
- **Maturity**: Beta
- **License**: AGPL-3.0-only
- **Runtime**: Asynchronous runtime powered by [Tokio](https://tokio.rs) (`tokio = { version = "1", features = ["full"] }`).
- **Optional GUI Feature**: `gui` (enables [`datapipe-gui`](file:///workspaces/datapipe/src/bin/datapipe_gui.rs) via `eframe`, `egui`, and `rfd`).

---

## 3. Architecture and Data Flow

`datapipe` is structured as a pipeline of asynchronous Tokio tasks communicating through bounded MPSC channels (`tokio::sync::mpsc::channel` with a buffer size of `QUEUE_SIZE = 2048`).

```
                    ┌─────────────────┐
                    │   InputReader   │
                    │ (File/Net/Pipe) │
                    └────────┬────────┘
                             │ (mpsc channel: Vec<u8>)
                             ▼
                    ┌─────────────────┐
                    │ StreamDecryptor │ (optional --decrypt)
                    │ (ChaCha20P1305) │
                    └────────┬────────┘
                             │ (mpsc channel: Vec<u8>)
                             ▼
                    ┌─────────────────┐
                    │ StreamEncryptor │ (optional --encrypt)
                    │ (ChaCha20P1305) │
                    └────────┬────────┘
                             │ (mpsc channel: Vec<u8>)
                             ▼
                    ┌─────────────────┐
                    │  OutputWriters  │ (fan-out / multicast)
                    │ (File/Net/Pipe) │
                    └─────────────────┘
```

### Core Pipeline Components
1. **Pipeline Engine ([`src/engine.rs`](file:///workspaces/datapipe/src/engine.rs))**:
   - `run_datapipe(parameters)`: Supervises pipeline stages in a `tokio::task::JoinSet`, aborting remaining tasks immediately if any stage fails.
   - `reader_child`: Continuously invokes `InputReader::read()` and pushes byte chunks to the outgoing queue. Handles transient read retries with backoff up to `RETRY_MAX = 5`.
   - `decryptor_child`: Optional stage receiving encrypted frames, producing decrypted plaintext, and invoking `StreamDecryptor::finish()` at EOF to verify authenticated end-of-stream framing.
   - `encryptor_child`: Optional stage encrypting plaintext into length-prefixed authenticated cipher chunks and invoking `StreamEncryptor::finish()` at EOF to emit the authenticated end-of-stream frame.
   - `writer_child`: Consumes chunks and writes them to one or more configured `OutputWriter` destinations, tracking consecutive failures per destination (failing the pipeline if any writer reaches `RETRY_MAX = 5`) and closing all writers (`close_all_writers` -> `OutputWriter::close()`) on EOF or error.
2. **Parameters Builder ([`src/parameters.rs`](file:///workspaces/datapipe/src/parameters.rs))**:
   - Provides `ParametersBuilder` to assemble and validate configured readers, decryptors, encryptors, writers, and optional `DatapipeMetrics` before pipeline startup.
3. **Core Types and Traits ([`src/datapipe_types.rs`](file:///workspaces/datapipe/src/datapipe_types.rs))**:
   - `InputReader`: Trait for sources (`async fn read(&mut self) -> Result<Bytes, std::io::Error>`).
   - `OutputWriter`: Trait for sinks (`async fn write(&mut self, bytes: &[u8]) -> Result<(), std::io::Error>` and `async fn close(&mut self) -> Result<(), std::io::Error>` for flushing/shutdown).
   - `DatapipeError`: Unified error type implemented via `thiserror`.
   - `EncryptionKey`: Full-entropy 51-byte secret (32-byte key + 19-byte nonce) for ChaCha20-Poly1305 AEAD, generated and formatted as `dpk1:<68 base64url chars>` (also parsing `hex:<102 hex chars>` and legacy 51-byte UTF-8 keys), with redacted `Debug` output and zeroization on `Drop`.
4. **Command-Line Parsing & TOML Configuration ([`src/args.rs`](file:///workspaces/datapipe/src/args.rs), [`src/config.rs`](file:///workspaces/datapipe/src/config.rs))**:
   - [`ProgramArgs`](file:///workspaces/datapipe/src/args.rs) is built using `clap` (derive) and manages CLI flags (`--encrypt-generate-key` printing to `stderr`, `--decrypt-allow-legacy`), PEM/DER TLS asset loading (`build_tls_root_cert_store`, `load_https_tls_assets`, `get_root_ca`, `get_tls_cert_chain`, `get_tls_private_key`), Iroh options, and custom redacting `Debug` implementations on secret-bearing option groups (`DecryptionArgs`, `EncryptionArgs`, `IrohInputArgs`, `IrohListenInputArgs`, `IrohOutputArgs`).
   - [`DatapipeConfig`](file:///workspaces/datapipe/src/config.rs) & [`OutputDestinationConfig`](file:///workspaces/datapipe/src/config.rs) provide TOML configuration serialization, shared validation (`count_outputs`, `validate_output_params` rejecting empty HTTP/HTTPS delimiters and simultaneous `[output]`/`[[outputs]]` definitions), restrictive `0o600` Unix permissions when saving configs containing secrets (`save_to_file`), and `to_parameters()` supporting 1 to 64 output destinations.
5. **Metrics & Live Display ([`src/metrics.rs`](file:///workspaces/datapipe/src/metrics.rs))**:
   - `ByteMetrics`, `ElapsedTime`, `DatapipeMetrics`, and `LiveDisplayGuard` track real-time byte counts, elapsed duration, and throughput across both CLI and GUI.
6. **Entrypoints ([`src/bin/datapipe.rs`](file:///workspaces/datapipe/src/bin/datapipe.rs), [`src/bin/datapipe_gui.rs`](file:///workspaces/datapipe/src/bin/datapipe_gui.rs))**:
   - `datapipe`: Installs the default `rustls` `ring` crypto provider, configures the `log4rs` logger (`DatapipeError`-returning initialization without `.unwrap()`), builds `Parameters` directly via `DatapipeConfig::to_parameters().await` under `--use-config` (preserving `[[outputs]]` fan-out while honoring CLI `--no-metrics`) or from CLI args, starts live stderr metrics unless disabled, and runs `run_datapipe`.
   - `datapipe-gui` (`--features gui`): Installs the `ring` crypto provider, initializes `log4rs`, and launches `datapipe::gui::run_gui()` returning `ExitCode` cleanly on error instead of forcing `process::exit`.
7. **Graphical User Interface ([`src/gui/`](file:///workspaces/datapipe/src/gui/mod.rs), behind `#[cfg(feature = "gui")]`)**:
   - [`mod.rs`](file:///workspaces/datapipe/src/gui/mod.rs): `DatapipeApp` and `run_gui` shell with Material Design 3 top bar, navigation tabs, and theme toggle.
   - [`pipeline.rs`](file:///workspaces/datapipe/src/gui/pipeline.rs): `PipelineController` managing background Tokio pipeline execution, duplicate-launch prevention in `start()`, live/frozen metric snapshots, and cancellation.
   - [`types.rs`](file:///workspaces/datapipe/src/gui/types.rs): UI form state models (`InputState`, `DecryptState` with `allow_legacy` v1 toggle, `EncryptState`, `OutputState` with delimiter escape parsing/formatting via `parse_gui_delimiter`/`format_gui_delimiter`), validation, single-vs-multi output `states_to_config`, and `load_config_into_states`.
   - [`style.rs`](file:///workspaces/datapipe/src/gui/style.rs): Material Design 3 Tonal Blue Light/Dark theme, vector-painted icons, clipboard-enabled text fields (`Cut`/`Copy`/`Paste`/`Select All`), and masked `m3_secret_field` (ASCII `'*'` layouter + `Show`/`Hide` toggle) plus shared helpers (`m3_field_label`, `m3_insecure_checkbox`, `m3_encryption_key_status`).
   - Tab views: [`input_tab.rs`](file:///workspaces/datapipe/src/gui/input_tab.rs), [`decrypt_tab.rs`](file:///workspaces/datapipe/src/gui/decrypt_tab.rs), [`encrypt_tab.rs`](file:///workspaces/datapipe/src/gui/encrypt_tab.rs), [`output_tab.rs`](file:///workspaces/datapipe/src/gui/output_tab.rs), [`run_tab.rs`](file:///workspaces/datapipe/src/gui/run_tab.rs).

---

## 4. Supported Protocols & Dispatchers

The codebase wraps all concrete reader and writer implementations in enum dispatchers:

### Inputs ([`src/reader.rs`](file:///workspaces/datapipe/src/reader.rs))
| Protocol | Implementation File | Description |
| :--- | :--- | :--- |
| `FILE` | [`src/file_reader.rs`](file:///workspaces/datapipe/src/file_reader.rs) | Reads from local file via `tokio::fs::File` |
| `HTTP` | [`src/http_reader.rs`](file:///workspaces/datapipe/src/http_reader.rs) | Polls HTTP endpoints with configurable interval (`reqwest`, `0` ms polls continuously) |
| `HTTPS` | [`src/https_reader.rs`](file:///workspaces/datapipe/src/https_reader.rs) | Secure HTTPS polling with PEM/DER root CAs, PEM CRLs, client certs, and rate `0` ms continuous support |
| `Iroh` | [`src/iroh_reader_writer.rs`](file:///workspaces/datapipe/src/iroh_reader_writer.rs) | Connects outbound over Iroh P2P QUIC by `EndpointId` to receive data |
| `Iroh Listen` | [`src/iroh_listen_reader.rs`](file:///workspaces/datapipe/src/iroh_listen_reader.rs) | Binds local Iroh `Endpoint`, accepts connection (optional `allowed_peer`), and reads stream |
| `STDIN` | [`src/stdin_reader.rs`](file:///workspaces/datapipe/src/stdin_reader.rs) | Reads bytes from standard input (`tokio::io::stdin`) |
| `TCP` | [`src/tcp_reader_writer.rs`](file:///workspaces/datapipe/src/tcp_reader_writer.rs) | Connects outbound to a TCP server and reads into fixed-size stack buffers |
| `TCP Listen` | [`src/tcp_listen_reader.rs`](file:///workspaces/datapipe/src/tcp_listen_reader.rs) | Binds local port and accepts inbound TCP connection |
| `TLS` | [`src/tls_reader_writer.rs`](file:///workspaces/datapipe/src/tls_reader_writer.rs) | Connects outbound via TLS (`tokio-rustls`) with IPv4/IPv6/hostname SNI handling |
| `TLS Listen` | [`src/tls_listen_reader.rs`](file:///workspaces/datapipe/src/tls_listen_reader.rs) | Binds local port and accepts TLS connection (fail-closed client auth by default) |
| `UDP` | [`src/udp_reader.rs`](file:///workspaces/datapipe/src/udp_reader.rs) | Receives UDP packets using a 64 KiB datagram buffer |
| `UDP Multicast` | [`src/udp_reader.rs`](file:///workspaces/datapipe/src/udp_reader.rs) | Validates IPv4 multicast `A.B.C.D[:PORT]`, joins group, and receives datagrams |

### Outputs ([`src/writer.rs`](file:///workspaces/datapipe/src/writer.rs))
| Protocol | Implementation File | Description |
| :--- | :--- | :--- |
| `FILE` | [`src/file_writer.rs`](file:///workspaces/datapipe/src/file_writer.rs) | Writes via `write_all` and flushes/shuts down cleanly on `close()` |
| `HTTP` | [`src/http_writer.rs`](file:///workspaces/datapipe/src/http_writer.rs) | Uploads delimited frames via `DelimitedHttpSender` (`PUT`), checks status codes, and flushes on `close()` |
| `HTTPS` | [`src/https_writer.rs`](file:///workspaces/datapipe/src/https_writer.rs) | Uploads delimited frames securely over HTTPS via `build_https_client` and `DelimitedHttpSender` |
| `Iroh` | [`src/iroh_reader_writer.rs`](file:///workspaces/datapipe/src/iroh_reader_writer.rs) | Connects outbound over Iroh P2P QUIC by `EndpointId` to stream data |
| `STDOUT` | [`src/stdout_writer.rs`](file:///workspaces/datapipe/src/stdout_writer.rs) | Writes to standard output (`tokio::io::stdout`) |
| `TCP` | [`src/tcp_reader_writer.rs`](file:///workspaces/datapipe/src/tcp_reader_writer.rs) | Writes data to connected TCP socket and flushes/shuts down on `close()` |
| `TLS` | [`src/tls_reader_writer.rs`](file:///workspaces/datapipe/src/tls_reader_writer.rs) | Writes data securely over TLS socket and flushes/shuts down on `close()` |
| `UDP` | [`src/udp_writer.rs`](file:///workspaces/datapipe/src/udp_writer.rs) | Sends UDP packets to target socket address |

---

## 5. Security, Cryptography, and Reliability Guidelines

1. **In-Line Stream Encryption ([`src/encryption.rs`](file:///workspaces/datapipe/src/encryption.rs), [`src/datapipe_types.rs`](file:///workspaces/datapipe/src/datapipe_types.rs))**:
   - Uses `chacha20poly1305` (`XChaCha20Poly1305` with `EncryptorBE32`/`DecryptorBE32` from `aead-stream`).
   - Version 2 framing (`MESSAGE_START_V2`) binds frame metadata via AEAD AAD, enforces `MAX_CIPHER_FRAME_LEN` (64 MiB) with checked arithmetic and non-panicking `MessageLength`, and authenticates EOF via `encrypt_last`/`decrypt_last` (`finish()`). Legacy unauthenticated v1 streams (`MESSAGE_START`) are rejected unless `--decrypt-allow-legacy` / `StreamDecryptor::new_with_legacy` is enabled.
   - `EncryptionKey` wraps a 51-byte secret (32-byte key + 19-byte nonce): `generate()` / `Display` emit full-entropy `dpk1:<68 base64url chars>` keys, `new()` parses `dpk1:`, `hex:<102 hex chars>`, and legacy 51-byte UTF-8 keys, `--encrypt-generate-key` prints to `stderr`, `Debug` is redacted, and `Drop` zeroizes key material.
   - GUI secret inputs use `m3_secret_field` (ASCII `'*'` mask + `Show`/`Hide` toggle), CLI arg groups redact secrets in `Debug`, and `DatapipeConfig::save_to_file` enforces `0o600` Unix permissions when secrets are saved.
2. **TLS Engine & HTTPS ([`src/args.rs`](file:///workspaces/datapipe/src/args.rs), [`src/https_reader.rs`](file:///workspaces/datapipe/src/https_reader.rs), [`src/https_writer.rs`](file:///workspaces/datapipe/src/https_writer.rs))**:
   - Backed by `rustls` (version 0.23+) and `tokio-rustls`; `rustls::crypto::ring` is registered globally at startup.
   - TLS loaders (`get_root_ca`, `get_tls_cert_chain`, `get_tls_private_key`) and HTTPS root CA loading automatically detect both PEM and DER formats and reject empty certificate/CA files. HTTPS CRLs and client identities expect non-empty PEM input.
   - `build_https_client` rejects empty root CA/CRL lists and disallows CRLs or `--*-allow-invalid-hostnames` (when `--*-allow-invalid-certificates` is false) unless custom root CA certificates are provided.
   - Testing flags (`--*-allow-invalid-certificates`, `--*-allow-invalid-hostnames`, `--*-skip-server-verify`, `--tls-listen-input-skip-client-verify`, `--tls-listen-input-generate-self-signed`) emit warnings and stay opt-in: `--tls-listen-input-generate-self-signed` keeps client certificate verification enabled (fail-closed) unless `--tls-listen-input-skip-client-verify` is also passed.
   - `get_domain` in [`src/tls_reader_writer.rs`](file:///workspaces/datapipe/src/tls_reader_writer.rs) extracts hostnames cleanly for IPv4, bracketed/unbracketed IPv6, and DNS names.
3. **Iroh Peer-to-Peer Transport ([`src/iroh_listen_reader.rs`](file:///workspaces/datapipe/src/iroh_listen_reader.rs), [`src/iroh_reader_writer.rs`](file:///workspaces/datapipe/src/iroh_reader_writer.rs))**:
   - Uses `iroh` (`1.4`) QUIC transport with Ed25519 `EndpointId` cryptographic peer identity (`SecretKey` 32-byte key in 64-character hex or z-base-32 format).
   - `IrohListenReader` supports `allowed_peer` (`--iroh-listen-input-allowed-peer`) to restrict inbound connections to a specific remote peer `EndpointId` and logs a warning if omitted.
   - `IrohReader`/`IrohWriter` close endpoints with a timeout on setup failures and treat graceful zero-code `ApplicationClosed` closes cleanly.
   - [`.cargo/audit.toml`](file:///workspaces/datapipe/.cargo/audit.toml) temporarily ignores `RUSTSEC-2024-0436` (`paste` unmaintained, pulled in transitively via `iroh` -> `netwatch` -> `netlink-packet-core`); [`git-hooks/pre-commit`](file:///workspaces/datapipe/git-hooks/pre-commit) checks crates.io for newer `iroh` releases updating `netwatch`.
4. **Transport, Buffer & Error-Handling Hardening**:
   - **Shared HTTP/HTTPS Logic**: `build_poll_interval` treats `0` ms rate as continuous operation (no zero-duration timer panic), `read_http_response` checks HTTP status codes cleanly, and `DelimitedHttpSender` rejects empty delimiters, drains multi-frame chunks, checks `error_for_status()`, preserves rollback state on failed retries, and flushes trailing bytes on `close()`.
   - **Buffer & Listener Correctness**: `TcpReaderWriter`, `TlsReaderWriter`, and `UdpReader` allocate initialized receive buffers rather than 0-length `Vec::with_capacity(...)`; `UdpReader::new_multicast` validates `is_multicast()` and `A.B.C.D[:PORT]` input without `.unwrap()`; `TcpListenReader` and `TlsListenReader` initialize streams via `Option::insert`; [`src/logger.rs`](file:///workspaces/datapipe/src/logger.rs) and [`src/utilities.rs`](file:///workspaces/datapipe/src/utilities.rs) propagate errors cleanly without `.unwrap()`.

---

## 6. Testing Conventions

- **Unit Tests**:
  - Located within test modules (`mod tests { ... }`) adjacent to the code they test in `src/`.
  - Use `#[tokio::test]` for asynchronous test functions.
- **Integration Tests**:
  - [`tests/integration_test_datapipe.rs`](file:///workspaces/datapipe/tests/integration_test_datapipe.rs): End-to-end binary tests across ephemeral ports using [`get_unused_port()`](file:///workspaces/datapipe/src/utilities.rs) and [`identical_contents()`](file:///workspaces/datapipe/src/utilities.rs). When spawning reader and writer processes in tests (e.g., TCP/TLS listeners), ensure the listener binds before launching the client to avoid `ECONNREFUSED` races.
  - [`tests/test_cli_cross_product.rs`](file:///workspaces/datapipe/tests/test_cli_cross_product.rs): CLI reader/writer cross-product matrix tests.
  - [`tests/test_library_readers_writers.rs`](file:///workspaces/datapipe/tests/test_library_readers_writers.rs): Library API (`ParametersBuilder`, `run_datapipe`, `InputReader`/`OutputWriter`) cross-product tests.
  - [`tests/test_error_handling.rs`](file:///workspaces/datapipe/tests/test_error_handling.rs): Validation, malformed input, and error-propagation scenarios across CLI and library APIs.
  - [`tests/test_metrics.rs`](file:///workspaces/datapipe/tests/test_metrics.rs): Metrics tracking (`ByteMetrics`, `ElapsedTime`, `DatapipeMetrics`) and CLI live display tests.
  - [`tests/test_gui.rs`](file:///workspaces/datapipe/tests/test_gui.rs): Material Design 3 GUI state, round-trip TOML conversion, clipboard/ASCII glyph, and rendering tests (`#![cfg(feature = "gui")]`; requires `--features gui` or `--all-features`).
- **CI & Pre-Commit Checks**:
  - [`.github/workflows/build-and-test.yml`](file:///workspaces/datapipe/.github/workflows/build-and-test.yml) and [`git-hooks/pre-commit`](file:///workspaces/datapipe/git-hooks/pre-commit) validate non-ASCII filenames, `cargo fmt`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo doc --locked --no-deps --all-features`, `cargo test --locked --all-targets --all-features` (CI) / `cargo test` (hook), and `cargo audit`.

---

## 7. Agent Checklist Before Submitting Code

When modifying or introducing code, verify every step below:

- [ ] Reviewed [`rust-instructions.md`](file:///workspaces/datapipe/rust-instructions.md) to ensure design complies with community standards and project patterns.
- [ ] No `.unwrap()` or `.expect()` used in production paths.
- [ ] All public items documented with `///` rustdoc comments and examples.
- [ ] `cargo test --locked --all-targets --all-features` passes cleanly.
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings` reports 0 warnings and 0 errors.
- [ ] `cargo fmt -- --check` reports clean formatting.
- [ ] `cargo doc --locked --no-deps --all-features` generates documentation without errors or missing doc warnings.

