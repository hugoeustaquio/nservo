use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;

use crate::core::{DeadKey, DeadKeyComposer, needs_utf16_injection};

/// Outcome of passing an Input-mode key event through the IME & UTF-16 composer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImOutcome {
    /// Inject this committed Unicode/UTF-16 text into the focused page input.
    InsertText(String),
    /// Key was consumed by an active dead-key or IME composition sequence.
    Composing,
    /// Let `servo-gtk` handle the raw key event natively (plain ASCII / control keys).
    PassThrough,
}

/// Bridges dead-key composition ([`DeadKeyComposer`]) and GTK4's [`gtk::IMMulticontext`]
/// so Portuguese accents, dead keys, direct Unicode keys, and system IMEs work
/// reliably inside `servo-gtk` inputs on both Wayland and X11.
#[derive(Clone)]
pub struct ImBridge {
    im: gtk::IMMulticontext,
    composer: Rc<RefCell<DeadKeyComposer>>,
    in_im_composition: Rc<Cell<bool>>,
    filtering: Rc<Cell<bool>>,
    sync_commit: Rc<RefCell<Option<String>>>,
}

impl ImBridge {
    pub fn new() -> Self {
        Self {
            im: gtk::IMMulticontext::new(),
            composer: Rc::new(RefCell::new(DeadKeyComposer::new())),
            in_im_composition: Rc::new(Cell::new(false)),
            filtering: Rc::new(Cell::new(false)),
            sync_commit: Rc::new(RefCell::new(None)),
        }
    }

    pub fn attach(&self, window: &gtk::ApplicationWindow, on_async_commit: Rc<dyn Fn(String)>) {
        self.im.set_client_widget(Some(window));

        let filtering = self.filtering.clone();
        let sync_commit = self.sync_commit.clone();
        let in_comp = self.in_im_composition.clone();
        self.im.connect_commit(move |_, text| {
            if text.is_empty() {
                return;
            }
            if filtering.get() {
                *sync_commit.borrow_mut() = Some(text.to_string());
            } else {
                in_comp.set(false);
                on_async_commit(text.to_string());
            }
        });
    }

    pub fn set_active(&self, active: bool) {
        if active {
            self.im.focus_in();
        } else {
            self.im.focus_out();
            self.im.reset();
            self.in_im_composition.set(false);
            self.composer.borrow_mut().reset();
        }
    }

    pub fn handle_key_press(&self, event: Option<&gdk::Event>, keyval: gdk::Key) -> ImOutcome {
        // 1. Handle dead keys directly so Wayland/X11 IM quirks never swallow them.
        if let Some(dead) = gdk_dead_key(keyval) {
            return match self.composer.borrow_mut().push_dead_key(dead) {
                Some(text) => ImOutcome::InsertText(text),
                None => ImOutcome::Composing,
            };
        }

        // 2. If a dead key is pending, compose it with the next printable key.
        if self.composer.borrow().has_pending() {
            if keyval == gdk::Key::BackSpace {
                self.composer.borrow_mut().reset();
                return ImOutcome::Composing;
            }
            if let Some(ch) = keyval.to_unicode()
                && !ch.is_control()
                && let Some(composed) = self.composer.borrow_mut().push_char(ch)
            {
                return ImOutcome::InsertText(composed);
            }
            self.composer.borrow_mut().reset();
            return ImOutcome::PassThrough;
        }

        // 3. Direct non-ASCII Unicode keys (e.g. 'ç' on Brazilian ABNT2).
        if let Some(ch) = keyval.to_unicode()
            && !ch.is_ascii()
            && !ch.is_control()
        {
            return ImOutcome::InsertText(ch.to_string());
        }

        // 4. Delegate remaining keys to GTK's system IME (Compose, IBus, Fcitx, emoji).
        if let Some(ev) = event {
            *self.sync_commit.borrow_mut() = None;
            self.filtering.set(true);
            let handled = self.im.filter_keypress(ev);
            self.filtering.set(false);

            let committed = self.sync_commit.borrow_mut().take();
            if let Some(text) = committed {
                let was_composed = self.in_im_composition.replace(false);
                return if needs_utf16_injection(&text, was_composed) {
                    ImOutcome::InsertText(text)
                } else {
                    ImOutcome::PassThrough
                };
            }
            if handled {
                self.in_im_composition.set(true);
                return ImOutcome::Composing;
            }
        }

        ImOutcome::PassThrough
    }

    pub fn handle_key_release(&self, event: Option<&gdk::Event>) {
        if self.composer.borrow().has_pending() {
            return;
        }
        if let Some(ev) = event {
            self.im.filter_keypress(ev);
        }
    }
}

impl Default for ImBridge {
    fn default() -> Self {
        Self::new()
    }
}

fn gdk_dead_key(keyval: gdk::Key) -> Option<DeadKey> {
    match keyval {
        gdk::Key::dead_acute => Some(DeadKey::Acute),
        gdk::Key::dead_grave => Some(DeadKey::Grave),
        gdk::Key::dead_circumflex => Some(DeadKey::Circumflex),
        gdk::Key::dead_tilde => Some(DeadKey::Tilde),
        gdk::Key::dead_diaeresis => Some(DeadKey::Diaeresis),
        gdk::Key::dead_cedilla => Some(DeadKey::Cedilla),
        _ => None,
    }
}
