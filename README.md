# datapipe
Stream data from here to there

datapipe is a tool used to stream data from one place to another across a variety of protocols.

A datapipe is configured by selecting one input and one or more outputs.  Data streams from the input to the output(s) until no more input is available.

# Navigation
1. [Input Protocols](#input-protocols)
2. [Output Protocols](#output-protocols)
3. [In Transit Options](#in-transit-options)
4. [Metrics & Live Display](#metrics--live-display)
5. [Configuration Files](#configuration-files)
6. [Graphical User Interface](#graphical-user-interface)
7. [Program Information](#program-information)
8. [Library API](#library-api)

## Input protocols
* [FILE](#file-input) - read data from a file
* [HTTP](#http-input) - read data from an HTTP URL
* [HTTPS](#https-input) - read data securely from an HTTPS URL
* [Iroh](#iroh-input) - connect outbound and read data from a remote Iroh peer by EndpointId
* [Iroh Listen](#iroh-listen-input) - open a local Iroh endpoint to listen and receive data from a connecting peer
* [STDIN](#stdin-input) - read data from stdin (keyboard or piped output)
* [TCP](#tcp-input) - read data from a TCP address and port
* [TCP Listen](#tcp-listen-input) - open a local port to listen and receive data using a TCP connection
* [TLS](#tls-input) - read data securely from a TLS address and port
* [TLS Listen](#tls-listen-input) - open a local port to listen and receive data using a TLS connection
* [UDP](#udp-input) - read data from a UDP address and port
* [UDP Multicast](#udp-multicast-input) - read data from a UDP multicast address and port

### File Input
File input requires the path to the file.

```datapipe --file-input /home/me/that_file.dat```

### HTTP Input
HTTP input requires a URL and an input read rate (`--http-input-rate`) specified in milliseconds.  The input rate specifies how often to read from the URL.  It is expected that the data at the URL will change, thus the need to read repeatedly.  A tool such as [wget](https://www.gnu.org/software/wget/) or [curl](https://curl.se/) should be used for one-time downloads.  Specifying an input rate of `0` polls continuously without delay, which could negatively impact the target web server's performance.  Non-2xx HTTP status responses are reported as read errors.

```datapipe --http-input http://local-weather.org/hourly_forecast --http-input-rate 3600000```

### HTTPS Input
HTTPS input requires a URL and an input read rate (`--https-input-rate`) specified in milliseconds.  The input rate specifies how often to read from the URL.  It is expected that the data at the URL will change, thus the need to read repeatedly.  A tool such as [wget](https://www.gnu.org/software/wget/) or [curl](https://curl.se/) should be used for one-time downloads.  Specifying an input rate of `0` polls continuously without delay, which could negatively impact the target web server's performance.  Non-2xx HTTPS status responses are reported as read errors.

```datapipe --https-input https://stock-ticker.net/ABCD --https-input-rate 5000```

Custom certificates can be specified (`--https-input-root-certificates`) by giving the path to a non-empty certificates file in PEM bundle or DER format.

A custom certificate revocation list can be specified (`--https-input-certificate-revocation-list`) by giving the path to a non-empty certificate revocation list file in PEM format.  Using a CRL requires custom root certificates (`--https-input-root-certificates`).

HTTPS client identity can be specified (`--https-input-client-identity`) by giving the path to the client's private key and X509 certificate in PEM format.  The private key must be RSA, SEC1 Elliptic Curve, or PKCS#8.

In rare circumstances, hostname validation can be skipped (`--https-input-allow-invalid-hostnames`; requires custom `--https-input-root-certificates` unless `--https-input-allow-invalid-certificates` is also set).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source web server.

In rare circumstances, invalid certificates can be accepted (`--https-input-allow-invalid-certificates`).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source web server.

### Iroh Input
Iroh input connects outbound over peer-to-peer QUIC (`iroh`) to a remote endpoint identified by its `EndpointId` (64-character hex or z-base-32 Ed25519 public key) and reads incoming stream data.

```datapipe --iroh-input <REMOTE_ENDPOINT_ID> --iroh-input-addrs 192.168.1.50:11204 --file-output received.dat```

Optional flags for `--iroh-input`:
* `--iroh-input-addrs <ADDRS>`: comma- or whitespace-separated direct IP socket addresses (`ip:port`) for the target Iroh endpoint.
* `--iroh-input-relay-url <URL>`: custom relay server URL (`http://` or `https://`).
* `--iroh-input-alpn <ALPN>`: custom ALPN protocol identifier (defaults to `datapipe/iroh/1`).
* `--iroh-input-secret-key <KEY>`: local 32-byte Ed25519 secret key in 64-character hex or z-base-32 format to authenticate the connecting client endpoint.
* `--iroh-input-no-relay`: disable default relay servers and address discovery (direct IP transport only).

### Iroh Listen Input
Iroh Listen input binds a local `iroh` endpoint, waits for an incoming peer-to-peer QUIC connection matching the configured ALPN, and streams received bytes into the pipeline.

```datapipe --iroh-listen-input --iroh-listen-input-bind-addr 0.0.0.0:11204 --iroh-listen-input-allowed-peer <SENDER_ENDPOINT_ID> --file-output received.dat```

Optional flags for `--iroh-listen-input`:
* `--iroh-listen-input-bind-addr <ADDR>`: local UDP socket address (`ip:port`) to bind (uses an OS-assigned port if omitted).
* `--iroh-listen-input-secret-key <KEY>`: local 32-byte Ed25519 secret key in 64-character hex (or z-base-32) format for a deterministic `EndpointId`.
* `--iroh-listen-input-alpn <ALPN>`: custom ALPN protocol identifier (defaults to `datapipe/iroh/1`).
* `--iroh-listen-input-relay-url <URL>`: custom relay server URL (`http://` or `https://`).
* `--iroh-listen-input-allowed-peer <ENDPOINT_ID>`: restrict inbound connections to the specified remote peer `EndpointId` (recommended; if omitted, `datapipe` logs a warning because any peer knowing the `EndpointId` and ALPN can connect).
* `--iroh-listen-input-no-relay`: disable default relay servers and address discovery (direct IP transport only).

### STDIN Input
STDIN input allows datapipe to accept input from the keyboard or the output from a pipe.

```./my_useful_program --do-that-thing | datapipe --stdin-input```

### TCP Input
TCP input connects and reads data from a TCP address and port.

```datapipe --tcp-input 10.50.70.90:2255```

### TCP Listen Input
TCP Listen input opens a TCP port on the local machine to listen and accept a TCP connection.

```datapipe --tcp-listen-input localhost:9090```

### TLS Input
TLS input securely reads data from a TLS address and port.

```datapipe --tls-input 10.50.70.90:2288 --tls-input-cert-chain certificates.der --tls-input-client-key client_key.der --tls-input-root-ca ca_root.der```

A custom certificate chain (`--tls-input-cert-chain`) can be specified if needed along with a custom client private key (`--tls-input-client-key`) to ensure client integrity.  If one of these options is used, the other must also be used.  The certificate chain can be in PEM or DER format.  The private key can be a PEM- or DER-encoded PKCS#1, PKCS#8, or SEC1 key.

TLS input uses web Certificate Authority roots by default.  A custom Certificate Authority root can be used (`--tls-input-root-ca`) if wanted, in PEM or DER format.

In rare circumstances, TLS server verification can be skipped (`--tls-input-skip-server-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source server.

### TLS Listen Input
TLS Listen input opens a TLS port on the local machine to listen and accept a TLS connection.

```datapipe --tls-listen-input localhost:9191 --tls-listen-input-cert-chain certificates.der --tls-listen-input-server-key key.der```

A custom certificate chain (`--tls-listen-input-cert-chain`) must be specified along with a custom server private key (`--tls-listen-input-server-key`) to prove server identity.  The certificate chain can be in PEM or DER format.  The private key can be a PEM- or DER-encoded PKCS#1, PKCS#8, or SEC1 key.

Default behavior is to verify client identity against the certificate chain.  In rare circumstances, TLS client verification can be skipped (`--tls-listen-input-skip-client-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be an authorized client.

For convenience in testing environments, a self-signed server certificate and private key can be generated and used (`--tls-listen-input-generate-self-signed`).  DANGER! This should only be used for testing in controlled environments.  Generating a self-signed certificate and private key means that clients will not be able to verify the server's identity unless they disable verification.  Client certificate verification remains enabled (fail-closed) by default; pass `--tls-listen-input-skip-client-verify` explicitly in testing if clients do not present certificates trusted by the generated self-signed certificate.

```datapipe --tls-listen-input localhost:9292 --tls-listen-input-generate-self-signed --tls-listen-input-skip-client-verify```

### UDP Input
UDP input reads data from a UDP address and port.

```datapipe --udp-input 10.80.120.45:11000```

### UDP Multicast Input
UDP multicast input joins an IPv4 multicast group (`224.0.0.0`–`239.255.255.255`) using `A.B.C.D` (ephemeral port) or `A.B.C.D:PORT`.

```datapipe --udp-multicast-input 224.0.120.77:8888```

## Output protocols
* [FILE](#file-output) - write data to a file
* [HTTP](#http-output) - write data to an HTTP URL
* [HTTPS](#https-output) - write data securely to an HTTPS URL
* [Iroh](#iroh-output) - connect outbound and write data to a remote Iroh peer by EndpointId
* [STDOUT](#stdout-output) - write data to stdout (screen or piped input)
* [TCP](#tcp-output) - write data to a TCP address and port
* [TLS](#tls-output) - write data securely to a TLS address and port
* [UDP](#udp-output) - write data to a UDP address and port

**The datapipe command line flags accept only one output of each output type.**  For example, outputs to a file and a UDP address work directly via CLI flags, whereas multiple outputs of the same protocol (up to 64 total sinks) are supported through the CLI `--use-config` flag, the GUI (`datapipe-gui`), and the Library API (`ParametersBuilder`) using TOML `[[outputs]]` arrays.

### File Output
File output requires the path to the output file.

```datapipe --file-output records.dat```

### HTTP Output
HTTP output requires a destination URL and an output rate (`--http-output-rate`) given in milliseconds (`0` sends immediately without delay).  An optional non-empty delimiter byte sequence (`--http-output-delimiter`) and whether to include the delimiter byte sequence with the segment that proceeds it (`--http-output-include-delimiter`) can also be specified.

The default delimiter is the newline character (`\n`).  The default behavior is to include the delimiter sequence with the segment that proceeds it.  Trailing buffered bytes are flushed when the stream closes, and non-2xx server status responses are reported as errors.

Note that inline encryption occurs **BEFORE** output delimiter segmentation, so delimiters in the unencrypted data stream cannot be used to segment the encrypted data stream.

```datapipe --http-output http://www.interesting-potato-facts.com/data-upload --http-output-rate 10000```

### HTTPS Output
HTTPS output requires a destination URL and an output rate (`--https-output-rate`) given in milliseconds (`0` sends immediately without delay).  An optional non-empty delimiter byte sequence (`--https-output-delimiter`) and whether to include the delimiter byte sequence with the segment that proceeds it (`--https-output-include-delimiter`) can also be specified.

The default delimiter is the newline character (`\n`).  The default behavior is to include the delimiter sequence with the segment that proceeds it.  Trailing buffered bytes are flushed when the stream closes, and non-2xx server status responses are reported as errors.

Note that inline encryption occurs **BEFORE** output delimiter segmentation, so delimiters in the unencrypted data stream cannot be used to segment the encrypted data stream.

Custom certificates can be specified (`--https-output-root-certificates`) by giving the path to a non-empty certificates file in PEM bundle or DER format.

A custom certificate revocation list can be specified (`--https-output-certificate-revocation-list`) by giving the path to a non-empty certificate revocation list file in PEM format.  Using a CRL requires custom root certificates (`--https-output-root-certificates`).

HTTPS client identity can be specified (`--https-output-client-identity`) by giving the path to the client's private key and X509 certificate in PEM format.  The private key must be RSA, SEC1 Elliptic Curve, or PKCS#8.

In rare circumstances, hostname validation can be skipped (`--https-output-allow-invalid-hostnames`; requires custom `--https-output-root-certificates` unless `--https-output-allow-invalid-certificates` is also set).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the destination web server.

In rare circumstances, invalid certificates can be accepted (`--https-output-allow-invalid-certificates`).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the destination web server.

```datapipe --https-output https://192.168.15.35:443 --https-output-rate 7000```

### Iroh Output
Iroh output connects outbound over peer-to-peer QUIC (`iroh`) to a listening Iroh endpoint identified by its `EndpointId` (64-character hex or z-base-32 Ed25519 public key) and streams data to it.

```datapipe --file-input send.dat --iroh-output <RECEIVER_ENDPOINT_ID> --iroh-output-addrs 192.168.15.35:11204```

Optional flags for `--iroh-output`:
* `--iroh-output-addrs <ADDRS>`: comma- or whitespace-separated direct IP socket addresses (`ip:port`) for the target Iroh endpoint.
* `--iroh-output-relay-url <URL>`: custom relay server URL (`http://` or `https://`).
* `--iroh-output-alpn <ALPN>`: custom ALPN protocol identifier (defaults to `datapipe/iroh/1`).
* `--iroh-output-secret-key <KEY>`: local 32-byte Ed25519 secret key in 64-character hex or z-base-32 format to authenticate the sender endpoint.
* `--iroh-output-no-relay`: disable default relay servers and address discovery (direct IP transport only).

### STDOUT Output
STDOUT output allows datapipe to send output to the screen or as input to a pipe.

```datapipe --stdout-output | perl fantastic_data_muncher.pl --lean --mean --green```

### TCP Output
TCP output connects and writes data to a TCP address and port.

```datapipe --tcp-output 10.50.70.91:2495```

### TLS Output
TLS output securely writes data to a TLS address and port.

```datapipe --tls-output 10.50.70.90:2288 --tls-output-cert-chain certificates.der --tls-output-client-key client_key.der --tls-output-root-ca ca_root.der```

A custom certificate chain (`--tls-output-cert-chain`) can be specified if needed along with a custom client private key (`--tls-output-client-key`) to ensure client integrity.  If one of these options is used, the other must also be used.  The certificate chain can be in PEM or DER format.  The private key can be a PEM- or DER-encoded PKCS#1, PKCS#8, or SEC1 key.

TLS output uses web Certificate Authority roots by default.  A custom Certificate Authority root can be used (`--tls-output-root-ca`) if wanted, in PEM or DER format.

In rare circumstances, TLS server verification can be skipped (`--tls-output-skip-server-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the destination server.

### UDP Output
UDP output writes data to a UDP address and port.

```datapipe --udp-output 10.80.120.99:2090```


## In-Transit Options
### Encryption
In-line streaming encryption and decryption can provide additional security.  NOTE: for optimal data security, encrypt your data with another encryption system before using `datapipe`.  Then, use datapipe's stream encryption over TLS to send your data.  This will provide three layers of encryption which should help to deter most attackers.

`datapipe` uses XChaCha20-Poly1305 AEAD (`aead-stream`) with a 51-byte symmetric secret (32-byte cipher key + 19-byte stream nonce) and versioned v2 authenticated end-of-stream framing (`MESSAGE_START_V2`) to detect stream truncation and tampering.  Supported key formats are:

* `dpk1:<68 base64url chars>` — full-entropy unpadded base64url key (default output of `--encrypt-generate-key` and the GUI)
* `hex:<102 hex chars>` — full-entropy hexadecimal key
* Legacy 51-byte UTF-8 key — accepted for backward compatibility

#### Encryption
Provide an encryption key (`dpk1:...`, `hex:...`, or legacy 51-byte UTF-8 key):

```datapipe --encrypt dpk1:<68_base64url_characters>```

Have `datapipe` generate a full-entropy encryption key (printed to standard error so stdout pipelines remain clean):

```datapipe --encrypt-generate-key```

#### Decryption
Provide the encryption key used during encryption:

```datapipe --decrypt dpk1:<68_base64url_characters>```

By default, decryption rejects legacy unauthenticated v1 streams that lack authenticated end-of-stream framing.  To decrypt legacy v1 data at your own risk (vulnerable to silent stream truncation), pass `--decrypt-allow-legacy`.

Secret values (`--encrypt`, `--decrypt`, `--iroh-*-secret-key`) are redacted in debug logs, masked by default with a `Show`/`Hide` toggle in the GUI, and saved with owner-only (`0600`) Unix permissions when written to TOML configuration files.

## Metrics & Live Display
`datapipe` tracks streaming transfer statistics in real time, including the elapsed duration, total bytes read, instantaneous read throughput, total bytes written, and write throughput.

### Live Display in CLI
By default, the `datapipe` CLI displays live, updating metrics on standard error:

```text
[datapipe] Elapsed: 00:00:03 | Read: 4.50 MB (1.50 MB/s) | Written: 4.50 MB (1.50 MB/s)
```

Upon pipeline completion, a final summary is printed:

```text
[datapipe] Completed in 3.12s | Read: 4.50 MB (1.44 MB/s) | Written: 4.50 MB (1.44 MB/s)
```

### Disabling Metrics
To disable metrics tracking and live display, provide the `--no-metrics` CLI flag:

```datapipe --file-input source.dat --file-output dest.dat --no-metrics```

When using TOML configuration files, metrics tracking can also be disabled by setting `no_metrics = true`:

```toml
[input]
file_input = "source.dat"

[output]
file_output = "dest.dat"

no_metrics = true
```

## Configuration Files
`datapipe` supports reading, saving, and verifying TOML configuration files. A configuration file must define one input and at least one output destination.

The `--use-config`, `--save-to-config`, and `--verify-config` flags are mutually exclusive.

### Using a Configuration File
Run `datapipe` with parameters defined in a TOML configuration file (preserving multi-output `[[outputs]]` fan-out destinations up to 64 sinks):

```datapipe --use-config /path/to/datapipe.toml```

Other input or output options cannot be combined with `--use-config`, though `--no-metrics` can be passed on the command line to suppress live metrics output.

### Saving Configuration to File
Save given command-line parameters to a TOML configuration file for reuse without starting the pipeline (files containing encryption keys, decryption keys, or Iroh secret keys are written with owner-only `0600` permissions on Unix):

```datapipe --file-input input.dat --file-output output.dat --save-to-config /path/to/datapipe.toml```

### Verifying a Configuration File
Check whether a configuration file is valid without starting the pipeline (exits with status 0 if valid, non-zero if invalid):

```datapipe --verify-config /path/to/datapipe.toml```

### Sample TOML Configuration File
```toml
[input]
file_input = "source.dat"

[output]
file_output = "destination.dat"

[encryption_args]
# Optional: ChaCha20-Poly1305 inline encryption key (dpk1:..., hex:..., or 51-byte legacy UTF-8)
# encryption_key = "dpk1:<68_base64url_characters>"
```

Option tables matching CLI argument groups (such as `[http_input]`, `[https_input]`, `[tls_input]`, `[tls_listen_input]`, `[iroh_input]`, `[iroh_listen_input]`, `[decryption_args]`, `[http_output]`, `[https_output]`, `[tls_output]`, `[iroh_output]`, and `[logging_args]`) can also be specified in TOML files, along with `[[outputs]]` arrays (up to 64 destinations) for `--use-config`, the GUI, or library use.  A configuration file cannot populate both `[output]` and `[[outputs]]`, and HTTP/HTTPS output delimiters cannot be empty.

## Graphical User Interface
`datapipe` includes an optional desktop GUI binary (`datapipe-gui`) built with `egui` / `eframe` and `rfd` file dialogs under the `gui` Cargo feature.

### Launching the GUI
```bash
cargo run --features gui --bin datapipe-gui
```

Or build a release binary:
```bash
cargo build --release --features gui --bin datapipe-gui
```

### GUI Capabilities
* **Material Design 3 Theme**: Tonal Blue styling with a Light / Dark theme toggle in the top app bar.
* **Five-Stage Workflow Tabs**:
  * **Input**: Configure any of the 12 supported input protocols (`File`, `HTTP`, `HTTPS`, `STDIN`, `TCP`, `TCP Listen`, `TLS`, `TLS Listen`, `UDP`, `UDP Multicast`, `Iroh`, and `Iroh Listen`), browse local files, validate non-negative millisecond rates / IPv4 multicast addresses, and generate Ed25519 Iroh secret keys (masked with `Show`/`Hide` and live `EndpointId` derivation for `Iroh Listen`).
  * **Decrypt**: Enable or bypass inline ChaCha20-Poly1305 stream decryption with live key validation (`dpk1:`, `hex:`, or legacy 51-byte UTF-8), masked secret entry with `Show`/`Hide` toggle, and an explicit opt-in checkbox for legacy unauthenticated v1 streams.
  * **Encrypt**: Enable or bypass inline ChaCha20-Poly1305 stream encryption, generate full-entropy `dpk1:` keys, mask/reveal key entry, and copy keys to the clipboard.
  * **Output**: Configure 1 to 64 concurrent fan-out output destinations (`File`, `STDOUT`, `TCP`, `UDP`, `HTTP`, `HTTPS`, `TLS`, and `Iroh`) with file pickers, PEM/DER certificate support, escaped HTTP/HTTPS delimiters (`\n`, `\r`, `\t`, etc.), and masked Iroh secret keys.
  * **Run**: Inspect an animated pipeline topology diagram, review live validation diagnostics, control execution (`Start Datapipe` with duplicate-run prevention, `Stop Datapipe`, `Clear`), monitor live throughput telemetry (`Elapsed Time`, `Bytes Read`, `Bytes Written`, `Read Throughput`, `Write Throughput`), and Load / Save TOML configuration profiles.
* **Clipboard & Text Editing**: All text fields (including masked secret fields) support text selection, standard `Ctrl+X` / `Ctrl+C` / `Ctrl+V` / `Ctrl+A` keyboard shortcuts, and a right-click context menu (`Cut`, `Copy`, `Paste`, `Select All`).

## Program Information
`datapipe` provides command-line parameters to inspect version, copyright, and license information.

### Version
Display the datapipe name, version, and copyright information, then exit:

```datapipe --version```

### License
Display the datapipe license, then exit:

```datapipe --license```

## Library API
`datapipe` can be used as a Rust library in your own projects.  Add it as a dependency in your `Cargo.toml`:

```toml
[dependencies]
datapipe = "0.3"
tokio = { version = "1", features = ["full"] }
```

### Core Concepts

The library API centers around three components:

1. **`ParametersBuilder`** — A builder that assembles the pipeline configuration: one `Reader` (input source), one or more `Writer`s (output destinations), and optional `StreamEncryptor`/`StreamDecryptor` stages (`StreamDecryptor::new` enforces v2 authenticated EOF framing via `finish()`, while `StreamDecryptor::new_with_legacy` optionally permits v1 streams).
2. **`Parameters`** — The validated pipeline configuration produced by `ParametersBuilder::build()`.
3. **`run_datapipe(parameters)`** — The async entry point that supervises pipeline tasks in a `JoinSet`, tracks per-writer retries, flushes and closes all `OutputWriter` sinks on EOF or failure, and returns structured `DatapipeError` results without panicking.

### Basic Usage

```rust,no_run
use datapipe::datapipe_types::DatapipeError;
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::writer::Writer;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), DatapipeError> {
    let reader = FileReader::new(Path::new("input.dat")).await?;
    let writer = FileWriter::new(Path::new("output.dat")).await?;

    let parameters = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()?;

    run_datapipe(parameters).await?;
    Ok(())
}
```

### Available Readers and Writers

All input protocols (`FileReader`, `HttpReader`, `HttpsReader`, `IrohReader`, `IrohListenReader`, `StdinReader`, `TcpReaderWriter`, `TcpListenReader`, `TlsReaderWriter`, `TlsListenReader`, and `UdpReader` for UDP / UDP Multicast) and output protocols (`FileWriter`, `HttpWriter`, `HttpsWriter`, `IrohWriter`, `StdoutWriter`, `TcpReaderWriter`, `TlsReaderWriter`, and `UdpWriter`) are available as library types.  Each concrete type can be converted to the `Reader` or `Writer` enum using `Reader::from()` or `Writer::from()`.  Iroh configuration helpers (`IrohConnectConfig`, `IrohListenConfig`, and parsing/formatting helpers) live in `datapipe::iroh_listen_reader` and `datapipe::iroh_reader_writer`.

### Example Programs

Complete working examples are in the [`examples/`](examples/) directory:

* [`file_copy`](examples/file_copy.rs) — Copy a file using `ParametersBuilder` and `run_datapipe`
* [`encrypted_file_copy`](examples/encrypted_file_copy.rs) — Copy a file with ChaCha20-Poly1305 inline encryption
* [`encrypted_file_decrypt`](examples/encrypted_file_decrypt.rs) — Decrypt a file encrypted by `encrypted_file_copy`
* [`tcp_file_transfer`](examples/tcp_file_transfer.rs) — Transfer a file over a TCP connection on localhost
* [`multi_output`](examples/multi_output.rs) — Fan out a single input to multiple output files and stdout

Run an example with:

```bash
cargo run --example file_copy -- input.dat output.dat
```

# Production Readiness
`datapipe` is currently at **beta maturity and should be used with caution.**  Please report any errors as GitHub Issues.


