use nservo::adblock::{AdblockEngine, AdblockParser, ContentType, RuleType};

#[test]
fn test_parse_network_block_and_exception() {
    let rule_block = AdblockParser::parse_line("||doubleclick.net^$third-party").unwrap();
    assert_eq!(rule_block.rule_type, RuleType::NetworkBlock);
    assert_eq!(rule_block.pattern, "||doubleclick.net^");
    assert_eq!(rule_block.third_party, Some(true));

    let rule_exc = AdblockParser::parse_line("@@||doubleclick.net/whitelist^").unwrap();
    assert_eq!(rule_exc.rule_type, RuleType::NetworkException);
    assert_eq!(rule_exc.pattern, "||doubleclick.net/whitelist^");
}

#[test]
fn test_parse_element_hiding() {
    let rule = AdblockParser::parse_line("example.com##.sponsor-banner").unwrap();
    assert_eq!(rule.rule_type, RuleType::ElementHiding);
    assert_eq!(rule.pattern, ".sponsor-banner");
    assert_eq!(rule.domains, vec!["example.com"]);
}

#[test]
fn test_engine_blocking_logic() {
    let mut engine = AdblockEngine::new();
    let rules = r#"
||adservice.com^$third-party
@@||adservice.com/allowed^
/tracking.js$script
##.ad-box
"#;
    engine.load_rules_from_str(rules);

    // Blocked by ||adservice.com^
    assert!(engine.should_block_request(
        "https://adservice.com/serve.js",
        ContentType::Script,
        true
    ));

    // Not blocked if not third party (third-party constraint)
    assert!(!engine.should_block_request(
        "https://adservice.com/serve.js",
        ContentType::Script,
        false
    ));

    // Whitelisted by exception @@||adservice.com/allowed^
    assert!(!engine.should_block_request(
        "https://adservice.com/allowed/image.png",
        ContentType::Image,
        true
    ));

    // Element hiding
    let selectors = engine.get_element_hiding_selectors("example.com");
    assert!(selectors.contains(&".ad-box".to_string()));
}
