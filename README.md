# datapipe
Stream data from here to there

datapipe is a tool used to stream data from one place to another across a variety of protocols.

A datapipe is configured by selecting one input and one or more outputs.  Data streams from the input to the output(s) until no more input is available.

## Input protocols
* [FILE](#file-input) - read data from a file
* [HTTP](#http-input) - read data from an HTTP URL
* [HTTPS](#https-input) - read data securely from an HTTPS URL
* [STDIN](#stdin-input) - read data from stdin (keyboard or piped output)
* [TCP](#tcp-input) - read data from a TCP address and port
* [TCP Listen](#tcp-listen-input) - open a local port to listen and receive data using a TCP connection
* [TLS](#tls-input) - read data securely from a TLS address and port
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

In rare circumstances, TLS server verification can be skpped (`--tls-input-skip-server-verify`). DANGER! This should only be used for testing in controlled environments.  Misuse can allow an attacker to pretend to be the source server.

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
### Encryption Options
In-line streaming encryption and decryption can provide additional security.  

The current streaming encryption requires a symmetric key that is exactly 51 bytes in length and is valid UTF-8.  This allows the key to easily be copied or written down for later use or out-of-band transmission to a receiving party.  

The 51-byte UTF-8 key can be provided or automatically generated.

#### Encryption
Provide a 51-byte UTF-8 encryption key:

```datapipe --encrypt T8BRXrN15Xpz0KE2FjiZEYGmPk4IpHQmweh2DXERhx7vU6OIEJx```

Have `datapipe` generate a key:

```datapipe --encrypt-generate-key``

**Note that the generated encryption key will be printed on screen.**

#### Decryption
Provide the 51-byte UTF-8 encryption key used during encryption:

```datapipe --decrypt T8BRXrN15Xpz0KE2FjiZEYGmPk4IpHQmweh2DXERhx7vU6OIEJx```

# Production Readiness
datapipe is currently at **early alpha maturity and should not be used for production work.**

Work is underway to build a comprehensive testing suite to find and fix bugs and unexpected behavior.

crates.io:  https://crates.io/crates/datapipe