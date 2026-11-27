use nservo::core::{
    BrowserAction, CommandError, ZOOM_STEP, normalize_url, parse_command, step_zoom,
};

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
    assert_eq!(
        parse_command(":inspect"),
        Ok(BrowserAction::ToggleInspector)
    );
    assert_eq!(
        parse_command(":devtools"),
        Ok(BrowserAction::ToggleInspector)
    );
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
    assert_eq!(step_zoom(1.0, ZOOM_STEP), 1.1);
    assert_eq!(step_zoom(1.1, ZOOM_STEP), 1.2);
    assert_eq!(step_zoom(1.0, -ZOOM_STEP), 0.9);
    assert_eq!(step_zoom(0.1, -ZOOM_STEP), 0.1);
    assert_eq!(step_zoom(10.0, ZOOM_STEP), 10.0);
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
fn normalize_url_preserves_explicit_schemes_and_hosts() {
    assert_eq!(normalize_url("http://a.test"), "http://a.test");
    assert_eq!(normalize_url("https://a.test"), "https://a.test");
    assert_eq!(normalize_url("about:blank"), "about:blank");
    assert_eq!(normalize_url("file:///tmp/a.html"), "file:///tmp/a.html");
    assert_eq!(normalize_url("example.com"), "https://example.com");
    assert_eq!(
        normalize_url("sub.example.co.uk:8443/path?a=1#top"),
        "https://sub.example.co.uk:8443/path?a=1#top"
    );
    assert_eq!(normalize_url("localhost"), "http://localhost");
    assert_eq!(
        normalize_url("192.168.1.10/status"),
        "http://192.168.1.10/status"
    );
    assert_eq!(normalize_url("[::1]:8080"), "http://[::1]:8080");
}

#[test]
fn non_url_inputs_fall_back_to_google_search() {
    assert_eq!(
        parse_command(":open rust"),
        Ok(BrowserAction::OpenInCurrentTab(
            "https://www.google.com/search?q=rust".to_string()
        ))
    );
    assert_eq!(
        parse_command(":Open rust gtk4 browser"),
        Ok(BrowserAction::OpenInNewTab(
            "https://www.google.com/search?q=rust+gtk4+browser".to_string()
        ))
    );
    assert_eq!(
        normalize_url("c++ & ação?"),
        "https://www.google.com/search?q=c%2B%2B+%26+a%C3%A7%C3%A3o%3F"
    );
    assert_eq!(
        normalize_url("hello world.com"),
        "https://www.google.com/search?q=hello+world.com"
    );
    assert_eq!(
        normalize_url("3.14"),
        "https://www.google.com/search?q=3.14"
    );
    assert_eq!(
        normalize_url("std::vec::Vec"),
        "https://www.google.com/search?q=std%3A%3Avec%3A%3AVec"
    );
    assert_eq!(
        normalize_url("localhosting"),
        "https://www.google.com/search?q=localhosting"
    );
    assert_eq!(
        normalize_url("my_var.len"),
        "https://www.google.com/search?q=my_var.len"
    );
    assert_eq!(
        normalize_url("foo."),
        "https://www.google.com/search?q=foo."
    );
}
