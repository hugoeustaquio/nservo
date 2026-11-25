use nservo::core::{BrowserAction, CommandError, normalize_url, parse_command};

#[test]
fn open_lowercase_targets_current_tab() {
    assert_eq!(
        parse_command(":open https://rust-lang.org"),
        Ok(BrowserAction::OpenInCurrentTab(
            "https://rust-lang.org".to_string()
        ))
    );
    assert_eq!(
        parse_command("open example.com"),
        Ok(BrowserAction::OpenInCurrentTab(
            "https://example.com".to_string()
        ))
    );
    assert_eq!(
        parse_command(":o localhost:8080"),
        Ok(BrowserAction::OpenInCurrentTab(
            "http://localhost:8080".to_string()
        ))
    );
}

#[test]
fn open_uppercase_targets_new_tab() {
    assert_eq!(
        parse_command(":Open https://example.org"),
        Ok(BrowserAction::OpenInNewTab(
            "https://example.org".to_string()
        ))
    );
    assert_eq!(
        parse_command(":Open example.com/docs"),
        Ok(BrowserAction::OpenInNewTab(
            "https://example.com/docs".to_string()
        ))
    );
    assert_eq!(
        parse_command(":O 127.0.0.1:3000"),
        Ok(BrowserAction::OpenInNewTab(
            "http://127.0.0.1:3000".to_string()
        ))
    );
    assert_eq!(
        parse_command(":tabopen https://servo.org"),
        Ok(BrowserAction::OpenInNewTab("https://servo.org".to_string()))
    );
}

#[test]
fn navigation_and_tab_commands_parse() {
    assert_eq!(parse_command(":tabnext"), Ok(BrowserAction::NextTab));
    assert_eq!(parse_command(":tabprev"), Ok(BrowserAction::PrevTab));
    assert_eq!(parse_command(":tabclose"), Ok(BrowserAction::CloseTab));
    assert_eq!(parse_command(":reload"), Ok(BrowserAction::Reload));
    assert_eq!(parse_command(":back"), Ok(BrowserAction::GoBack));
    assert_eq!(parse_command(":forward"), Ok(BrowserAction::GoForward));
    assert_eq!(parse_command(":help"), Ok(BrowserAction::ShowHelp));
    assert_eq!(parse_command(":q"), Ok(BrowserAction::Quit));
}

#[test]
fn session_commands_parse_and_validate_names() {
    assert_eq!(
        parse_command(":w"),
        Ok(BrowserAction::SaveSession("default".to_string()))
    );
    assert_eq!(
        parse_command(":w work_1"),
        Ok(BrowserAction::SaveSession("work_1".to_string()))
    );
    assert_eq!(
        parse_command(":wq"),
        Ok(BrowserAction::SaveSessionAndQuit("default".to_string()))
    );
    assert_eq!(
        parse_command(":wq research"),
        Ok(BrowserAction::SaveSessionAndQuit("research".to_string()))
    );
    assert_eq!(
        parse_command(":session research"),
        Ok(BrowserAction::LoadSession("research".to_string()))
    );
    assert!(matches!(
        parse_command(":session"),
        Err(CommandError::MissingArgument(_))
    ));
    assert!(matches!(
        parse_command(":w ../escape"),
        Err(CommandError::InvalidArgument(_))
    ));
    assert!(matches!(
        parse_command(":session bad/name"),
        Err(CommandError::InvalidArgument(_))
    ));
}

#[test]
fn zoom_command_validates_bounds() {
    assert_eq!(parse_command(":zoom 1.5"), Ok(BrowserAction::SetZoom(1.5)));
    assert_eq!(
        parse_command(":zoom reset"),
        Ok(BrowserAction::SetZoom(1.0))
    );
    assert!(matches!(
        parse_command(":zoom"),
        Err(CommandError::MissingArgument(_))
    ));
    assert!(matches!(
        parse_command(":zoom 50"),
        Err(CommandError::InvalidArgument(_))
    ));
}

#[test]
fn invalid_and_empty_commands_return_errors() {
    assert_eq!(parse_command(""), Err(CommandError::Empty));
    assert_eq!(parse_command("   :   "), Err(CommandError::Empty));
    assert!(matches!(
        parse_command(":open"),
        Err(CommandError::MissingArgument(_))
    ));
    assert!(matches!(
        parse_command(":Open   "),
        Err(CommandError::MissingArgument(_))
    ));
    assert_eq!(
        parse_command(":bogus"),
        Err(CommandError::UnknownCommand("bogus".to_string()))
    );
}

#[test]
fn normalize_url_preserves_explicit_schemes() {
    assert_eq!(normalize_url("http://a.test"), "http://a.test");
    assert_eq!(normalize_url("https://a.test"), "https://a.test");
    assert_eq!(normalize_url("about:blank"), "about:blank");
    assert_eq!(normalize_url("file:///tmp/a.html"), "file:///tmp/a.html");
    assert_eq!(normalize_url("example.com"), "https://example.com");
}
