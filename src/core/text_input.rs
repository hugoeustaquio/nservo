/// Direction for word-wise caret movement and selection (`Ctrl+Arrow` / `Ctrl+Shift+Arrow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordDirection {
    Left,
    Right,
}

/// Target edge for line-wise caret movement (`Ctrl+A` / `Ctrl+E`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEdge {
    Start,
    End,
}

/// Encode a Rust UTF-8 string into UTF-16 code units (including surrogate pairs
/// for supplementary Unicode scalar values).
pub fn encode_utf16_units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// Decode UTF-16 code units back into a Rust `String`.
pub fn decode_utf16_units(units: &[u16]) -> Result<String, std::char::DecodeUtf16Error> {
    char::decode_utf16(units.iter().copied()).collect()
}

/// Whether committed input text must be injected via UTF-16 JS rather than
/// passed through as a raw GDK key event to `servo-gtk`.
pub fn needs_utf16_injection(text: &str, was_composed: bool) -> bool {
    !text.is_empty() && (was_composed || !text.is_ascii())
}

/// Common Latin/Portuguese dead keys for fallback composition when the system
/// input method does not handle dead keys directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadKey {
    Acute,
    Grave,
    Circumflex,
    Tilde,
    Diaeresis,
    Cedilla,
}

impl DeadKey {
    pub const fn standalone(self) -> char {
        match self {
            Self::Acute => '´',
            Self::Grave => '`',
            Self::Circumflex => '^',
            Self::Tilde => '~',
            Self::Diaeresis => '¨',
            Self::Cedilla => '¸',
        }
    }
}

/// Compose a dead key with a following character (supporting Portuguese and
/// common Latin diacritics).
pub fn compose_dead_key(dead: DeadKey, base: char) -> Option<char> {
    if base == ' ' {
        return Some(dead.standalone());
    }
    match (dead, base) {
        (DeadKey::Acute, 'a') => Some('á'),
        (DeadKey::Acute, 'e') => Some('é'),
        (DeadKey::Acute, 'i') => Some('í'),
        (DeadKey::Acute, 'o') => Some('ó'),
        (DeadKey::Acute, 'u') => Some('ú'),
        (DeadKey::Acute, 'c') => Some('ç'),
        (DeadKey::Acute, 'A') => Some('Á'),
        (DeadKey::Acute, 'E') => Some('É'),
        (DeadKey::Acute, 'I') => Some('Í'),
        (DeadKey::Acute, 'O') => Some('Ó'),
        (DeadKey::Acute, 'U') => Some('Ú'),
        (DeadKey::Acute, 'C') => Some('Ç'),
        (DeadKey::Grave, 'a') => Some('à'),
        (DeadKey::Grave, 'e') => Some('è'),
        (DeadKey::Grave, 'i') => Some('ì'),
        (DeadKey::Grave, 'o') => Some('ò'),
        (DeadKey::Grave, 'u') => Some('ù'),
        (DeadKey::Grave, 'A') => Some('À'),
        (DeadKey::Grave, 'E') => Some('È'),
        (DeadKey::Grave, 'I') => Some('Ì'),
        (DeadKey::Grave, 'O') => Some('Ò'),
        (DeadKey::Grave, 'U') => Some('Ù'),
        (DeadKey::Circumflex, 'a') => Some('â'),
        (DeadKey::Circumflex, 'e') => Some('ê'),
        (DeadKey::Circumflex, 'i') => Some('î'),
        (DeadKey::Circumflex, 'o') => Some('ô'),
        (DeadKey::Circumflex, 'u') => Some('û'),
        (DeadKey::Circumflex, 'A') => Some('Â'),
        (DeadKey::Circumflex, 'E') => Some('Ê'),
        (DeadKey::Circumflex, 'I') => Some('Î'),
        (DeadKey::Circumflex, 'O') => Some('Ô'),
        (DeadKey::Circumflex, 'U') => Some('Û'),
        (DeadKey::Tilde, 'a') => Some('ã'),
        (DeadKey::Tilde, 'o') => Some('õ'),
        (DeadKey::Tilde, 'n') => Some('ñ'),
        (DeadKey::Tilde, 'A') => Some('Ã'),
        (DeadKey::Tilde, 'O') => Some('Õ'),
        (DeadKey::Tilde, 'N') => Some('Ñ'),
        (DeadKey::Diaeresis, 'a') => Some('ä'),
        (DeadKey::Diaeresis, 'e') => Some('ë'),
        (DeadKey::Diaeresis, 'i') => Some('ï'),
        (DeadKey::Diaeresis, 'o') => Some('ö'),
        (DeadKey::Diaeresis, 'u') => Some('ü'),
        (DeadKey::Diaeresis, 'A') => Some('Ä'),
        (DeadKey::Diaeresis, 'E') => Some('Ë'),
        (DeadKey::Diaeresis, 'I') => Some('Ï'),
        (DeadKey::Diaeresis, 'O') => Some('Ö'),
        (DeadKey::Diaeresis, 'U') => Some('Ü'),
        (DeadKey::Cedilla, 'c') => Some('ç'),
        (DeadKey::Cedilla, 'C') => Some('Ç'),
        _ => None,
    }
}

/// Stateful fallback composer for dead-key sequences.
#[derive(Debug, Default)]
pub struct DeadKeyComposer {
    pending: Option<DeadKey>,
}

impl DeadKeyComposer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.pending = None;
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Press a dead key. Pressing the same dead key twice emits its standalone glyph.
    pub fn push_dead_key(&mut self, dead: DeadKey) -> Option<String> {
        match self.pending.take() {
            Some(prev) if prev == dead => Some(dead.standalone().to_string()),
            Some(prev) => {
                self.pending = Some(dead);
                Some(prev.standalone().to_string())
            }
            None => {
                self.pending = Some(dead);
                None
            }
        }
    }

    /// Combine a pending dead key with `ch`, returning composed text if one was pending.
    pub fn push_char(&mut self, ch: char) -> Option<String> {
        let dead = self.pending.take()?;
        if let Some(composed) = compose_dead_key(dead, ch) {
            Some(composed.to_string())
        } else {
            let mut out = String::new();
            out.push(dead.standalone());
            out.push(ch);
            Some(out)
        }
    }
}
