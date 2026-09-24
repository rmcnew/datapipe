/// # Multi-Output Example
///
/// Demonstrates using the datapipe library API to fan out a single input
/// to multiple output destinations simultaneously. In this example, a file
/// is copied to two different output files and stdout at the same time.
///
/// ## Usage
///
/// ```bash
/// cargo run --example multi_output -- <input_file> <output_file_1> <output_file_2>
/// ```
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::stdout_writer::StdoutWriter;
use datapipe::writer::Writer;
use std::path::Path;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <input_file> <output_file_1> <output_file_2>",
            args[0]
        );
        return ExitCode::FAILURE;
    }

    let input_path = Path::new(&args[1]);
    let output_path_1 = Path::new(&args[2]);
    let output_path_2 = Path::new(&args[3]);

    // Create the reader
    let reader = match FileReader::new(input_path).await {
        Ok(r) => Reader::from(r),
        Err(e) => {
            eprintln!("Error opening input file: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Create multiple writers
    let writer_1 = match FileWriter::new(output_path_1).await {
        Ok(w) => Writer::from(w),
        Err(e) => {
            eprintln!("Error opening output file 1: {e}");
            return ExitCode::FAILURE;
        }
    };

    let writer_2 = match FileWriter::new(output_path_2).await {
        Ok(w) => Writer::from(w),
        Err(e) => {
            eprintln!("Error opening output file 2: {e}");
            return ExitCode::FAILURE;
        }
    };

    let writer_stdout = Writer::from(StdoutWriter::new());

    // Build the pipeline with multiple writers using chained .writer() calls
    let parameters = match ParametersBuilder::new()
        .reader(reader)
        .writer(writer_1)
        .writer(writer_2)
        .writer(writer_stdout)
        .build()
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error building parameters: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Run the pipeline — data fans out to all writers simultaneously
    match run_datapipe(parameters).await {
        Ok(()) => {
            println!(
                "Multi-output complete: {} -> {}, {}, and stdout",
                input_path.display(),
                output_path_1.display(),
                output_path_2.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
