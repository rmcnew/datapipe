use datapipe::datapipe_types::{DatapipeError, EncryptionKey, generate_random_string};
use datapipe::utilities::{get_unused_port, identical_contents, start_after_random_delay};
use std::path::PathBuf;
use tokio::fs::remove_file;
use tokio::process::Command;

const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
const RANDOM_DELAY_MAX: u8 = 8;

#[cfg(test)]
fn get_datapipe_binary() -> Result<PathBuf, DatapipeError> {
    let cargo_manifest_dir = std::env::var(CARGO_MANIFEST_DIR)?;
    let datapipe_binary_pathbuf: PathBuf = [&cargo_manifest_dir, "target", "debug", "datapipe"]
        .iter()
        .collect();
    Ok(datapipe_binary_pathbuf)
}

#[cfg(test)]
fn get_test_document() -> Result<PathBuf, DatapipeError> {
    let cargo_manifest_dir = std::env::var(CARGO_MANIFEST_DIR)?;
    let test_document_pathbuf: PathBuf = [&cargo_manifest_dir, "tests", "data", "document.pdf"]
        .iter()
        .collect();
    Ok(test_document_pathbuf)
}

#[cfg(test)]
fn get_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

#[cfg(test)]
async fn run_datapipe_status(args: &[String]) -> std::process::ExitStatus {
    let datapipe_pathbuf = get_datapipe_binary().unwrap();
    Command::new(datapipe_pathbuf)
        .args(args)
        .spawn()
        .unwrap()
        .wait()
        .await
        .unwrap()
}

#[cfg(test)]
async fn run_datapipe_output(args: &[String]) -> std::process::Output {
    let datapipe_pathbuf = get_datapipe_binary().unwrap();
    Command::new(datapipe_pathbuf)
        .args(args)
        .output()
        .await
        .unwrap()
}

#[cfg(test)]
async fn run_datapipe(args: Vec<String>) {
    let exit_status = run_datapipe_status(&args).await;
    assert!(exit_status.success());
}

#[tokio::test]
async fn integration_test_copy_file() {
    start_after_random_delay(RANDOM_DELAY_MAX).await;
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    // put the output file in the temp dir
    let mut output_file_path = temp_dir_pathbuf.clone();
    let mut random_filename = generate_random_string(16);
    random_filename.push_str(".pdf");
    output_file_path.push(&random_filename);

    // copy a file using datapipe
    let args: Vec<String> = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
    ];
    run_datapipe(args).await;
    // verify file copy worked
    assert!(output_file_path.exists());
    // compare the file contents
    assert!(
        identical_contents(&test_document_pathbuf, &output_file_path)
            .await
            .unwrap()
    );
    // delete the output file
    remove_file(&output_file_path).await.unwrap();
}

#[tokio::test]
async fn integration_test_uucp_file() {
    start_after_random_delay(RANDOM_DELAY_MAX).await;
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    // put the output file in the temp dir
    let mut output_file_path = temp_dir_pathbuf.clone();
    let mut random_filename = generate_random_string(16);
    random_filename.push_str(".pdf");
    output_file_path.push(&random_filename);

    // "uucp" a file using two datapipe instances
    let port = get_unused_port().await.unwrap();
    println!("uucp using port {port}");
    let tcp_address = format!("localhost:{}", port);
    let mut children = Vec::new();
    let args1: Vec<String> = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--tcp-output".to_string(),
        tcp_address.clone(),
        //"--keep-logs".to_string(),
    ];
    let args2 = vec![
        "--tcp-listen-input".to_string(),
        tcp_address.clone(),
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
        //"--keep-logs".to_string(),
    ];
    children.push(tokio::spawn(run_datapipe(args2)));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    children.push(tokio::spawn(run_datapipe(args1)));
    for child in children {
        match child.await {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{error}");
            }
        }
    }
    // verify file copy worked
    assert!(output_file_path.exists());
    // compare the file contents
    assert!(
        identical_contents(&test_document_pathbuf, &output_file_path)
            .await
            .unwrap()
    );
    // delete the output file
    remove_file(&output_file_path).await.unwrap();
}

// send file over localhost socket using inline encryption over TCP
#[tokio::test]
async fn integration_test_weak_scp_file() {
    start_after_random_delay(RANDOM_DELAY_MAX).await;
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    // put the output file in the temp dir
    let mut output_file_path = temp_dir_pathbuf.clone();
    let mut random_filename = generate_random_string(16);
    random_filename.push_str(".pdf");
    output_file_path.push(&random_filename);

    // "scp" a file using two datapipe instances
    let encryption_key = EncryptionKey::generate();
    let encryption_key_string = encryption_key.to_string();
    //println!("Encryption key is: {}; it has length {}", encryption_key_string, encryption_key_string.len());
    let port = get_unused_port().await.unwrap();
    println!("scp using port {port}");
    let tcp_address = format!("localhost:{}", port);
    let mut children = Vec::new();
    let args1: Vec<String> = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--encrypt".to_string(),
        encryption_key_string.clone(),
        "--tcp-output".to_string(),
        tcp_address.clone(),
        //"--keep-logs".to_string(),
    ];
    let args2 = vec![
        "--tcp-listen-input".to_string(),
        tcp_address.clone(),
        "--decrypt".to_string(),
        encryption_key_string,
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
        //"--keep-logs".to_string(),
    ];
    children.push(tokio::spawn(run_datapipe(args2)));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    children.push(tokio::spawn(run_datapipe(args1)));
    for child in children {
        match child.await {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{error}");
            }
        }
    }
    // verify file copy worked
    assert!(output_file_path.exists());
    // compare the file contents
    assert!(
        identical_contents(&test_document_pathbuf, &output_file_path)
            .await
            .unwrap()
    );
    // delete the output file
    remove_file(&output_file_path).await.unwrap();
}

// send file over localhost socket using inline encryption over TLS
#[tokio::test]
async fn integration_test_strong_scp_file() {
    start_after_random_delay(RANDOM_DELAY_MAX).await;
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    // put the output file in the temp dir
    let mut output_file_path = temp_dir_pathbuf.clone();
    let mut random_filename = generate_random_string(16);
    random_filename.push_str(".pdf");
    output_file_path.push(&random_filename);

    // "scp" a file using two datapipe instances
    let encryption_key = EncryptionKey::generate();
    let encryption_key_string = encryption_key.to_string();
    //println!("Encryption key is: {}; it has length {}", encryption_key_string, encryption_key_string.len());
    let port = get_unused_port().await.unwrap();
    println!("scp using port {port}");
    let tcp_address = format!("localhost:{}", port);
    let mut children = Vec::new();
    let args1: Vec<String> = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--encrypt".to_string(),
        encryption_key_string.clone(),
        "--tls-output".to_string(),
        tcp_address.clone(),
        "--tls-output-skip-server-verify".to_string(),
        "--keep-logs".to_string(),
    ];
    let args2 = vec![
        "--tls-listen-input".to_string(),
        tcp_address.clone(),
        "--tls-listen-input-generate-self-signed".to_string(),
        "--tls-listen-input-skip-client-verify".to_string(),
        "--decrypt".to_string(),
        encryption_key_string,
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
        "--keep-logs".to_string(),
    ];
    children.push(tokio::spawn(run_datapipe(args2)));
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    children.push(tokio::spawn(run_datapipe(args1)));
    for child in children {
        match child.await {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{error}");
            }
        }
    }
    // verify file copy worked
    assert!(output_file_path.exists());
    // compare the file contents
    assert!(
        identical_contents(&test_document_pathbuf, &output_file_path)
            .await
            .unwrap()
    );
    // delete the output file
    remove_file(&output_file_path).await.unwrap();
}

#[tokio::test]
async fn integration_test_verify_config_valid() {
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    let config_path = temp_dir_pathbuf.join(format!("{}.toml", generate_random_string(16)));
    let output_file_path = temp_dir_pathbuf.join(format!("{}.pdf", generate_random_string(16)));

    let toml_content = format!(
        "[input]\nfile_input = {:?}\n\n[output]\nfile_output = {:?}\n",
        test_document_pathbuf, output_file_path
    );
    tokio::fs::write(&config_path, toml_content).await.unwrap();

    let args = vec![
        "--verify-config".to_string(),
        config_path.to_str().unwrap().to_string(),
    ];
    let status = run_datapipe_status(&args).await;
    assert!(status.success());

    remove_file(&config_path).await.unwrap();
}

#[tokio::test]
async fn integration_test_verify_config_invalid() {
    let temp_dir_pathbuf = get_temp_dir();
    let config_path = temp_dir_pathbuf.join(format!("{}.toml", generate_random_string(16)));

    // Missing output destination
    let toml_content = "[input]\nfile_input = \"some_file.dat\"\n";
    tokio::fs::write(&config_path, toml_content).await.unwrap();

    let args = vec![
        "--verify-config".to_string(),
        config_path.to_str().unwrap().to_string(),
    ];
    let status = run_datapipe_status(&args).await;
    assert!(!status.success());

    remove_file(&config_path).await.unwrap();
}

#[tokio::test]
async fn integration_test_save_to_config() {
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    let config_path = temp_dir_pathbuf.join(format!("{}.toml", generate_random_string(16)));
    let output_file_path = temp_dir_pathbuf.join(format!("{}.pdf", generate_random_string(16)));

    let args = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
        "--save-to-config".to_string(),
        config_path.to_str().unwrap().to_string(),
    ];
    let status = run_datapipe_status(&args).await;
    assert!(status.success());
    assert!(config_path.exists());

    // Verify the saved configuration file is valid
    let verify_args = vec![
        "--verify-config".to_string(),
        config_path.to_str().unwrap().to_string(),
    ];
    let verify_status = run_datapipe_status(&verify_args).await;
    assert!(verify_status.success());

    remove_file(&config_path).await.unwrap();
}

#[tokio::test]
async fn integration_test_use_config_file_copy() {
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();
    let config_path = temp_dir_pathbuf.join(format!("{}.toml", generate_random_string(16)));
    let output_file_path = temp_dir_pathbuf.join(format!("{}.pdf", generate_random_string(16)));

    let toml_content = format!(
        "[input]\nfile_input = {:?}\n\n[output]\nfile_output = {:?}\n",
        test_document_pathbuf, output_file_path
    );
    tokio::fs::write(&config_path, toml_content).await.unwrap();

    let args = vec![
        "--use-config".to_string(),
        config_path.to_str().unwrap().to_string(),
    ];
    run_datapipe(args).await;

    assert!(output_file_path.exists());
    assert!(
        identical_contents(&test_document_pathbuf, &output_file_path)
            .await
            .unwrap()
    );

    remove_file(&config_path).await.unwrap();
    remove_file(&output_file_path).await.unwrap();
}

#[tokio::test]
async fn integration_test_config_mutual_exclusion() {
    let args = vec![
        "--use-config".to_string(),
        "config_a.toml".to_string(),
        "--verify-config".to_string(),
        "config_b.toml".to_string(),
    ];
    let status = run_datapipe_status(&args).await;
    assert!(!status.success());
}

#[tokio::test]
async fn integration_test_version() {
    let args = vec!["--version".to_string()];
    let output = run_datapipe_output(&args).await;
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("datapipe"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    assert!(stdout.contains("Copyright"));
}

#[tokio::test]
async fn integration_test_license() {
    let args = vec!["--license".to_string()];
    let output = run_datapipe_output(&args).await;
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GNU AFFERO GENERAL PUBLIC LICENSE"));
    assert!(stdout.contains("Version 3, 19 November 2007"));
}
