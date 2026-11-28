use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::core::{
    BrowserAction, BrowserConfig, KEY_SEQUENCE_TIMEOUT, KeyMapper, KeyOutcome, Mode,
};
use crate::ui::im_bridge::{ImBridge, ImOutcome};
use crate::ui::key_chords::{
    has_non_text_modifier, is_modifier_only, match_ctrl_editing_chord, match_word_motion,
    match_zoom_chord, translate_key,
};

/// Callbacks connecting GTK key capture events to the browser window controller.
pub struct KeyCallbacks {
    pub on_action: Rc<dyn Fn(BrowserAction)>,
    pub on_pending_changed: Rc<dyn Fn(Option<String>)>,
    pub is_command_entry_empty: Rc<dyn Fn() -> bool>,
    pub is_inspector_focused: Rc<dyn Fn() -> bool>,
}

/// Bridges GTK4 Capture-phase key events and a 400ms `glib` timeout to [`KeyMapper`].
#[derive(Clone)]
pub struct KeyHandler {
    mapper: Rc<RefCell<KeyMapper>>,
    timer_gen: Rc<Cell<u64>>,
    im: ImBridge,
    mode: Rc<Cell<Mode>>,
}

impl KeyHandler {
    pub fn new(mode: Rc<Cell<Mode>>, config: BrowserConfig) -> Self {
        Self {
            mapper: Rc::new(RefCell::new(KeyMapper::with_config(config))),
            timer_gen: Rc::new(Cell::new(0)),
            im: ImBridge::new(),
            mode,
        }
    }

    /// Live-reload keybindings after the user saves or resets `about:config`.
    pub fn set_config(&self, config: BrowserConfig) {
        self.cancel_timer();
        self.mapper.borrow_mut().set_config(config);
    }

    /// Cancel any buffered multi-key sequence and its 400ms expiration timer.
    pub fn reset_pending(&self) {
        self.cancel_timer();
        self.mapper.borrow_mut().clear_pending();
    }

    /// Notify the GTK input method context when entering or leaving Insert mode.
    pub fn set_input_active(&self, active: bool) {
        self.im.set_active(active);
    }

    /// Attach a Capture-phase [`gtk::EventControllerKey`] to `window`.
    pub fn attach(&self, window: &gtk::ApplicationWindow, callbacks: KeyCallbacks) {
        let on_action_async = callbacks.on_action.clone();
        let mode_async = self.mode.clone();
        self.im.attach(
            window,
            Rc::new(move |text| {
                if mode_async.get() == Mode::Insert {
                    on_action_async(BrowserAction::InsertText(text));
                }
            }),
        );

        let controller = gtk::EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);

        let handler = self.clone();
        controller.connect_key_pressed(move |ctrl, keyval, _keycode, modifiers| {
            handler.on_key_pressed(ctrl, keyval, modifiers, &callbacks)
        });

        let handler_rel = self.clone();
        controller.connect_key_released(move |ctrl, _keyval, _keycode, _modifiers| {
            if handler_rel.mode.get() == Mode::Insert {
                handler_rel
                    .im
                    .handle_key_release(ctrl.current_event().as_ref());
            }
        });

        window.add_controller(controller);
    }

    fn on_key_pressed(
        &self,
        controller: &gtk::EventControllerKey,
        keyval: gdk::Key,
        modifiers: gdk::ModifierType,
        callbacks: &KeyCallbacks,
    ) -> glib::Propagation {
        if is_modifier_only(keyval) {
            return glib::Propagation::Proceed;
        }

        // Global zoom chords: Ctrl++ / Ctrl+= (zoom in), Ctrl+- (zoom out), Ctrl+0 (reset).
        if let Some(action) = match_zoom_chord(keyval, modifiers) {
            (callbacks.on_action)(action);
            return glib::Propagation::Stop;
        }

        let current_mode = self.mode.get();

        // When the built-in inspector drawer has keyboard focus, let it receive
        // native GTK typing/editing; Esc returns focus to the page in Normal mode.
        if (callbacks.is_inspector_focused)() {
            if keyval == gdk::Key::Escape {
                (callbacks.on_action)(BrowserAction::EnterMode(Mode::Normal));
                return glib::Propagation::Stop;
            }
            return glib::Propagation::Proceed;
        }

        // In Command or Search mode, Backspace on an empty entry exits back to Normal mode.
        if matches!(current_mode, Mode::Command | Mode::Search)
            && keyval == gdk::Key::BackSpace
            && (callbacks.is_command_entry_empty)()
        {
            (callbacks.on_action)(BrowserAction::EnterMode(Mode::Normal));
            return glib::Propagation::Stop;
        }

        // Editing & clipboard chords: Ctrl+C in Normal/Insert; Ctrl+X/V/A/E in Insert.
        if let Some(action) = match_ctrl_editing_chord(current_mode, keyval, modifiers) {
            (callbacks.on_action)(action);
            return glib::Propagation::Stop;
        }

        // In Insert mode, Ctrl+Arrow / Ctrl+Shift+Arrow moves or selects by word.
        if current_mode == Mode::Insert
            && let Some(action) = match_word_motion(keyval, modifiers)
        {
            (callbacks.on_action)(action);
            return glib::Propagation::Stop;
        }

        // Allow WM / system chords (Ctrl, Alt, Super, Meta) to pass through.
        if has_non_text_modifier(modifiers) {
            return glib::Propagation::Proceed;
        }

        // In Insert mode, Escape returns to Normal; everything else goes through
        // the IME / UTF-16 dead-key bridge before reaching servo-gtk.
        if current_mode == Mode::Insert {
            if keyval == gdk::Key::Escape {
                (callbacks.on_action)(BrowserAction::EnterMode(Mode::Normal));
                return glib::Propagation::Stop;
            }
            let event = controller.current_event();
            return match self.im.handle_key_press(event.as_ref(), keyval) {
                ImOutcome::InsertText(text) => {
                    (callbacks.on_action)(BrowserAction::InsertText(text));
                    glib::Propagation::Stop
                }
                ImOutcome::Composing => glib::Propagation::Stop,
                ImOutcome::PassThrough => glib::Propagation::Proceed,
            };
        }

        let input = translate_key(keyval);
        let outcome = self
            .mapper
            .borrow_mut()
            .handle_key(current_mode, input, Instant::now());

        match outcome {
            KeyOutcome::Action(action) => {
                self.cancel_timer();
                (callbacks.on_pending_changed)(None);
                (callbacks.on_action)(action);
                glib::Propagation::Stop
            }
            KeyOutcome::Pending(prefix) => {
                let timeout = self
                    .mapper
                    .borrow()
                    .pending_timeout()
                    .unwrap_or(KEY_SEQUENCE_TIMEOUT);
                self.schedule_timeout(timeout, callbacks.on_pending_changed.clone());
                (callbacks.on_pending_changed)(Some(prefix));
                glib::Propagation::Stop
            }
            KeyOutcome::Consumed => {
                self.cancel_timer();
                (callbacks.on_pending_changed)(None);
                glib::Propagation::Stop
            }
            KeyOutcome::PassThrough => glib::Propagation::Proceed,
        }
    }

    fn schedule_timeout(&self, timeout: Duration, on_pending_changed: Rc<dyn Fn(Option<String>)>) {
        let next_gen = self.timer_gen.get().wrapping_add(1);
        self.timer_gen.set(next_gen);

        let mapper = self.mapper.clone();
        let timer_gen = self.timer_gen.clone();
        glib::timeout_add_local_once(timeout, move || {
            if timer_gen.get() != next_gen {
                return;
            }
            mapper.borrow_mut().clear_pending();
            on_pending_changed(None);
        });
    }

    fn cancel_timer(&self) {
        self.timer_gen.set(self.timer_gen.get().wrapping_add(1));
    }
}
