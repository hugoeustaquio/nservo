use nservo::core::{
    DeadKey, DeadKeyComposer, compose_dead_key, decode_utf16_units, encode_utf16_units,
    needs_utf16_injection,
};
use nservo::engine::text_input_js::insert_utf16_script;

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
