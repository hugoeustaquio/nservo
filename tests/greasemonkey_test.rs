use nservo::greasemonkey::{metadata::RunAt, ScriptMetadata, UserScript};
use std::path::PathBuf;

#[test]
fn test_metadata_parsing() {
    let script_content = r#"// ==UserScript==
// @name         Test Script
// @description  Automated test helper
// @match        *://github.com/*
// @run-at       document-start
// ==/UserScript==

console.log("running test");
"#;

    let meta = ScriptMetadata::parse(script_content).unwrap();
    assert_eq!(meta.name, "Test Script");
    assert_eq!(meta.description, "Automated test helper");
    assert_eq!(meta.matches, vec!["*://github.com/*"]);
    assert_eq!(meta.run_at, RunAt::DocumentStart);
}

#[test]
fn test_url_matching_and_injection() {
    let meta = ScriptMetadata {
        name: "Test".to_string(),
        matches: vec!["*://github.com/*".to_string()],
        ..Default::default()
    };

    let user_script = UserScript::new(PathBuf::from("test.user.js"), meta, "alert(1);".to_string());

    assert!(user_script.matches_url("https://github.com/nservo/nservo"));
    assert!(!user_script.matches_url("https://gitlab.com/nservo/nservo"));

    let injection = user_script.generate_injection_code();
    assert!(injection.contains("NServo Greasemonkey Script: Test"));
    assert!(injection.contains("alert(1);"));
}
