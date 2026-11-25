/// Build JavaScript that inserts UTF-16 code units into the currently focused
/// `<input>`, `<textarea>`, or `[contenteditable]` element on the page.
///
/// Using `String.fromCharCode(...)` reconstructs the exact UTF-16 string
/// (including accented characters and surrogate pairs) without escaping issues,
/// updates the native DOM value setter so frameworks (React/Vue/Google) react,
/// dispatches an `input` event, and flushes Servo's software compositor.
pub fn insert_utf16_script(units: &[u16]) -> String {
    let args = units
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"(function () {{
  var text = String.fromCharCode({args});
  if (!text) return;
  var el = document.activeElement;
  if (!el) return;
  var tag = (el.tagName || '').toLowerCase();

  function flush(target) {{
    var host = document.body || document.documentElement || target;
    if (!host) return;
    void host.offsetHeight;
    setTimeout(function () {{ host.style.outline = '1px solid transparent'; }}, 20);
    setTimeout(function () {{ host.style.outline = ''; }}, 60);
  }}

  if (tag === 'input' || tag === 'textarea') {{
    var val = el.value || '';
    var start = typeof el.selectionStart === 'number' ? el.selectionStart : val.length;
    var end = typeof el.selectionEnd === 'number' ? el.selectionEnd : val.length;
    var next = val.slice(0, start) + text + val.slice(end);

    var proto = tag === 'textarea' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    var desc = Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && typeof desc.set === 'function') {{
      desc.set.call(el, next);
    }} else {{
      el.value = next;
    }}

    var caret = start + text.length;
    try {{ el.setSelectionRange(caret, caret); }} catch (_) {{}}

    try {{
      el.dispatchEvent(new InputEvent('input', {{ bubbles: true, data: text, inputType: 'insertText' }}));
    }} catch (_) {{
      el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    }}
    flush(el);
    return;
  }}

  if (el.isContentEditable) {{
    var inserted = false;
    try {{
      inserted = document.execCommand && document.execCommand('insertText', false, text);
    }} catch (_) {{}}
    if (!inserted) {{
      var sel = window.getSelection && window.getSelection();
      if (sel && sel.rangeCount > 0) {{
        var range = sel.getRangeAt(0);
        range.deleteContents();
        var node = document.createTextNode(text);
        range.insertNode(node);
        range.setStartAfter(node);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
      }}
    }}
    try {{
      el.dispatchEvent(new InputEvent('input', {{ bubbles: true, data: text, inputType: 'insertText' }}));
    }} catch (_) {{
      el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    }}
    flush(el);
  }}
}})();"#
    )
}
