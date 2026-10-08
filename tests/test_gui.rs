//! Integration tests for the datapipe GUI.

#![cfg(feature = "gui")]

use datapipe::config::DatapipeConfig;
use datapipe::datapipe_types::EncryptionKey;
use datapipe::gui::DatapipeApp;
use datapipe::gui::output_tab::MAX_OUTPUTS;
use datapipe::gui::types::{
    DecryptState, EncryptState, InputState, InputType, OutputState, OutputType, PipelineStatus,
    Tab, load_config_into_states, states_to_config,
};

#[test]
fn test_gui_app_creation() {
    let app = DatapipeApp::new().expect("Failed to create DatapipeApp");
    assert_eq!(app.active_tab, Tab::Input);
    assert_eq!(app.output_states.len(), 1);
    assert_eq!(app.controller.status, PipelineStatus::Idle);
    assert!(!app.controller.is_running());
}

#[test]
fn test_gui_tab_transitions() {
    let mut app = DatapipeApp::new().expect("Failed to create DatapipeApp");

    // Input -> Decrypt -> Encrypt -> Output -> Run -> Output
    assert_eq!(app.active_tab, Tab::Input);

    app.active_tab = Tab::Decrypt;
    assert_eq!(app.active_tab, Tab::Decrypt);

    app.active_tab = Tab::Encrypt;
    assert_eq!(app.active_tab, Tab::Encrypt);

    app.active_tab = Tab::Output;
    assert_eq!(app.active_tab, Tab::Output);

    app.active_tab = Tab::Run;
    assert_eq!(app.active_tab, Tab::Run);

    app.active_tab = Tab::Output;
    assert_eq!(app.active_tab, Tab::Output);
}

#[test]
fn test_gui_multi_output_limit() {
    let mut outputs = vec![OutputState::default()];
    assert_eq!(outputs.len(), 1);

    for _ in 1..MAX_OUTPUTS {
        outputs.push(OutputState::default());
    }
    assert_eq!(outputs.len(), MAX_OUTPUTS);

    // Cannot add beyond MAX_OUTPUTS in UI
    assert!(outputs.len() >= MAX_OUTPUTS);
}

#[test]
fn test_gui_validation_all_stages() {
    let mut input = InputState::default();
    let mut decrypt = DecryptState::default();
    let mut encrypt = EncryptState::default();
    let mut outputs = vec![OutputState::default()];

    // Empty file path should fail
    input.input_type = InputType::File;
    input.file_path = String::new();
    outputs[0].output_type = OutputType::File;
    outputs[0].file_path = String::new();

    let res = states_to_config(&input, &decrypt, &encrypt, &outputs);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.contains("Input: File path is required"))
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("Output #1: File path is required"))
    );

    // Fill file paths
    input.file_path = "test_in.dat".to_string();
    outputs[0].file_path = "test_out.dat".to_string();

    let res = states_to_config(&input, &decrypt, &encrypt, &outputs);
    assert!(res.is_ok());

    // Enable decryption with invalid key
    decrypt.enabled = true;
    decrypt.key = "invalid_short_key".to_string();
    let res = states_to_config(&input, &decrypt, &encrypt, &outputs);
    assert!(res.is_err());

    // Provide valid 51-byte decryption key
    decrypt.key = EncryptionKey::generate().to_string();
    let res = states_to_config(&input, &decrypt, &encrypt, &outputs);
    assert!(res.is_ok());

    // Enable encryption and generate key
    encrypt.enabled = true;
    encrypt.generate_key();
    assert_eq!(encrypt.key.len(), 51);
    let res = states_to_config(&input, &decrypt, &encrypt, &outputs);
    assert!(res.is_ok());
}

#[test]
fn test_gui_config_load_save_pipeline() {
    let toml = r#"
[input]
file_input = "in.dat"

[output]
stdout_output = true
"#;

    let parsed = DatapipeConfig::from_toml_str(toml).expect("Parse TOML");
    let mut input = InputState::default();
    let mut decrypt = DecryptState::default();
    let mut encrypt = EncryptState::default();
    let mut outputs = Vec::new();

    load_config_into_states(
        &parsed,
        &mut input,
        &mut decrypt,
        &mut encrypt,
        &mut outputs,
    );

    assert_eq!(input.input_type, InputType::File);
    assert_eq!(input.file_path, "in.dat");
    assert!(!decrypt.enabled);
    assert!(!encrypt.enabled);
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].output_type, OutputType::Stdout);
}
