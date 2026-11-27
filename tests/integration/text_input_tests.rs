use nservo::core::{
    DeadKey, DeadKeyComposer, LineEdge, WordDirection, compose_dead_key, decode_utf16_units,
    encode_utf16_units, needs_utf16_injection,
};
use nservo::engine::clipboard::{
    copy_selection_script, cut_selection_script, parse_clipboard_message,
};
use nservo::engine::text_input_js::{insert_utf16_script, move_line_edge_script, move_word_script};

#[test]
fn utf16_roundtrip_handles_portuguese_accents_and_surrogate_pairs() {
    let portuguese = "Olá, ação e coração! áéíóú âêô ãõ à ü ç ÁÉÍÓÚ ÃÕ Ç";
    let units = encode_utf16_units(portuguese);
    assert_eq!(decode_utf16_units(&units).unwrap(), portuguese);

    // 'á' is U+00E1 (single UTF-16 code unit: 225).
    assert_eq!(encode_utf16_units("á"), vec![0x00E1]);

    // Astral-plane emoji uses a UTF-16 surrogate pair (2 code units).
    let emoji_units = encode_utf16_units("😀");
    assert_eq!(emoji_units, vec![0xD83D, 0xDE00]);
    assert_eq!(decode_utf16_units(&emoji_units).unwrap(), "😀");
}

#[test]
fn dead_key_composer_handles_portuguese_diacritics() {
    assert_eq!(compose_dead_key(DeadKey::Acute, 'a'), Some('á'));
    assert_eq!(compose_dead_key(DeadKey::Acute, 'e'), Some('é'));
    assert_eq!(compose_dead_key(DeadKey::Acute, 'c'), Some('ç'));
    assert_eq!(compose_dead_key(DeadKey::Tilde, 'a'), Some('ã'));
    assert_eq!(compose_dead_key(DeadKey::Tilde, 'o'), Some('õ'));
    assert_eq!(compose_dead_key(DeadKey::Circumflex, 'e'), Some('ê'));
    assert_eq!(compose_dead_key(DeadKey::Circumflex, 'o'), Some('ô'));
    assert_eq!(compose_dead_key(DeadKey::Grave, 'a'), Some('à'));
    assert_eq!(compose_dead_key(DeadKey::Diaeresis, 'u'), Some('ü'));
    assert_eq!(compose_dead_key(DeadKey::Acute, 'A'), Some('Á'));
    assert_eq!(compose_dead_key(DeadKey::Tilde, ' '), Some('~'));

    let mut composer = DeadKeyComposer::new();
    assert_eq!(composer.push_dead_key(DeadKey::Acute), None);
    assert_eq!(composer.push_char('a'), Some("á".to_string()));
    // No pending dead key left.
    assert_eq!(composer.push_char('a'), None);

    // Non-combinable pair falls back to standalone diacritic + char.
    assert_eq!(composer.push_dead_key(DeadKey::Tilde), None);
    assert_eq!(composer.push_char('x'), Some("~x".to_string()));

    // Full Portuguese word sequence: "áéção"
    let mut out = String::new();
    assert_eq!(composer.push_dead_key(DeadKey::Acute), None);
    assert!(composer.has_pending());
    out.push_str(&composer.push_char('a').unwrap());
    assert!(!composer.has_pending());
    assert_eq!(composer.push_dead_key(DeadKey::Acute), None);
    out.push_str(&composer.push_char('e').unwrap());
    out.push('ç');
    assert_eq!(composer.push_dead_key(DeadKey::Tilde), None);
    out.push_str(&composer.push_char('a').unwrap());
    out.push('o');
    assert_eq!(out, "áéção");
}

#[test]
fn utf16_injection_decision_and_script_generation() {
    assert!(!needs_utf16_injection("a", false));
    assert!(needs_utf16_injection("~", true));
    assert!(needs_utf16_injection("á", false));
    assert!(needs_utf16_injection("ç", false));
    assert!(!needs_utf16_injection("", true));

    let script = insert_utf16_script(&encode_utf16_units("á😀"));
    assert!(script.contains("String.fromCharCode(225,55357,56832)"));
}

#[test]
fn move_word_script_encodes_direction_and_selection_mode() {
    let prev = move_word_script(WordDirection::Left, false);
    assert!(prev.contains("var isLeft = true;"));
    assert!(prev.contains("var extend = false;"));

    let sel_prev = move_word_script(WordDirection::Left, true);
    assert!(sel_prev.contains("var isLeft = true;"));
    assert!(sel_prev.contains("var extend = true;"));

    let sel_next = move_word_script(WordDirection::Right, true);
    assert!(sel_next.contains("var isLeft = false;"));
    assert!(sel_next.contains("var extend = true;"));
}

#[test]
fn clipboard_payload_parsing_and_chunked_utf16_paste() {
    assert_eq!(
        parse_clipboard_message(r#"{"text":"Olá, ação!"}"#),
        Some("Olá, ação!".to_string())
    );
    assert_eq!(parse_clipboard_message(r#"{"text":""}"#), None);
    assert_eq!(parse_clipboard_message("invalid"), None);
    assert!(copy_selection_script().contains("nservoClipboard"));

    // Large paste (> 4096 UTF-16 units) splits into concatenated String.fromCharCode chunks.
    let big = "á".repeat(5000);
    let script = insert_utf16_script(&encode_utf16_units(&big));
    assert!(script.contains(")+String.fromCharCode("));
    assert!(cut_selection_script().contains("deleteByCut"));
}

#[test]
fn move_line_edge_script_encodes_start_and_end() {
    let start = move_line_edge_script(LineEdge::Start);
    assert!(start.contains("var toStart = true;"));
    assert!(start.contains("'backward', 'lineboundary'"));

    let end = move_line_edge_script(LineEdge::End);
    assert!(end.contains("var toStart = false;"));
    assert!(end.contains("'forward', 'lineboundary'"));
}
