use nservo::command::{AutocompleteEngine, CommandAction, CommandParser};

#[test]
fn test_parse_standard_commands() {
    assert_eq!(
        CommandParser::parse(":open https://servo.org").unwrap(),
        CommandAction::Open("https://servo.org".to_string())
    );

    assert_eq!(
        CommandParser::parse(":Open https://github.com").unwrap(),
        CommandAction::OpenNewTab("https://github.com".to_string())
    );

    assert_eq!(CommandParser::parse(":tabnext").unwrap(), CommandAction::TabNext);
    assert_eq!(CommandParser::parse(":tabprev").unwrap(), CommandAction::TabPrev);
    assert_eq!(CommandParser::parse(":tabclose").unwrap(), CommandAction::TabClose);
    assert_eq!(CommandParser::parse(":tabs").unwrap(), CommandAction::Tabs);
    assert_eq!(CommandParser::parse(":wq").unwrap(), CommandAction::WriteQuit);
    assert_eq!(CommandParser::parse(":q").unwrap(), CommandAction::Quit);
    assert_eq!(CommandParser::parse(":bookmark").unwrap(), CommandAction::Bookmark);
    assert_eq!(CommandParser::parse(":clean").unwrap(), CommandAction::Clean);

    assert_eq!(
        CommandParser::parse(":plugin pass").unwrap(),
        CommandAction::Plugin("pass".to_string())
    );
}

#[test]
fn test_autocomplete() {
    let suggestions = AutocompleteEngine::suggest("tab", None, None);
    assert!(suggestions.contains(&":tabnew".to_string()));
    assert!(suggestions.contains(&":tabnext".to_string()));
    assert!(suggestions.contains(&":tabprev".to_string()));
    assert!(suggestions.contains(&":tabclose".to_string()));
    assert!(suggestions.contains(&":tabs".to_string()));
}
