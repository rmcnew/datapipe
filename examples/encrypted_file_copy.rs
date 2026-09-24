/// # Encrypted File Copy Example
///
/// Demonstrates using the datapipe library API to copy a file with inline
/// stream encryption (ChaCha20-Poly1305). The encrypted output can be
/// decrypted using the `encrypted_file_decrypt` example or the `datapipe`
/// CLI with the `--decrypt` flag.
///
/// A new encryption key is generated automatically and printed to the console.
///
/// ## Usage
///
/// ```bash
/// cargo run --example encrypted_file_copy -- <input_file> <encrypted_output_file>
/// ```
use datapipe::datapipe_types::EncryptionKey;
use datapipe::encryption::StreamEncryptor;
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
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <encrypted_output_file>", args[0]);
        return ExitCode::FAILURE;
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    // Generate a new encryption key
    let encryption_key = EncryptionKey::generate();
    println!("Generated encryption key: {encryption_key}");
    println!("Save this key to decrypt the file later.");

    // Create the encryptor
    let encryptor = match StreamEncryptor::new(encryption_key) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error creating encryptor: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Create reader and writer
    let reader = match FileReader::new(input_path).await {
        Ok(r) => Reader::from(r),
        Err(e) => {
            eprintln!("Error opening input file: {e}");
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

    // Build the pipeline with encryption enabled
    let parameters = match ParametersBuilder::new()
        .reader(reader)
        .encryptor(encryptor)
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
                "Encrypted file written: {} -> {}",
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
