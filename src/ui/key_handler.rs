use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::core::{BrowserAction, KEY_SEQUENCE_TIMEOUT, KeyInput, KeyMapper, KeyOutcome, Mode};
use crate::ui::im_bridge::{ImBridge, ImOutcome};

/// Callbacks connecting GTK key capture events to the browser window controller.
pub struct KeyCallbacks {
    pub on_action: Rc<dyn Fn(BrowserAction)>,
    pub on_pending_changed: Rc<dyn Fn(Option<String>)>,
    pub is_command_entry_empty: Rc<dyn Fn() -> bool>,
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
    pub fn new(mode: Rc<Cell<Mode>>) -> Self {
        Self {
            mapper: Rc::new(RefCell::new(KeyMapper::new())),
            timer_gen: Rc::new(Cell::new(0)),
            im: ImBridge::new(),
            mode,
        }
    }

    /// Cancel any buffered multi-key sequence and its 400ms expiration timer.
    pub fn reset_pending(&self) {
        self.cancel_timer();
        self.mapper.borrow_mut().clear_pending();
    }

    /// Notify the GTK input method context when entering or leaving Input mode.
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
                if mode_async.get() == Mode::Input {
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
            if handler_rel.mode.get() == Mode::Input {
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

        let current_mode = self.mode.get();

        // In Command mode, Backspace on an empty entry exits back to Normal mode.
        if current_mode == Mode::Command
            && keyval == gdk::Key::BackSpace
            && (callbacks.is_command_entry_empty)()
        {
            (callbacks.on_action)(BrowserAction::EnterMode(Mode::Normal));
            return glib::Propagation::Stop;
        }

        // Allow WM / system chords (Ctrl, Alt, Super, Meta) to pass through.
        if has_non_text_modifier(modifiers) {
            return glib::Propagation::Proceed;
        }

        // In Input mode, Escape returns to Normal; everything else goes through
        // the IME / UTF-16 dead-key bridge before reaching servo-gtk.
        if current_mode == Mode::Input {
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
                self.schedule_timeout(callbacks.on_pending_changed.clone());
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

    fn schedule_timeout(&self, on_pending_changed: Rc<dyn Fn(Option<String>)>) {
        let next_gen = self.timer_gen.get().wrapping_add(1);
        self.timer_gen.set(next_gen);

        let mapper = self.mapper.clone();
        let timer_gen = self.timer_gen.clone();
        glib::timeout_add_local_once(KEY_SEQUENCE_TIMEOUT, move || {
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

fn translate_key(keyval: gdk::Key) -> KeyInput {
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

fn has_non_text_modifier(modifiers: gdk::ModifierType) -> bool {
    modifiers.intersects(
        gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::META_MASK,
    )
}

fn is_modifier_only(keyval: gdk::Key) -> bool {
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
