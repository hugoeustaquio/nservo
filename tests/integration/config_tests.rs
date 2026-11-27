use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use nservo::core::{
    BrowserAction, BrowserConfig, CompletionSources, ConfigEvent, KeyInput, KeyMapper, KeyOutcome,
    Mode, complete_command_input, parse_command, parse_config_message, resolve_config_path,
};
use nservo::engine::config_page::{
    CONFIG_MESSAGE_HANDLER, config_feedback_script, render_config_html,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_config_path(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("nservo-cfg-{tag}-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    dir.join("config.json")
}

#[test]
fn config_path_uses_xdg_config_home_then_home() {
    assert_eq!(
        resolve_config_path(Some("/tmp/xdg"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg/nservo/config.json")
    );
    assert_eq!(
        resolve_config_path(None, Some("/home/alice")),
        PathBuf::from("/home/alice/.config/nservo/config.json")
    );
}

#[test]
fn config_command_parses_and_completes() {
    assert_eq!(parse_command(":config"), Ok(BrowserAction::ShowConfig));
    assert_eq!(
        parse_command(":open about:config"),
        Ok(BrowserAction::OpenInCurrentTab("about:config".to_string()))
    );
    let items = complete_command_input("conf", &CompletionSources::default());
    assert!(items.iter().any(|i| i.primary == "config"));
}

#[test]
fn saves_loads_and_applies_custom_single_and_sequence_keybindings() {
    let path = temp_config_path("custom");
    let mut cfg = BrowserConfig::default();
    cfg.keybindings.insert("scroll_down".to_string(), "s".to_string());
    cfg.keybindings.insert("close_tab".to_string(), "x".to_string());
    cfg.keybindings.insert("toggle_inspector".to_string(), "zi".to_string());
    cfg.save(&path).expect("save custom config");

    let loaded = BrowserConfig::load(&path);
    assert_eq!(loaded.key_for("scroll_down"), "s");
    assert_eq!(loaded.key_for("close_tab"), "x");
    assert_eq!(loaded.key_for("toggle_inspector"), "zi");

    let mut mapper = KeyMapper::with_config(loaded);
    let now = Instant::now();
    // Old 'd' no longer closes tab; new 'x' does; counted '3s' scrolls down 3 steps.
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('d'), now), KeyOutcome::Consumed);
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('x'), now),
        KeyOutcome::Action(BrowserAction::CloseTab)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('3'), now),
        KeyOutcome::Pending("3".to_string())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('s'), now),
        KeyOutcome::Action(BrowserAction::ScrollDown(3))
    );
    // Custom 2-key sequence 'zi' toggles inspector.
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('z'), now),
        KeyOutcome::Pending("z".to_string())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('i'), now),
        KeyOutcome::Action(BrowserAction::ToggleInspector)
    );

    if let Some(dir) = path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn validates_duplicate_prefix_and_digit_conflicts() {
    let mut dup = BrowserConfig::default();
    dup.keybindings.insert("close_tab".to_string(), "j".to_string());
    assert!(dup.validate().is_err());

    let mut prefix_clash = BrowserConfig::default();
    prefix_clash.keybindings.insert("close_tab".to_string(), "g".to_string());
    assert!(prefix_clash.validate().is_err());

    let mut digit = BrowserConfig::default();
    digit.keybindings.insert("close_tab".to_string(), "5".to_string());
    assert!(digit.validate().is_err());

    let mut too_long = BrowserConfig::default();
    too_long.keybindings.insert("close_tab".to_string(), "xyz".to_string());
    assert!(too_long.validate().is_err());
}

#[test]
fn about_config_renders_options_and_parses_save_and_reset_messages() {
    let path = PathBuf::from("/tmp/xdg/nservo/config.json");
    let mut cfg = BrowserConfig::default();
    cfg.keybindings.insert("close_tab".to_string(), "x".to_string());

    let html = render_config_html(&cfg, &path);
    assert!(html.contains("/tmp/xdg/nservo/config.json"));
    assert!(html.contains(CONFIG_MESSAGE_HANDLER));
    assert!(html.contains(r#"data-id="close_tab""#));
    assert!(html.contains(r#"value="x""#));
    assert!(config_feedback_script(true, "Saved").contains("__nservoConfigStatus(true, \"Saved\")"));

    let save_msg = r#"{"action":"save","keybindings":{"close_tab":"x"}}"#;
    match parse_config_message(save_msg) {
        Some(ConfigEvent::Save(parsed)) => {
            assert_eq!(parsed.key_for("close_tab"), "x");
            assert_eq!(parsed.key_for("scroll_down"), "j");
        }
        other => panic!("expected Save event, got {other:?}"),
    }
    assert_eq!(parse_config_message(r#"{"action":"reset"}"#), Some(ConfigEvent::Reset));
    assert_eq!(parse_config_message(r#"{"action":"unknown"}"#), None);

    // Partial on-disk config merges cleanly with defaults.
    let mut partial = BTreeMap::new();
    partial.insert("close_tab".to_string(), "x".to_string());
    let merged = BrowserConfig::merge_with_defaults(partial);
    assert_eq!(merged.key_for("close_tab"), "x");
    assert_eq!(merged.key_for("reload"), "r");
}
