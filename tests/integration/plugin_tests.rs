use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use nservo::core::{
    BrowserAction, BrowserConfig, CompletionSources, KeyInput, KeyMapper, KeyOutcome, Mode,
    PluginAction, PluginMode, complete_command_input, discover_plugins, load_plugin_from_dir,
    normalize_plugin_repo_url, parse_command, parse_plugin_actions, resolve_plugins_dir,
};
use nservo::engine::config_page::render_config_html;
use nservo::engine::plugin_runner::{
    PluginContext, build_page_js_payload, execute_plugin_command, install_plugin, remove_plugin,
    update_plugins,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("nservo-plug-{tag}-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn plugin_paths_commands_repo_urls_and_completion() {
    assert_eq!(
        resolve_plugins_dir(Some("/tmp/xdg"), Some("/home/u")),
        PathBuf::from("/tmp/xdg/nservo/plugins")
    );
    assert_eq!(
        normalize_plugin_repo_url("alice/nservo-reader"),
        Ok((
            "https://github.com/alice/nservo-reader.git".to_string(),
            "nservo-reader".to_string()
        ))
    );
    assert_eq!(
        normalize_plugin_repo_url("https://github.com/alice/pass-fill.git"),
        Ok((
            "https://github.com/alice/pass-fill.git".to_string(),
            "pass-fill".to_string()
        ))
    );
    assert!(normalize_plugin_repo_url("bad repo").is_err());

    assert_eq!(
        parse_command(":plugin readability arg1"),
        Ok(BrowserAction::RunPlugin {
            name: "readability".to_string(),
            args: "arg1".to_string()
        })
    );
    assert_eq!(
        parse_command(":plugin-install alice/nservo-reader"),
        Ok(BrowserAction::InstallPlugin(
            "alice/nservo-reader".to_string()
        ))
    );
    assert_eq!(
        parse_command(":plugin-update"),
        Ok(BrowserAction::UpdatePlugins(None))
    );
    assert_eq!(
        parse_command(":plugin-update readability"),
        Ok(BrowserAction::UpdatePlugins(Some(
            "readability".to_string()
        )))
    );
    assert_eq!(
        parse_command(":plugin-remove readability"),
        Ok(BrowserAction::RemovePlugin("readability".to_string()))
    );

    let sources = CompletionSources {
        plugins: vec![("readability".to_string(), "Reader view".to_string())],
        ..Default::default()
    };
    let items = complete_command_input("plugin read", &sources);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].insert_text, "plugin readability");
}

#[test]
fn parses_plugin_stdout_actions_and_runs_bundled_examples() {
    let stdout = r#"
{"op":"eval_js","code":"console.log(1)"}
{"op":"copy","text":"secret"}
{"op":"open","url":"https://example.com","new_tab":true}
{"op":"notice","text":"Done"}
"#;
    let parsed = parse_plugin_actions(stdout).unwrap();
    assert_eq!(
        parsed,
        vec![
            PluginAction::EvalJs("console.log(1)".to_string()),
            PluginAction::Copy("secret".to_string()),
            PluginAction::Open {
                url: "https://example.com".to_string(),
                new_tab: true
            },
            PluginAction::Notice("Done".to_string()),
        ]
    );

    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/plugins");
    let bundled = discover_plugins(&examples_dir);
    assert_eq!(bundled.len(), 2);

    let reader = load_plugin_from_dir(&examples_dir.join("readability")).unwrap();
    assert_eq!(reader.manifest.mode, PluginMode::PageJs);
    let ctx = PluginContext {
        url: "https://news.example/article".to_string(),
        title: "Article".to_string(),
        args: String::new(),
        config_dir: PathBuf::from("/tmp/cfg"),
    };
    let js = build_page_js_payload(&reader, &ctx).unwrap();
    assert!(js.contains("nservo-readability-overlay"));
    assert!(js.contains("nservo-plugin-style-readability"));

    // Verify plugin shortcuts apply in KeyMapper and render in about:config.
    let mut cfg = BrowserConfig::default();
    cfg.apply_plugin_defaults(&bundled);
    assert_eq!(cfg.plugin_key_for("readability"), "zr");
    assert_eq!(cfg.plugin_key_for("pass-fill"), "zp");

    let html = render_config_html(&cfg, Path::new("/tmp/config.json"), &bundled);
    assert!(html.contains("plugin:readability"));
    assert!(html.contains("plugin:pass-fill"));

    let mut mapper = KeyMapper::with_config(cfg);
    let now = Instant::now();
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('z'), now),
        KeyOutcome::Pending("z".to_string())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('r'), now),
        KeyOutcome::Action(BrowserAction::RunPlugin {
            name: "readability".to_string(),
            args: String::new()
        })
    );
}

#[test]
fn git_install_update_exec_and_remove_plugin_end_to_end() {
    let root = temp_dir("e2e");
    let origin = root.join("demo-plug");
    let plugins_dir = root.join("plugins");
    fs::create_dir_all(&origin).unwrap();

    fs::write(
        origin.join("plugin.json"),
        r#"{"name":"demo-plug","description":"Echo host","default_shortcut":"zd","mode":"exec","command":"./run.sh","timeout_ms":5000}"#,
    )
    .unwrap();
    let script = origin.join("run.sh");
    fs::write(
        &script,
        "#!/bin/sh\nprintf '{\"op\":\"notice\",\"text\":\"v1:%s\"}\\n' \"$NSERVO_ORIGIN\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let git = |args: &[&str]| {
        let st = Command::new("git")
            .current_dir(&origin)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(args)
            .status()
            .expect("git");
        assert!(st.success(), "git {args:?} failed");
    };
    git(&["init"]);
    git(&["add", "."]);
    git(&["commit", "-m", "v1"]);

    let repo_url = format!("file://{}", origin.display());
    let installed = install_plugin(&plugins_dir, &repo_url).unwrap();
    assert_eq!(installed.manifest.name, "demo-plug");

    let ctx = PluginContext {
        url: "https://github.com/login".to_string(),
        title: "Login".to_string(),
        args: String::new(),
        config_dir: root.clone(),
    };
    let actions = execute_plugin_command(&installed, &ctx).unwrap();
    assert_eq!(
        actions,
        vec![PluginAction::Notice("v1:https://github.com".to_string())]
    );

    // Commit v2 upstream and verify :plugin-update pulls it cleanly.
    fs::write(
        &script,
        "#!/bin/sh\nprintf '{\"op\":\"notice\",\"text\":\"v2:%s\"}\\n' \"$NSERVO_ORIGIN\"\n",
    )
    .unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "v2"]);

    let summary = update_plugins(&plugins_dir, None);
    assert_eq!(summary.total, 1);
    assert_eq!(summary.updated, 1);
    assert!(summary.failed.is_empty());

    let actions_v2 = execute_plugin_command(&installed, &ctx).unwrap();
    assert_eq!(
        actions_v2,
        vec![PluginAction::Notice("v2:https://github.com".to_string())]
    );

    assert_eq!(
        remove_plugin(&plugins_dir, "demo-plug"),
        Ok("demo-plug".to_string())
    );
    assert!(discover_plugins(&plugins_dir).is_empty());

    let _ = fs::remove_dir_all(&root);
}
