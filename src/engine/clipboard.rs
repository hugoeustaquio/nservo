use gtk::gdk;
use gtk::gio;
use gtk::prelude::*;
use serde::Deserialize;

use crate::engine::web_tab::WebTab;

/// Name of the `servo-gtk` `UserContentManager` handler for page-to-clipboard copy.
pub const CLIPBOARD_MESSAGE_HANDLER: &str = "nservoClipboard";

#[derive(Deserialize)]
struct ClipboardPayload {
    text: String,
}

/// Parse a JSON message payload sent by [`copy_selection_script`].
pub fn parse_clipboard_message(body: &str) -> Option<String> {
    let payload: ClipboardPayload = serde_json::from_str(body).ok()?;
    (!payload.text.is_empty()).then_some(payload.text)
}

/// JavaScript executed on `Ctrl+C` to extract the currently selected text from
/// either an `<input>`/`<textarea>` selection or the page's DOM selection.
pub fn copy_selection_script() -> &'static str {
    r#"(function () {
  var text = '';
  var el = document.activeElement;
  var tag = el && el.tagName ? el.tagName.toLowerCase() : '';
  if (
    el &&
    (tag === 'input' || tag === 'textarea') &&
    typeof el.selectionStart === 'number' &&
    typeof el.selectionEnd === 'number' &&
    el.selectionEnd > el.selectionStart
  ) {
    text = (el.value || '').slice(el.selectionStart, el.selectionEnd);
  } else if (window.getSelection) {
    var sel = window.getSelection();
    if (sel) text = sel.toString();
  }
  if (
    text &&
    window.servoGtk &&
    window.servoGtk.messageHandlers &&
    window.servoGtk.messageHandlers.nservoClipboard
  ) {
    window.servoGtk.messageHandlers.nservoClipboard.postMessage({ text: text });
  }
})();"#
}

/// JavaScript executed on `Ctrl+X` in Insert mode to copy the selected text to
/// the system clipboard and remove it from the focused `<input>`, `<textarea>`,
/// or `[contenteditable]` element.
pub fn cut_selection_script() -> &'static str {
    r#"(function () {
  var el = document.activeElement;
  if (!el) return;
  var tag = (el.tagName || '').toLowerCase();

  function notify(text) {
    if (
      text &&
      window.servoGtk &&
      window.servoGtk.messageHandlers &&
      window.servoGtk.messageHandlers.nservoClipboard
    ) {
      window.servoGtk.messageHandlers.nservoClipboard.postMessage({ text: text });
    }
  }

  function flush(target) {
    var host = document.body || document.documentElement || target;
    if (!host) return;
    void host.offsetHeight;
    setTimeout(function () { host.style.outline = '1px solid transparent'; }, 20);
    setTimeout(function () { host.style.outline = ''; }, 60);
  }

  if (tag === 'input' || tag === 'textarea') {
    if (typeof el.selectionStart !== 'number' || typeof el.selectionEnd !== 'number') return;
    var start = el.selectionStart;
    var end = el.selectionEnd;
    if (end <= start) return;
    var val = el.value || '';
    var selected = val.slice(start, end);
    var next = val.slice(0, start) + val.slice(end);

    var proto = tag === 'textarea' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    var desc = Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && typeof desc.set === 'function') {
      desc.set.call(el, next);
    } else {
      el.value = next;
    }

    try { el.setSelectionRange(start, start, 'none'); } catch (_) {}
    try {
      el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'deleteByCut' }));
    } catch (_) {
      el.dispatchEvent(new Event('input', { bubbles: true }));
    }
    notify(selected);
    flush(el);
    return;
  }

  if (el.isContentEditable && window.getSelection) {
    var sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    var selected = sel.toString();
    if (!selected) return;
    var range = sel.getRangeAt(0);
    range.deleteContents();
    range.collapse(true);
    try {
      el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'deleteByCut' }));
    } catch (_) {
      el.dispatchEvent(new Event('input', { bubbles: true }));
    }
    notify(selected);
    flush(el);
  }
})();"#
}

/// Write `text` to the default GTK4 system clipboard.
pub fn write_system_clipboard(text: &str) {
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);
    }
}

/// Asynchronously read text from the GTK4 system clipboard and insert it as
/// UTF-16 code units into the focused input of `tab`.
pub fn paste_into_tab(tab: WebTab) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    display
        .clipboard()
        .read_text_async(gio::Cancellable::NONE, move |result| {
            if let Ok(Some(text)) = result
                && !text.is_empty()
            {
                tab.insert_utf16_text(&text);
            }
        });
}
