# Agent Guide for datapipe

This document provides an overview of the `datapipe` codebase and outlines mandatory instructions, conventions, and verification workflows for AI coding agents and automated tools working in this repository.

---

## 1. Mandatory Compliance with `rust.instructions.md`

> [!IMPORTANT]
> **ALL Rust code written, modified, refactored, or reviewed in this repository MUST strictly comply with the requirements, conventions, and patterns defined in [`rust.instructions.md`](file:///workspaces/datapipe/rust.instructions.md).**

Before implementing features, fixing bugs, or refactoring code, agents must review [`rust.instructions.md`](file:///workspaces/datapipe/rust.instructions.md). Key mandates include:

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
   # 1. Run unit and integration tests
   cargo test

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
`datapipe` is an asynchronous data streaming CLI utility and library written in Rust. It enables streaming raw bytes from an input source to one or more output destinations across multiple network and system protocols, with optional in-line streaming encryption and decryption.

- **Crate Name**: `datapipe`
- **Rust Edition**: `2024`
- **Maturity**: Alpha
- **License**: AGPL-3.0-only
- **Runtime**: Asynchronous runtime powered by [Tokio](https://tokio.rs) (`tokio = { version = "1", features = ["full"] }`).

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
   - `run_data_pipe(parameters)`: Spawns the pipeline tasks and waits for their completion.
   - `reader_child`: Continuously invokes `InputReader::read()` and pushes byte chunks to the outgoing queue. Handles retries with backoff up to `RETRY_MAX = 5`.
   - `decryptor_child`: Optional stage receiving encrypted frames and producing decrypted plaintext.
   - `encryptor_child`: Optional stage encrypting plaintext into length-prefixed authenticated cipher chunks.
   - `writer_child`: Consumes chunks and writes them to one or more configured `OutputWriter` destinations.
2. **Parameters Builder ([`src/parameters.rs`](file:///workspaces/datapipe/src/parameters.rs))**:
   - Provides `ParametersBuilder` to assemble and validate configured readers, decryptors, encryptors, and writers before pipeline startup.
3. **Core Types and Traits ([`src/datapipe_types.rs`](file:///workspaces/datapipe/src/datapipe_types.rs))**:
   - `InputReader`: Trait for sources (`async fn read(&mut self) -> Result<Bytes, std::io::Error>`).
   - `OutputWriter`: Trait for sinks (`async fn write(&mut self, bytes: &[u8]) -> Result<(), std::io::Error>`).
   - `DatapipeError`: Unified error type implemented via `thiserror`.
   - `EncryptionKey`: 51-byte ASCII key representation (32-byte key + 19-byte nonce) for ChaCha20-Poly1305 AEAD.
4. **Command-Line Parsing & Configuration ([`src/args.rs`](file:///workspaces/datapipe/src/args.rs))**:
   - Built using `clap` (derive). Manages CLI flags and validates input combinations, certificate configurations, and encryption keys.
5. **Entrypoint ([`src/bin/datapipe.rs`](file:///workspaces/datapipe/src/bin/datapipe.rs))**:
   - Installs the default `rustls` `ring` crypto provider, configures the `log4rs` logger, parses CLI arguments into `Parameters`, and executes the pipeline.

---

## 4. Supported Protocols & Dispatchers

The codebase wraps all concrete reader and writer implementations in enum dispatchers:

### Inputs ([`src/reader.rs`](file:///workspaces/datapipe/src/reader.rs))
| Protocol | Implementation File | Description |
| :--- | :--- | :--- |
| `FILE` | [`src/file_reader.rs`](file:///workspaces/datapipe/src/file_reader.rs) | Reads from local file via `tokio::fs::File` |
| `HTTP` | [`src/http_reader.rs`](file:///workspaces/datapipe/src/http_reader.rs) | Polls HTTP endpoints with configurable interval (`reqwest`) |
| `HTTPS` | [`src/https_reader.rs`](file:///workspaces/datapipe/src/https_reader.rs) | Secure HTTPS polling with client certs, custom CA, and CRLs |
| `STDIN` | [`src/stdin_reader.rs`](file:///workspaces/datapipe/src/stdin_reader.rs) | Reads bytes from standard input (`tokio::io::stdin`) |
| `TCP` | [`src/tcp_reader_writer.rs`](file:///workspaces/datapipe/src/tcp_reader_writer.rs) | Connects outbound to a TCP server |
| `TCP Listen` | [`src/tcp_listen_reader.rs`](file:///workspaces/datapipe/src/tcp_listen_reader.rs) | Binds local port and accepts inbound TCP connection |
| `TLS` | [`src/tls_reader_writer.rs`](file:///workspaces/datapipe/src/tls_reader_writer.rs) | Connects outbound via TLS (`tokio-rustls`) |
| `TLS Listen` | [`src/tls_listen_reader.rs`](file:///workspaces/datapipe/src/tls_listen_reader.rs) | Binds local port and accepts TLS connection (self-signed cert support) |
| `UDP` | [`src/udp_reader.rs`](file:///workspaces/datapipe/src/udp_reader.rs) | Receives UDP packets on a socket |
| `UDP Multicast` | [`src/udp_reader.rs`](file:///workspaces/datapipe/src/udp_reader.rs) | Joins and receives from a UDP multicast group |

### Outputs ([`src/writer.rs`](file:///workspaces/datapipe/src/writer.rs))
| Protocol | Implementation File | Description |
| :--- | :--- | :--- |
| `FILE` | [`src/file_writer.rs`](file:///workspaces/datapipe/src/file_writer.rs) | Appends/writes to local file |
| `HTTP` | [`src/http_writer.rs`](file:///workspaces/datapipe/src/http_writer.rs) | Posts chunks/delimited data to HTTP endpoint |
| `HTTPS` | [`src/https_writer.rs`](file:///workspaces/datapipe/src/https_writer.rs) | Posts chunks securely over HTTPS with TLS client identity |
| `STDOUT` | [`src/stdout_writer.rs`](file:///workspaces/datapipe/src/stdout_writer.rs) | Writes to standard output (`tokio::io::stdout`) |
| `TCP` | [`src/tcp_reader_writer.rs`](file:///workspaces/datapipe/src/tcp_reader_writer.rs) | Writes data to connected TCP socket |
| `TLS` | [`src/tls_reader_writer.rs`](file:///workspaces/datapipe/src/tls_reader_writer.rs) | Writes data securely over TLS socket |
| `UDP` | [`src/udp_writer.rs`](file:///workspaces/datapipe/src/udp_writer.rs) | Sends UDP packets to target socket address |

---

## 5. Security and Cryptography Guidelines

1. **In-Line Stream Encryption ([`src/encryption.rs`](file:///workspaces/datapipe/src/encryption.rs))**:
   - Uses `chacha20poly1305` in stream mode.
   - Keys are 51 characters in length (valid ASCII/UTF-8), composed of a 32-byte cipher key and a 19-byte nonce.
   - Encrypted data chunks are length-encoded before transmission and decoded on receipt.
2. **TLS Engine**:
   - Backed by `rustls` (version 0.23+) and `tokio-rustls`.
   - Ring crypto provider (`rustls::crypto::ring`) is registered globally at application startup.
   - Certificates and keys are DER or PEM format depending on the CLI mode.
   - Testing flags (`--allow-invalid-certificates`, `--tls-listen-input-skip-client-verify`, `--tls-listen-input-generate-self-signed`) must be handled with appropriate warnings and guarded against unsafe defaults.

---

## 6. Testing Conventions

- **Unit Tests**:
  - Located within test modules (`mod tests { ... }`) adjacent to the code they test in `src/`.
  - Use `#[tokio::test]` for asynchronous test functions.
- **Integration Tests ([`tests/integration_test_datapipe.rs`](file:///workspaces/datapipe/tests/integration_test_datapipe.rs))**:
  - Spawn actual `datapipe` binary processes across ephemeral ports using [`get_unused_port()`](file:///workspaces/datapipe/src/utilities.rs).
  - When spawning reader and writer instances in tests (e.g. TCP/TLS listeners), always ensure the listener process has bound to the port before launching the connecting client (e.g., via a short sleep or synchronization delay) to prevent `ECONNREFUSED` connection race conditions.
  - Test files are compared using [`identical_contents()`](file:///workspaces/datapipe/src/utilities.rs).

---

## 7. Agent Checklist Before Submitting Code

When modifying or introducing code, verify every step below:

- [ ] Reviewed [`rust.instructions.md`](file:///workspaces/datapipe/rust.instructions.md) to ensure design complies with community standards and project patterns.
- [ ] No `.unwrap()` or `.expect()` used in production paths.
- [ ] All public items documented with `///` rustdoc comments and examples.
- [ ] `cargo test` passes cleanly.
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings` reports 0 warnings and 0 errors.
- [ ] `cargo fmt -- --check` reports clean formatting.
- [ ] `cargo doc --locked --no-deps --all-features` generates documentation without errors or missing doc warnings.

