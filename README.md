# datapipe
Stream data from here to there

datapipe is a tool used to stream data from one place to another across a variety of protocols.

A datapipe is configured by selecting one input and one or more outputs.  Data streams from the input to the output(s) until no more input is available.

# Navigation
1. [Input Protocols](#input-protocols)
2. [Output Protocols](#output-protocols)
3. [In Transit Options](#in-transit-options)
4. [Configuration Files](#configuration-files)
5. [Library API](#library-api)
6. [Program Information](#program-information)

## Input protocols
* [FILE](#file-input) - read data from a file
* [HTTP](#http-input) - read data from an HTTP URL
* [HTTPS](#https-input) - read data securely from an HTTPS URL
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
HTTP input requires a URL and an input read rate (`--http-input-rate`) specified in milliseconds.  The input rate specifies how often to read from the URL.  It is expected that the data at the URL will change, thus the need to read repeatedly.  A tool such as [wget](https://www.gnu.org/software/wget/) or [curl](https://curl.se/) should be used for one-time downloads.  Specifying an input rate of `0` will download as often as possible which could negatively impact the target web server's performance.

```datapipe --http-input http://local-weather.org/hourly_forecast --http-input-rate 3600000```

### HTTPS Input
HTTPS input requires a URL and an input read rate (`--https-input-rate`) specified in milliseconds.  The input rate specifies how often to read from the URL.  It is expected that the data at the URL will change, thus the need to read repeatedly.  A tool such as [wget](https://www.gnu.org/software/wget/) or [curl](https://curl.se/) should be used for one-time downloads.  Specifying an input rate of `0` will download as often as possible which could negatively impact the target web server's performance.

```datapipe --https-input https://stock-ticker.net/ABCD --https-input-rate 5000```

Custom certificates can be specified (`--https-input-root-certificates`) by giving the path to certificates file.  The certificates file should be in PEM bundle format.

A custom certificate revocation list can be specified (`--https-input-certificate-revocation-list`) by giving the path to the certificate revocation list file.  The file should be in PEM format.

HTTPS client identity can be specified (`--https-input-client-identity`) by giving the path to the client's private key and X509 certificate in PEM format.  The private key must be RSA, SEC1 Elliptic Curve, or PKCS#8.

In rare circumstances, hostname validation can be skipped (`--https-input-allow-invalid-hostnames`).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source web server.

In rare circumstances, invalid certificates can be accepted (`--https-input-allow-invalid-certificates`).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source web server.

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

A custom certificate chain (`--tls-input-cert-chain`) can be specified if needed along with a custom client private key (`--tls-input-client-key`) to ensure client integrity.  If one of these options is used, the other must also be used.  The certificate chain must be in DER format.  Private key must be DER-encoded PKCS#1, PKCS#8, or SEC1.

TLS input uses web Certificate Authority roots by default.  A custom Certificate Authority root can be used (`--tls-input-root-ca`) if wanted.  The certificate must be in DER format.

In rare circumstances, TLS server verification can be skipped (`--tls-input-skip-server-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source server.

### TLS Listen Input
TLS Listen input opens a TLS port on the local machine to listen and accept a TLS connection.

```datapipe --tls-listen-input localhost:9191 --tls-listen-input-cert-chain certificates.der --tls-listen-input-server-key key.der```

A custom certificate chain (`--tls-listen-input-cert-chain`) must be specified along with a custom server private key (`--tls-listen-input-server-key`) to prove server identity.  The certificate chain must be in DER format.  Private key must be DER-encoded PKCS#1, PKCS#8, or SEC1.

Default behavior is to verify client identity against the certificate chain.  In rare circumstances, TLS client verification can be skipped (`--tls-listen-input-skip-client-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be an authorized client.

For convenience in testing environments, a self-signed server certificate and private key can be generated and used (`--tls-listen-input-generate-self-signed`).  DANGER! This should only be used for testing in controlled environments.  Generating a self-signed certificate and private key mean that clients will not be able to verify the server's identity.  This implies 'skip-client-verify' since certificates presented by the client will not be in the generated self-signed certificate chain.

```datapipe --tls-listen-input localhost:9292 --tls-listen-input-generate-self-signed```

### UDP Input
UDP input reads data from a UDP address and port.

```datapipe --udp-input 10.80.120.45:11000```

### UDP Multicast Input
UDP multicast input reads data from a UDP multicast address and port.

```datapipe --udp-multicast-input 10.80.120.77:8888```

## Output protocols
* [FILE](#file-output) - write data to a file
* [HTTP](#http-output) - write data to an HTTP URL
* [HTTPS](#https-output) - write data securely to an HTTPS URL
* [STDOUT](#stdout-output) - write data to stdout (screen or piped input)
* [TCP](#tcp-output) - write data to a TCP address and port
* [TLS](#tls-output) - write data securely to a TLS address and port
* [UDP](#udp-output) - write data to a UDP address and port

**The current version of the datapipe command line tool accepts only one output of each output type.**  For example, outputs to a file and a UDP address should work fine, but output to two files will not work due to command line parser limitations.

### File Output
File output requires the path to the output file.

```datapipe --file-output records.dat```

### HTTP Output
HTTP output requires a destination URL and an output rate (`--http-output-rate`) given in milliseconds.  An optional delimiter byte sequence (`--http-output-delimiter`) and whether to include the delimiter byte sequence with the segment that proceeds it (`--http-output-include-delimiter`) can also be specified.

The default delimiter is the newline character (`\n`).  The default behavior is to include the delimiter sequence with the segment that proceeds it.

Note that inline encryption occurs **BEFORE** output delimiter segmentation, so delimiters in the unencrypted data stream cannot be used to segment the encrypted data stream.

```datapipe --http-output http://www.interesting-potato-facts.com/data-upload --http-output-rate 10000```

### HTTPS Output
HTTPS output requires a destination URL and an output rate (`--https-output-rate`) given in milliseconds.  An optional delimiter byte sequence (`--https-output-delimiter`) and whether to include the delimiter byte sequence with the segment that proceeds it (`--https-output-include-delimiter`) can also be specified.

The default delimiter is the newline character (`\n`).  The default behavior is to include the delimiter sequence with the segment that proceeds it.

Note that inline encryption occurs **BEFORE** output delimiter segmentation, so delimiters in the unencrypted data stream cannot be used to segment the encrypted data stream.

Custom certificates can be specified (`--https-output-root-certificates`) by giving the path to certificates file.  The certificates file should be in PEM bundle format.

A custom certificate revocation list can be specified (`--https-output-certificate-revocation-list`) by giving the path to the certificate revocation list file.  The file should be in PEM format.

HTTPS client identity can be specified (`--https-output-client-identity`) by giving the path to the client's private key and X509 certificate in PEM format.  The private key must be RSA, SEC1 Elliptic Curve, or PKCS#8.

In rare circumstances, hostname validation can be skipped (`--https-output-allow-invalid-hostnames`).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the destination web server.

In rare circumstances, invalid certificates can be accepted (`--https-output-allow-invalid-certificates`).  DANGER!! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the destination web server.

```datapipe --https-output http://192.168.15.35:443 --https-output-rate 7000```

### STDOUT Output
STDOUT output allows datapipe to send output to the screen or as input to a pipe.

```datapipe --stdout-output | perl fantastic_data_muncher.pl --lean --mean --green```

### TCP Output
TCP output connects and writes data to a TCP address and port.

```datapipe --tcp-output 10.50.70.91:2495```

### TLS Output
TLS output securely writes data to a TLS address and port.

```datapipe --tls-output 10.50.70.90:2288 --tls-output-cert-chain certificates.der --tls-output-client-key client_key.der --tls-output-root-ca ca_root.der```

A custom certificate chain (`--tls-output-cert-chain`) can be specified if needed along with a custom client private key (`--tls-output-client-key`) to ensure client integrity.  If one of these options is used, the other must also be used.  The certificate chain must be in DER format.  Private key must be DER-encoded PKCS#1, PKCS#8, or SEC1.

TLS output uses web Certificate Authority roots by default.  A custom Certificate Authority root can be used (`--tls-output-root-ca`) if wanted.  The certificate must be in DER format.

In rare circumstances, TLS server verification can be skpped (`--tls-output-skip-server-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the destination server.

### UDP Output
UDP output writes data to a UDP address and port.

```datapipe --udp-output 10.80.120.99:2090```


## In-Transit Options
### Encryption
In-line streaming encryption and decryption can provide additional security.  NOTE: for optimal data security, encrypt your data with another encryption system before using `datapipe`.  Then, use datapipe's stream encryption over TLS to send your data.  This will provide three layers of encryption which should help to deter most attackers.

The current streaming encryption requires a symmetric key that is exactly 51 bytes in length and is valid UTF-8.  This allows the key to easily be copied or written down for later use and out-of-band transmission to a receiving party.

The 51-byte UTF-8 key can be provided or automatically generated.

#### Encryption
Provide a 51-byte UTF-8 encryption key:

```datapipe --encrypt T8BRXrN15Xpz0KE2FjiZEYGmPk4IpHQmweh2DXERhx7vU6OIEJx```

Have `datapipe` generate a key:

```datapipe --encrypt-generate-key```

**Note that the generated encryption key will be printed on screen.**

#### Decryption
Provide the 51-byte UTF-8 encryption key used during encryption:

```datapipe --decrypt T8BRXrN15Xpz0KE2FjiZEYGmPk4IpHQmweh2DXERhx7vU6OIEJx```

## Configuration Files
`datapipe` supports reading, saving, and verifying TOML configuration files. A configuration file must define one input and at least one output destination.

The `--use-config`, `--save-to-config`, and `--verify-config` flags are mutually exclusive.

### Using a Configuration File
Run `datapipe` with parameters defined in a TOML configuration file:

```datapipe --use-config /path/to/datapipe.toml```

Note that other input or output options cannot be used with "--use-config".

### Saving Configuration to File
Save given command-line parameters to a TOML configuration file for reuse without starting the pipeline:

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

[encryption]
# Optional: 51-byte ASCII key for ChaCha20-Poly1305 inline encryption
# encrypt = "T8BRXrN15Xpz0KE2FjiZEYGmPk4IpHQmweh2DXERhx7vU6OIEJx"
```

## Program Information
`datapipe` provides command-line parameters to inspect version, copyright, and license information.

### Version
Display the datapipe name, version, and copyright information, then exit:

```datapipe --version```

### License
Display the full text of the AGPL-3.0 license embedded directly within the datapipe binary, then exit:

```datapipe --license```

## Library API
`datapipe` can be used as a Rust library in your own projects.  Add it as a dependency in your `Cargo.toml`:

```toml
[dependencies]
datapipe = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Core Concepts

The library API centers around three components:

1. **`ParametersBuilder`** — A builder that assembles the pipeline configuration: one `Reader` (input source), one or more `Writer`s (output destinations), and optional `StreamEncryptor`/`StreamDecryptor` stages.
2. **`Parameters`** — The validated pipeline configuration produced by `ParametersBuilder::build()`.
3. **`run_datapipe(parameters)`** — The async entry point that spawns the pipeline tasks and runs data from input to output(s).

### Basic Usage

```rust,no_run
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::writer::Writer;
use std::path::Path;

#[tokio::main]
async fn main() {
    let reader = FileReader::new(Path::new("input.dat")).await.unwrap();
    let writer = FileWriter::new(Path::new("output.dat")).await.unwrap();

    let parameters = ParametersBuilder::new()
        .reader(Reader::from(reader))
        .writer(Writer::from(writer))
        .build()
        .unwrap();

    run_datapipe(parameters).await;
}
```

### Available Readers and Writers

All input protocols (File, TCP, TCP Listen, TLS, TLS Listen, UDP, HTTP, HTTPS, Stdin) and output protocols (File, TCP, TLS, UDP, HTTP, HTTPS, Stdout) are available as library types.  Each concrete type can be converted to the `Reader` or `Writer` enum using `Reader::from()` or `Writer::from()`.

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


