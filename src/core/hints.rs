use serde::Deserialize;

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

/// Generate prefix-free lowercase alphabetic hint labels for `count` targets.
///
/// - `1..=26` elements use single letters (`a`, `b`, `c`, ...).
/// - `27..=676` elements use fixed 2-letter codes (`aa`, `ab`, ...), ensuring no
///   hint is ever a prefix of another so selection resolves immediately.
pub fn generate_hint_labels(count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }

    let base = ALPHABET.len();
    let mut length = 1usize;
    let mut capacity = base;
    while capacity < count {
        length += 1;
        capacity = capacity.saturating_mul(base);
    }

    (0..count)
        .map(|mut idx| {
            let mut chars = vec![b'a'; length];
            for pos in (0..length).rev() {
                chars[pos] = ALPHABET[idx % base];
                idx /= base;
            }
            String::from_utf8(chars).expect("ASCII alphabet is always valid UTF-8")
        })
        .collect()
}

/// Native events posted back from the injected hint script over `UserContentManager`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintEvent {
    Shown { count: usize },
    Activated { is_input: bool },
    OpenInNewTab { url: String },
    Cancelled,
    Empty,
}

#[derive(Deserialize)]
struct RawHintMessage {
    event: String,
    #[serde(default)]
    count: usize,
    #[serde(default, rename = "isInput")]
    is_input: bool,
    #[serde(default)]
    url: String,
}

/// Parse a JSON message payload sent from page JavaScript via `nservoHints`.
pub fn parse_hint_message(body: &str) -> Option<HintEvent> {
    let raw: RawHintMessage = serde_json::from_str(body).ok()?;
    match raw.event.as_str() {
        "shown" => Some(HintEvent::Shown { count: raw.count }),
        "activated" => Some(HintEvent::Activated {
            is_input: raw.is_input,
        }),
        "open_new_tab" => {
            let url = raw.url.trim();
            if url.is_empty() {
                None
            } else {
                Some(HintEvent::OpenInNewTab {
                    url: url.to_string(),
                })
            }
        }
        "cancelled" => Some(HintEvent::Cancelled),
        "empty" => Some(HintEvent::Empty),
        _ => None,
    }
}
