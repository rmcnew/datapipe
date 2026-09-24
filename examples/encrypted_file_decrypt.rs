/// # Encrypted File Decrypt Example
///
/// Demonstrates using the datapipe library API to decrypt a file that was
/// encrypted with `encrypted_file_copy` or the `datapipe` CLI `--encrypt` flag.
///
/// ## Usage
///
/// ```bash
/// cargo run --example encrypted_file_decrypt -- <encrypted_file> <output_file> <encryption_key>
/// ```
use datapipe::datapipe_types::EncryptionKey;
use datapipe::encryption::StreamDecryptor;
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::writer::Writer;
use std::path::Path;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <encrypted_file> <output_file> <encryption_key>",
            args[0]
        );
        return ExitCode::FAILURE;
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);
    let key_str = &args[3];

    // Parse the 51-byte encryption key
    let encryption_key = match EncryptionKey::new(key_str) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Invalid encryption key: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Create the decryptor
    let decryptor = match StreamDecryptor::new(encryption_key) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error creating decryptor: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Create reader and writer
    let reader = match FileReader::new(input_path).await {
        Ok(r) => Reader::from(r),
        Err(e) => {
            eprintln!("Error opening encrypted file: {e}");
            return ExitCode::FAILURE;
        }
    };

    let writer = match FileWriter::new(output_path).await {
        Ok(w) => Writer::from(w),
        Err(e) => {
            eprintln!("Error opening output file: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Build the pipeline with decryption enabled
    let parameters = match ParametersBuilder::new()
        .reader(reader)
        .decryptor(decryptor)
        .writer(writer)
        .build()
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error building parameters: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Run the pipeline
    match run_datapipe(parameters).await {
        Ok(()) => {
            println!(
                "Decrypted file written: {} -> {}",
                input_path.display(),
                output_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
