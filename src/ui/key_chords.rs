use gtk::gdk;

use crate::core::{BrowserAction, KeyInput, LineEdge, Mode, WordDirection};

/// Match global zoom chords (`Ctrl++` / `Ctrl+=`, `Ctrl+-`, `Ctrl+0`).
///
/// Allows `Shift` for `+` / `=` so keyboards where `+` is shifted `=` work
/// cleanly alongside numpad `KP_Add` / `KP_Subtract`.
pub(super) fn match_zoom_chord(
    keyval: gdk::Key,
    modifiers: gdk::ModifierType,
) -> Option<BrowserAction> {
    let has_ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
    let has_other = modifiers.intersects(
        gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK | gdk::ModifierType::META_MASK,
    );
    if !has_ctrl || has_other {
        return None;
    }

    match keyval {
        gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => Some(BrowserAction::ZoomIn),
        gdk::Key::minus | gdk::Key::underscore | gdk::Key::KP_Subtract => {
            Some(BrowserAction::ZoomOut)
        }
        gdk::Key::_0 | gdk::Key::KP_0 if !modifiers.contains(gdk::ModifierType::SHIFT_MASK) => {
            Some(BrowserAction::SetZoom(1.0))
        }
        _ => None,
    }
}

/// Match editing & clipboard chords (`Ctrl+C/X/V/A/E`).
pub(super) fn match_ctrl_editing_chord(
    mode: Mode,
    keyval: gdk::Key,
    modifiers: gdk::ModifierType,
) -> Option<BrowserAction> {
    let has_ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
    let has_other = modifiers.intersects(
        gdk::ModifierType::SHIFT_MASK
            | gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::META_MASK,
    );
    if !has_ctrl || has_other {
        return None;
    }

    match (mode, keyval) {
        (Mode::Normal | Mode::Insert, gdk::Key::c | gdk::Key::C) => {
            Some(BrowserAction::CopySelection)
        }
        (Mode::Insert, gdk::Key::x | gdk::Key::X) => Some(BrowserAction::CutSelection),
        (Mode::Insert, gdk::Key::v | gdk::Key::V) => Some(BrowserAction::PasteClipboard),
        (Mode::Insert, gdk::Key::a | gdk::Key::A) => {
            Some(BrowserAction::MoveLineEdge(LineEdge::Start))
        }
        (Mode::Insert, gdk::Key::e | gdk::Key::E) => {
            Some(BrowserAction::MoveLineEdge(LineEdge::End))
        }
        _ => None,
    }
}

/// Match `Ctrl+Arrow` and `Ctrl+Shift+Arrow` word-wise cursor movement.
pub(super) fn match_word_motion(
    keyval: gdk::Key,
    modifiers: gdk::ModifierType,
) -> Option<BrowserAction> {
    let has_ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
    let has_other = modifiers.intersects(
        gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK | gdk::ModifierType::META_MASK,
    );
    if !has_ctrl || has_other {
        return None;
    }

    let direction = match keyval {
        gdk::Key::Left | gdk::Key::KP_Left => WordDirection::Left,
        gdk::Key::Right | gdk::Key::KP_Right => WordDirection::Right,
        _ => return None,
    };
    let select = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
    Some(BrowserAction::MoveWord { direction, select })
}

pub(super) fn translate_key(keyval: gdk::Key) -> KeyInput {
    if keyval == gdk::Key::Escape {
        KeyInput::Escape
    } else if keyval == gdk::Key::BackSpace {
        KeyInput::Backspace
    } else if keyval == gdk::Key::F5 {
        KeyInput::Reload
    } else if let Some(ch) = keyval.to_unicode() {
        KeyInput::Char(ch)
    } else {
        KeyInput::Other
    }
}

pub(super) fn has_non_text_modifier(modifiers: gdk::ModifierType) -> bool {
    modifiers.intersects(
        gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::META_MASK,
    )
}

pub(super) fn is_modifier_only(keyval: gdk::Key) -> bool {
    matches!(
        keyval,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Hyper_L
            | gdk::Key::Hyper_R
            | gdk::Key::Caps_Lock
            | gdk::Key::Num_Lock
            | gdk::Key::ISO_Level3_Shift
            | gdk::Key::ISO_Level5_Shift
    )
}
