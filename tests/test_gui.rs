//! Integration tests for the datapipe Material Design 3 GUI.

#![cfg(feature = "gui")]

use datapipe::config::DatapipeConfig;
use datapipe::datapipe_types::EncryptionKey;
use datapipe::gui::DatapipeApp;
use datapipe::gui::output_tab::MAX_OUTPUTS;
use datapipe::gui::style::{M3Palette, ThemeMode, apply_theme, m3_text_field};
use datapipe::gui::types::{
    DecryptState, EncryptState, InputState, InputType, OutputState, OutputType, PipelineStatus,
    Tab, load_config_into_states, states_to_config,
};

fn assert_shape_text_is_ascii(shape: &egui::Shape) {
    match shape {
        egui::Shape::Text(text_shape) => {
            let text = text_shape.galley.text();
            assert!(
                text.is_ascii(),
                "Rendered GUI text contains non-ASCII characters that may render as missing glyph boxes: {text:?}"
            );
        }
        egui::Shape::Vec(nested) => {
            for inner in nested {
                assert_shape_text_is_ascii(inner);
            }
        }
        _ => {}
    }
}

fn assert_all_rendered_text_is_ascii(shapes: &[egui::epaint::ClippedShape]) {
    for clipped in shapes {
        assert_shape_text_is_ascii(&clipped.shape);
    }
}

fn render_frame_and_assert_ascii(ctx: &egui::Context, app: &mut DatapipeApp) {
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render(ui);
    });
    assert_all_rendered_text_is_ascii(&output.shapes);
    output.textures_delta.clear();
}

#[test]
fn test_gui_app_creation() {
    let app = DatapipeApp::new().expect("Failed to create DatapipeApp");
    assert_eq!(app.active_tab, Tab::Input);
    assert_eq!(app.theme_mode, ThemeMode::Light);
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

#[test]
fn test_gui_m3_theme_modes_and_headless_rendering_without_missing_glyphs() {
    let mut app = DatapipeApp::new().expect("Failed to create DatapipeApp");
    let ctx = egui::Context::default();

    apply_theme(&ctx, ThemeMode::Light);
    assert_eq!(
        ctx.global_style().visuals.panel_fill,
        M3Palette::light().surface
    );

    // Exercise full M3 UI rendering across both Light and Dark modes and all tabs/protocols,
    // asserting every rendered text galley uses universally supported ASCII characters.
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        app.theme_mode = mode;

        // 1. Input tab across every InputType
        app.active_tab = Tab::Input;
        for &it in InputType::all() {
            app.input_state.input_type = it;
            render_frame_and_assert_ascii(&ctx, &mut app);
        }

        // 2. Decrypt tab (disabled, invalid key, valid key)
        app.active_tab = Tab::Decrypt;
        for (enabled, key) in [
            (false, String::new()),
            (true, "short".to_string()),
            (true, EncryptionKey::generate().to_string()),
        ] {
            app.decrypt_state.enabled = enabled;
            app.decrypt_state.key = key;
            render_frame_and_assert_ascii(&ctx, &mut app);
        }

        // 3. Encrypt tab (disabled & enabled with generated key)
        app.active_tab = Tab::Encrypt;
        app.encrypt_state.enabled = false;
        render_frame_and_assert_ascii(&ctx, &mut app);

        app.encrypt_state.enabled = true;
        app.encrypt_state.generate_key();
        render_frame_and_assert_ascii(&ctx, &mut app);

        // 4. Output tab across every OutputType + multi-sink card layout
        app.active_tab = Tab::Output;
        app.output_states = vec![OutputState::default(), OutputState::default()];
        for &ot in OutputType::all() {
            app.output_states[0].output_type = ot;
            render_frame_and_assert_ascii(&ctx, &mut app);
        }

        // 5. Run tab across all PipelineStatus states
        app.active_tab = Tab::Run;
        app.last_message = Some(("Configuration validated.".to_string(), false));
        for status in [
            PipelineStatus::Idle,
            PipelineStatus::Ready,
            PipelineStatus::Running,
            PipelineStatus::Stopped,
            PipelineStatus::Completed,
            PipelineStatus::Error("Simulated connection failure".to_string()),
        ] {
            app.controller.status = status;
            render_frame_and_assert_ascii(&ctx, &mut app);
        }
    }

    assert!(ctx.global_style().visuals.dark_mode);
    assert_eq!(
        ctx.global_style().visuals.panel_fill,
        M3Palette::dark().surface
    );
}

fn find_shape_text_pos(shape: &egui::Shape, needle: &str) -> Option<egui::Pos2> {
    match shape {
        egui::Shape::Text(text_shape) => {
            if text_shape.galley.text() == needle {
                Some(text_shape.pos)
            } else {
                None
            }
        }
        egui::Shape::Vec(nested) => nested.iter().find_map(|s| find_shape_text_pos(s, needle)),
        _ => None,
    }
}

fn find_rendered_text_pos(
    shapes: &[egui::epaint::ClippedShape],
    needle: &str,
) -> Option<egui::Pos2> {
    shapes
        .iter()
        .find_map(|clipped| find_shape_text_pos(&clipped.shape, needle))
}

fn extract_copied_text(output: &egui::FullOutput) -> Option<String> {
    output
        .platform_output
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

fn has_request_paste(output: &egui::FullOutput) -> bool {
    output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|vp| {
            vp.commands
                .iter()
                .any(|cmd| matches!(cmd, egui::ViewportCommand::RequestPaste))
        })
}

fn make_input_with_events(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(840.0, 600.0),
        )),
        focused: true,
        events,
        ..Default::default()
    }
}

#[test]
fn test_m3_text_field_context_menu_preserves_selection_and_executes_cut_copy_paste() {
    let ctx = egui::Context::default();
    apply_theme(&ctx, ThemeMode::Light);

    let mut value = String::from("http://example.com:8080/stream");
    let mut field_id = egui::Id::NULL;
    let mut field_rect = egui::Rect::ZERO;

    let run_field = |ctx: &egui::Context,
                     value: &mut String,
                     events: Vec<egui::Event>,
                     out_id: &mut egui::Id,
                     out_rect: &mut egui::Rect|
     -> egui::FullOutput {
        let mut output = ctx.run_ui(make_input_with_events(events), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let resp = m3_text_field(ui, value, "http://host:port", 320.0);
                *out_id = resp.id;
                *out_rect = resp.rect;
            });
        });
        assert_all_rendered_text_is_ascii(&output.shapes);
        output.textures_delta.clear();
        output
    };

    // Initial render
    let _ = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);

    // Focus the field and highlight "example.com" (chars 7..18)
    ctx.memory_mut(|mem| mem.request_focus(field_id));
    let mut state = egui::text_edit::TextEditState::load(&ctx, field_id).unwrap_or_default();
    let highlight =
        egui::text::CCursorRange::two(egui::text::CCursor::new(7), egui::text::CCursor::new(18));
    state.cursor.set_char_range(Some(highlight));
    state.store(&ctx, field_id);

    // Right-click (secondary press + release across two frames) on the text field
    let click_pos = field_rect.center();
    let _ = run_field(
        &ctx,
        &mut value,
        vec![
            egui::Event::PointerMoved(click_pos),
            egui::Event::PointerButton {
                pos: click_pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut field_id,
        &mut field_rect,
    );

    // Verify right-click press did NOT erase the highlighted selection!
    let after_press = egui::text_edit::TextEditState::load(&ctx, field_id)
        .and_then(|s| s.cursor.char_range())
        .expect("Cursor range should exist");
    assert_eq!(after_press, highlight);

    // Release secondary button to open context menu, then run one settle frame after sizing pass
    let _ = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        &mut field_id,
        &mut field_rect,
    );
    let menu_output = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);

    assert!(
        egui::Popup::is_any_open(&ctx),
        "Context menu should be open"
    );
    let copy_pos = find_rendered_text_pos(&menu_output.shapes, "Copy")
        .expect("Context menu must render 'Copy'")
        + egui::vec2(8.0, 5.0);
    assert!(find_rendered_text_pos(&menu_output.shapes, "Cut").is_some());
    assert!(find_rendered_text_pos(&menu_output.shapes, "Paste").is_some());
    assert!(find_rendered_text_pos(&menu_output.shapes, "Select All").is_some());

    // 1. Click "Copy" in context menu -> copies "example.com" without mutating value
    let _ = run_field(
        &ctx,
        &mut value,
        vec![
            egui::Event::PointerMoved(copy_pos),
            egui::Event::PointerButton {
                pos: copy_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut field_id,
        &mut field_rect,
    );
    let copy_click_out = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::PointerButton {
            pos: copy_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        &mut field_id,
        &mut field_rect,
    );
    assert_eq!(
        extract_copied_text(&copy_click_out).as_deref(),
        Some("example.com")
    );
    assert_eq!(value, "http://example.com:8080/stream");

    // Run the requested repaint frame so the closed popup Area unmounts before the next click
    let _ = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);

    // 2. Re-open context menu with "example.com" still selected and click "Cut"
    let _ = run_field(
        &ctx,
        &mut value,
        vec![
            egui::Event::PointerMoved(click_pos),
            egui::Event::PointerButton {
                pos: click_pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut field_id,
        &mut field_rect,
    );
    let _ = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        &mut field_id,
        &mut field_rect,
    );
    let menu_output = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);
    let cut_pos = find_rendered_text_pos(&menu_output.shapes, "Cut")
        .expect("Context menu must render 'Cut'")
        + egui::vec2(8.0, 5.0);

    let _ = run_field(
        &ctx,
        &mut value,
        vec![
            egui::Event::PointerMoved(cut_pos),
            egui::Event::PointerButton {
                pos: cut_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut field_id,
        &mut field_rect,
    );
    let cut_click_out = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::PointerButton {
            pos: cut_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        &mut field_id,
        &mut field_rect,
    );
    assert_eq!(
        extract_copied_text(&cut_click_out).as_deref(),
        Some("example.com")
    );
    assert_eq!(value, "http://:8080/stream");

    // Run the requested repaint frame so the closed popup Area unmounts before the next click
    let _ = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);

    // 3. Highlight ":8080" (chars 7..12), right-click, and click "Paste" ->
    //    verifies ViewportCommand::RequestPaste + multi-frame eframe paste delivery
    let mut state = egui::text_edit::TextEditState::load(&ctx, field_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(7),
            egui::text::CCursor::new(12),
        )));
    state.store(&ctx, field_id);

    let _ = run_field(
        &ctx,
        &mut value,
        vec![
            egui::Event::PointerMoved(click_pos),
            egui::Event::PointerButton {
                pos: click_pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut field_id,
        &mut field_rect,
    );
    let _ = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        &mut field_id,
        &mut field_rect,
    );
    let menu_output = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);
    let paste_pos = find_rendered_text_pos(&menu_output.shapes, "Paste")
        .expect("Context menu must render 'Paste'")
        + egui::vec2(8.0, 5.0);

    let _ = run_field(
        &ctx,
        &mut value,
        vec![
            egui::Event::PointerMoved(paste_pos),
            egui::Event::PointerButton {
                pos: paste_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        &mut field_id,
        &mut field_rect,
    );
    let paste_click_out = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::PointerButton {
            pos: paste_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        &mut field_id,
        &mut field_rect,
    );
    assert!(
        has_request_paste(&paste_click_out),
        "Clicking Paste must emit ViewportCommand::RequestPaste"
    );

    // Simulate eframe's 2-frame ActionRequested::Paste -> Event::Paste delivery:
    // Frame N+1 runs before ActionRequested::Paste is drained at post-render,
    // Frame N+2 delivers Event::Paste("localhost:9090").
    let _ = run_field(&ctx, &mut value, vec![], &mut field_id, &mut field_rect);
    let _ = run_field(
        &ctx,
        &mut value,
        vec![egui::Event::Paste("localhost:9090".to_string())],
        &mut field_id,
        &mut field_rect,
    );
    assert_eq!(value, "http://localhost:9090/stream");
}

#[test]
fn test_gui_cut_copy_paste_keyboard_shortcuts_across_all_tabs() {
    let mut app = DatapipeApp::new().expect("Failed to create DatapipeApp");
    let ctx = egui::Context::default();
    apply_theme(&ctx, ThemeMode::Light);

    let run_app = |ctx: &egui::Context,
                   app: &mut DatapipeApp,
                   events: Vec<egui::Event>|
     -> egui::FullOutput {
        let mut output = ctx.run_ui(make_input_with_events(events), |ui| {
            app.render(ui);
        });
        assert_all_rendered_text_is_ascii(&output.shapes);
        output.textures_delta.clear();
        output
    };

    let click_at = |ctx: &egui::Context, app: &mut DatapipeApp, pos: egui::Pos2| {
        let _ = run_app(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let _ = run_app(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    };

    let ctrl_mods = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    let raw_ctrl_only = egui::Modifiers {
        ctrl: true,
        command: false,
        ..Default::default()
    };

    // 1. Input Tab (TCP address): click field, paste via Event::Paste (Ctrl+V), select all (Ctrl+A), copy (Ctrl+C), cut (Ctrl+X)
    app.active_tab = Tab::Input;
    app.input_state.input_type = InputType::Tcp;
    app.input_state.tcp_address.clear();
    let out = run_app(&ctx, &mut app, vec![]);
    let tcp_pos = find_rendered_text_pos(&out.shapes, "127.0.0.1:9000")
        .expect("TCP hint text should be rendered")
        + egui::vec2(12.0, 6.0);

    click_at(&ctx, &mut app, tcp_pos);
    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("10.0.0.42:7000".to_string())],
    );
    assert_eq!(app.input_state.tcp_address, "10.0.0.42:7000");

    // Select all via Ctrl+A and copy via Event::Copy (Ctrl+C)
    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::A,
            physical_key: Some(egui::Key::A),
            pressed: true,
            repeat: false,
            modifiers: ctrl_mods,
        }],
    );
    let copy_out = run_app(&ctx, &mut app, vec![egui::Event::Copy]);
    assert_eq!(
        extract_copied_text(&copy_out).as_deref(),
        Some("10.0.0.42:7000")
    );

    // Cut via raw Ctrl+X Event::Key fallback
    let cut_out = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::X,
            physical_key: Some(egui::Key::X),
            pressed: true,
            repeat: false,
            modifiers: raw_ctrl_only,
        }],
    );
    assert_eq!(
        extract_copied_text(&cut_out).as_deref(),
        Some("10.0.0.42:7000")
    );
    assert!(app.input_state.tcp_address.is_empty());

    // 2. Decrypt Tab (51-byte monospace key field)
    app.active_tab = Tab::Decrypt;
    app.decrypt_state.enabled = true;
    app.decrypt_state.key.clear();
    let out = run_app(&ctx, &mut app, vec![]);
    let decrypt_pos =
        find_rendered_text_pos(&out.shapes, "Enter 51-character UTF-8 decryption key...")
            .expect("Decrypt key hint should be rendered")
            + egui::vec2(16.0, 6.0);

    let generated_key = EncryptionKey::generate().to_string();
    click_at(&ctx, &mut app, decrypt_pos);
    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Paste(generated_key.clone())],
    );
    assert_eq!(app.decrypt_state.key, generated_key);

    // 3. Encrypt Tab (51-byte monospace key field): Select All + Ctrl+C copies the key
    app.active_tab = Tab::Encrypt;
    app.encrypt_state.enabled = true;
    app.encrypt_state.key.clear();
    let out = run_app(&ctx, &mut app, vec![]);
    let encrypt_pos =
        find_rendered_text_pos(&out.shapes, "Enter or generate a 51-character UTF-8 key...")
            .expect("Encrypt key hint should be rendered")
            + egui::vec2(16.0, 6.0);

    click_at(&ctx, &mut app, encrypt_pos);
    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Paste(generated_key.clone())],
    );
    assert_eq!(app.encrypt_state.key, generated_key);

    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::A,
            physical_key: Some(egui::Key::A),
            pressed: true,
            repeat: false,
            modifiers: raw_ctrl_only,
        }],
    );
    let copy_out = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::C,
            physical_key: Some(egui::Key::C),
            pressed: true,
            repeat: false,
            modifiers: raw_ctrl_only,
        }],
    );
    assert_eq!(
        extract_copied_text(&copy_out).as_deref(),
        Some(generated_key.as_str())
    );

    // 4. Output Tab (UDP destination address): paste, right-click open context menu, verify menu items
    app.active_tab = Tab::Output;
    app.output_states[0].output_type = OutputType::Udp;
    app.output_states[0].udp_address.clear();
    let out = run_app(&ctx, &mut app, vec![]);
    let udp_pos = find_rendered_text_pos(&out.shapes, "127.0.0.1:9101")
        .expect("UDP output hint should be rendered")
        + egui::vec2(16.0, 6.0);

    click_at(&ctx, &mut app, udp_pos);
    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("192.168.1.50:9101".to_string())],
    );
    assert_eq!(app.output_states[0].udp_address, "192.168.1.50:9101");

    // Right-click the Output tab text field to verify pop-up context menu opens in full DatapipeApp
    let _ = run_app(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(udp_pos),
            egui::Event::PointerButton {
                pos: udp_pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let _ = run_app(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos: udp_pos,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let menu_out = run_app(&ctx, &mut app, vec![]);
    assert!(find_rendered_text_pos(&menu_out.shapes, "Cut").is_some());
    assert!(find_rendered_text_pos(&menu_out.shapes, "Copy").is_some());
    assert!(find_rendered_text_pos(&menu_out.shapes, "Paste").is_some());
    assert!(find_rendered_text_pos(&menu_out.shapes, "Select All").is_some());
}
