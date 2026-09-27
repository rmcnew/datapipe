//! Integration tests for metrics tracking and live metrics display.

mod common;

use common::{TempFileGuard, run_cli_output};
use datapipe::config::DatapipeConfig;
use datapipe::datapipe_types::DatapipeError;
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::metrics::{ByteMetrics, DatapipeMetrics, ElapsedTime, ElaspedTime};
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::writer::Writer;
use std::fs;
use std::time::{Duration, Instant, SystemTime};

// =========================================================================
// 1. Library API Metrics Tests
// =========================================================================

#[tokio::test]
async fn test_library_metrics_file_to_file() -> Result<(), DatapipeError> {
    let payload = b"Hello, datapipe metrics tracking world! Testing 1 2 3.";
    let input_file = TempFileGuard::with_content("txt", payload);
    let output_file = TempFileGuard::new("txt");

    let metrics = DatapipeMetrics::new();

    let reader = Reader::from(FileReader::new(&input_file.path).await?);
    let writer = Writer::from(FileWriter::new(&output_file.path).await?);

    let parameters = ParametersBuilder::new()
        .reader(reader)
        .writer(writer)
        .metrics(metrics.clone())
        .build()?;

    run_datapipe(parameters).await?;

    let output_bytes =
        fs::read(&output_file.path).map_err(|e| DatapipeError::InputOutputError(e.to_string()))?;
    assert_eq!(output_bytes, payload);

    assert_eq!(metrics.read_bytes(), payload.len() as u64);
    assert_eq!(metrics.written_bytes(), payload.len() as u64);
    assert!(metrics.elapsed().as_nanos() > 0);
    assert!(metrics.read_rate_bytes_per_sec() >= 0.0);
    assert!(metrics.write_rate_bytes_per_sec() >= 0.0);

    let summary = metrics.format_summary();
    assert!(summary.contains("[datapipe] Completed in"));
    assert!(summary.contains("Read:"));
    assert!(summary.contains("Written:"));

    Ok(())
}

#[tokio::test]
async fn test_library_metrics_fan_out() -> Result<(), DatapipeError> {
    let payload = b"Multi-output fan-out metrics tracking verification payload";
    let input_file = TempFileGuard::with_content("txt", payload);
    let out1 = TempFileGuard::new("txt");
    let out2 = TempFileGuard::new("txt");

    let metrics = DatapipeMetrics::new();

    let reader = Reader::from(FileReader::new(&input_file.path).await?);
    let writer1 = Writer::from(FileWriter::new(&out1.path).await?);
    let writer2 = Writer::from(FileWriter::new(&out2.path).await?);

    let parameters = ParametersBuilder::new()
        .reader(reader)
        .writer(writer1)
        .writer(writer2)
        .metrics(metrics.clone())
        .build()?;

    run_datapipe(parameters).await?;

    assert_eq!(metrics.read_bytes(), payload.len() as u64);
    // Two writers both write the payload, so total written bytes should be 2x payload.len()
    assert_eq!(metrics.written_bytes(), (payload.len() * 2) as u64);

    Ok(())
}

#[tokio::test]
async fn test_elasped_time_struct() {
    let now_system = SystemTime::now();
    let now_instant = Instant::now();
    let timer = ElaspedTime::with_start(now_system, now_instant);

    assert_eq!(timer.start_time(), now_system);
    assert_eq!(timer.start_instant(), now_instant);
    assert!(timer.elapsed_secs() >= 0.0);

    let formatted = timer.format_elapsed();
    assert!(formatted.contains(':'));
    assert_eq!(timer.to_string(), formatted);

    // Verify alias ElapsedTime behaves identically
    let alias_timer = ElapsedTime::new();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(alias_timer.elapsed().as_millis() >= 10);
}

#[test]
fn test_byte_metrics_struct() {
    let m = ByteMetrics::new();
    assert_eq!(m.bytes(), 0);
    assert_eq!(m.human_readable(), "0 B");

    m.add_bytes(2048);
    assert_eq!(m.bytes(), 2048);
    assert_eq!(m.human_readable(), "2.00 KB");

    m.reset();
    assert_eq!(m.bytes(), 0);

    let m_init = ByteMetrics::with_bytes(1_048_576);
    assert_eq!(m_init.bytes(), 1_048_576);
    assert_eq!(m_init.to_string(), "1.00 MB");
}

// =========================================================================
// 2. CLI Live Metrics & --no-metrics Tests
// =========================================================================

#[tokio::test]
async fn test_cli_live_metrics_display_enabled_by_default() {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("pdf");

    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
    ];

    let output = run_cli_output(&args).await;
    assert!(output.status.success());

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_str.contains("[datapipe]"),
        "Expected stderr to contain live metrics display '[datapipe]', got: {stderr_str}"
    );
    assert!(
        stderr_str.contains("Read:"),
        "Expected stderr to contain 'Read:', got: {stderr_str}"
    );
    assert!(
        stderr_str.contains("Written:"),
        "Expected stderr to contain 'Written:', got: {stderr_str}"
    );
}

#[tokio::test]
async fn test_cli_metrics_disabled_with_no_metrics_flag() {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("pdf");

    let args = vec![
        "--file-input".to_string(),
        test_doc.to_str().unwrap().to_string(),
        "--file-output".to_string(),
        temp_out.path.to_str().unwrap().to_string(),
        "--no-metrics".to_string(),
    ];

    let output = run_cli_output(&args).await;
    assert!(output.status.success());

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr_str.contains("[datapipe]"),
        "Expected stderr to NOT contain '[datapipe]' when --no-metrics is set, got: {stderr_str}"
    );
}

#[tokio::test]
async fn test_cli_metrics_disabled_with_config_file() -> Result<(), DatapipeError> {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("pdf");
    let config_file = TempFileGuard::new("toml");

    let mut config = DatapipeConfig::new();
    config.input.file_input = Some(test_doc);
    config.output.file_output = Some(temp_out.path.clone());
    config.no_metrics = true;
    config.save_to_file(&config_file.path)?;

    let args = vec![
        "--use-config".to_string(),
        config_file.path.to_str().unwrap().to_string(),
    ];

    let output = run_cli_output(&args).await;
    assert!(output.status.success());

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr_str.contains("[datapipe]"),
        "Expected stderr to NOT contain '[datapipe]' with no_metrics in config, got: {stderr_str}"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_metrics_disabled_with_use_config_and_cli_flag() -> Result<(), DatapipeError> {
    let test_doc = common::get_test_document();
    let temp_out = TempFileGuard::new("pdf");
    let config_file = TempFileGuard::new("toml");

    // Config does NOT set no_metrics (default is false)
    let mut config = DatapipeConfig::new();
    config.input.file_input = Some(test_doc);
    config.output.file_output = Some(temp_out.path.clone());
    config.save_to_file(&config_file.path)?;

    // Pass --no-metrics on CLI alongside --use-config
    let args = vec![
        "--use-config".to_string(),
        config_file.path.to_str().unwrap().to_string(),
        "--no-metrics".to_string(),
    ];

    let output = run_cli_output(&args).await;
    assert!(output.status.success());

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr_str.contains("[datapipe]"),
        "Expected stderr to NOT contain '[datapipe]' when CLI overrides with --no-metrics, got: {stderr_str}"
    );

    Ok(())
}
