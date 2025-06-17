use datapipe::datapipe_types::{DatapipeError, EncryptionKey, generate_random_string};
use datapipe::utilities::{get_unused_port, identical_contents};
use std::path::PathBuf;
use tokio::fs::remove_file;
use tokio::process::Command;

const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";

#[cfg(test)]
fn get_datapipe_binary() -> Result<PathBuf, DatapipeError> {
    let cargo_manifest_dir = std::env::var(CARGO_MANIFEST_DIR)?;
    let datapipe_binary_pathbuf: PathBuf = [&cargo_manifest_dir, "target", "debug", "datapipe"].iter().collect();
    Ok(datapipe_binary_pathbuf)
}

#[cfg(test)]
fn get_test_document() -> Result<PathBuf, DatapipeError> {
    let cargo_manifest_dir = std::env::var(CARGO_MANIFEST_DIR)?;
    let test_document_pathbuf: PathBuf = [&cargo_manifest_dir, "tests", "data", "document.pdf"].iter().collect();
    Ok(test_document_pathbuf)
}

#[cfg(test)]
fn get_temp_dir() -> PathBuf {
    std::env::temp_dir()
}

#[cfg(test)]
async fn run_datapipe(args: Vec<String>) {
    let datapipe_pathbuf = get_datapipe_binary().unwrap();
    let exit_status = Command::new(datapipe_pathbuf)
        .args(&args)
        .spawn()
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(exit_status.success());
}


#[tokio::test]
async fn integration_test_copy_file() {    
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
        output_file_path.to_str().unwrap().to_string()
    ];
    run_datapipe(args).await;
    // verify file copy worked
    assert!(output_file_path.exists());
    // compare the file contents
    assert!(identical_contents(&test_document_pathbuf, &output_file_path).await.unwrap());
    // delete the output file
    remove_file(&output_file_path).await.unwrap();    
}


#[tokio::test]
async fn integration_test_uucp_file() {    
    let test_document_pathbuf = get_test_document().unwrap();
    let temp_dir_pathbuf = get_temp_dir();    
    // put the output file in the temp dir    
    let mut output_file_path = temp_dir_pathbuf.clone();
    let mut random_filename = generate_random_string(16);
    random_filename.push_str(".pdf");
    output_file_path.push(&random_filename);

    // "uucp" a file using two datapipe instances
    let port = get_unused_port().await.unwrap();
    let tcp_address = format!("localhost:{}", port);
    let mut children = Vec::new();
    let args1: Vec<String> = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--tcp-output".to_string(),
        tcp_address.clone(),
        "--keep-logs".to_string(),
    ];
    let args2 = vec![
        "--tcp-listen-input".to_string(),
        tcp_address.clone(),
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
        "--keep-logs".to_string(),
    ];
    children.push(tokio::spawn(run_datapipe(args2)));
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
    assert!(identical_contents(&test_document_pathbuf, &output_file_path).await.unwrap());
    // delete the output file
    remove_file(&output_file_path).await.unwrap();    
    
}

#[tokio::test]
async fn integration_test_scp_file() {    
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
    let tcp_address = format!("localhost:{}", port);
    let mut children = Vec::new();
    let args1: Vec<String> = vec![
        "--file-input".to_string(),
        test_document_pathbuf.to_str().unwrap().to_string(),
        "--encrypt".to_string(),
        encryption_key_string.clone(),
        "--tcp-output".to_string(),
        tcp_address.clone(),
        "--keep-logs".to_string(),
    ];
    let args2 = vec![
        "--tcp-listen-input".to_string(),
        tcp_address.clone(),
        "--decrypt".to_string(),
        encryption_key_string,
        "--file-output".to_string(),
        output_file_path.to_str().unwrap().to_string(),
        "--keep-logs".to_string(),
    ];
    children.push(tokio::spawn(run_datapipe(args2)));
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
    assert!(identical_contents(&test_document_pathbuf, &output_file_path).await.unwrap());
    // delete the output file
    //remove_file(&output_file_path).await.unwrap();    
    
}
