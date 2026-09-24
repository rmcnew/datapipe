/// # File Copy Example
///
/// Demonstrates the simplest use of the datapipe library API: copying a file
/// from one path to another using `ParametersBuilder` and `run_data_pipe`.
///
/// ## Usage
///
/// ```bash
/// cargo run --example file_copy -- <input_file> <output_file>
/// ```
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
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        return ExitCode::FAILURE;
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    // Create the reader and writer
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

    // Build the pipeline parameters
    let parameters = match ParametersBuilder::new()
        .reader(reader)
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
                "File copied: {} -> {}",
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
