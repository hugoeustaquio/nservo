use crate::core::{LineEdge, WordDirection};

/// Build JavaScript that inserts UTF-16 code units into the currently focused
/// `<input>`, `<textarea>`, or `[contenteditable]` element on the page.
///
/// Using `String.fromCharCode(...)` reconstructs the exact UTF-16 string
/// (including accented characters and surrogate pairs) without escaping issues,
/// updates the native DOM value setter so frameworks (React/Vue/Google) react,
/// dispatches an `input` event, and flushes Servo's software compositor.
pub fn insert_utf16_script(units: &[u16]) -> String {
    let text_expr = if units.is_empty() {
        "''".to_string()
    } else {
        units
            .chunks(4096)
            .map(|chunk| {
                let args = chunk
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("String.fromCharCode({args})")
            })
            .collect::<Vec<_>>()
            .join("+")
    };

    format!(
        r#"(function () {{
  var text = {text_expr};
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

/// Build JavaScript that moves the caret (or extends selection when `select` is
/// true) by one word in the currently focused `<input>`, `<textarea>`, or
/// `[contenteditable]` element.
pub fn move_word_script(direction: WordDirection, select: bool) -> String {
    let is_left = matches!(direction, WordDirection::Left);
    let sel_action = if select { "extend" } else { "move" };
    let sel_dir = if is_left { "backward" } else { "forward" };

    format!(
        r#"(function () {{
  var el = document.activeElement;
  if (!el) return;
  var tag = (el.tagName || '').toLowerCase();
  var isLeft = {is_left};
  var extend = {select};

  function flush(target) {{
    var host = document.body || document.documentElement || target;
    if (!host) return;
    void host.offsetHeight;
    setTimeout(function () {{ host.style.outline = '1px solid transparent'; }}, 20);
    setTimeout(function () {{ host.style.outline = ''; }}, 60);
  }}

  function isSpace(ch) {{ return /\s/.test(ch); }}
  function isWordChar(ch) {{
    try {{ return /[\p{{L}}\p{{N}}_]/u.test(ch); }}
    catch (_) {{ return /[A-Za-z0-9_\u00C0-\u024F]/.test(ch); }}
  }}

  function prevWord(val, pos) {{
    var i = Math.max(0, Math.min(pos, val.length));
    while (i > 0 && isSpace(val[i - 1])) i--;
    if (i > 0) {{
      var word = isWordChar(val[i - 1]);
      while (i > 0 && !isSpace(val[i - 1]) && isWordChar(val[i - 1]) === word) i--;
    }}
    return i;
  }}

  function nextWord(val, pos) {{
    var len = val.length;
    var i = Math.max(0, Math.min(pos, len));
    while (i < len && isSpace(val[i])) i++;
    if (i < len) {{
      var word = isWordChar(val[i]);
      while (i < len && !isSpace(val[i]) && isWordChar(val[i]) === word) i++;
    }}
    return i;
  }}

  if (tag === 'input' || tag === 'textarea') {{
    if (typeof el.selectionStart !== 'number' || typeof el.selectionEnd !== 'number') return;
    var val = el.value || '';
    var start = el.selectionStart;
    var end = el.selectionEnd;
    var dir = el.selectionDirection || 'none';

    if (!extend) {{
      var base = isLeft ? start : end;
      var next = isLeft ? prevWord(val, base) : nextWord(val, base);
      try {{ el.setSelectionRange(next, next, 'none'); }} catch (_) {{}}
    }} else {{
      var anchor = dir === 'backward' ? end : start;
      var focus = dir === 'backward' ? start : end;
      var nextFocus = isLeft ? prevWord(val, focus) : nextWord(val, focus);
      if (nextFocus <= anchor) {{
        try {{ el.setSelectionRange(nextFocus, anchor, 'backward'); }} catch (_) {{}}
      }} else {{
        try {{ el.setSelectionRange(anchor, nextFocus, 'forward'); }} catch (_) {{}}
      }}
    }}
    flush(el);
    return;
  }}

  if (el.isContentEditable && window.getSelection) {{
    var sel = window.getSelection();
    if (sel && typeof sel.modify === 'function') {{
      try {{ sel.modify('{sel_action}', '{sel_dir}', 'word'); }} catch (_) {{}}
      flush(el);
    }}
  }}
}})();"#
    )
}

/// Build JavaScript that moves the caret to the start (`Ctrl+A`) or end (`Ctrl+E`)
/// of the current line in the focused `<input>`, `<textarea>`, or `[contenteditable]`.
pub fn move_line_edge_script(edge: LineEdge) -> String {
    let to_start = matches!(edge, LineEdge::Start);
    let sel_dir = if to_start { "backward" } else { "forward" };

    format!(
        r#"(function () {{
  var el = document.activeElement;
  if (!el) return;
  var tag = (el.tagName || '').toLowerCase();
  var toStart = {to_start};

  function flush(target) {{
    var host = document.body || document.documentElement || target;
    if (!host) return;
    void host.offsetHeight;
    setTimeout(function () {{ host.style.outline = '1px solid transparent'; }}, 20);
    setTimeout(function () {{ host.style.outline = ''; }}, 60);
  }}

  if (tag === 'input' || tag === 'textarea') {{
    if (typeof el.selectionStart !== 'number' || typeof el.selectionEnd !== 'number') return;
    var val = el.value || '';
    var pos = toStart ? el.selectionStart : el.selectionEnd;
    var targetPos;
    if (tag === 'input') {{
      targetPos = toStart ? 0 : val.length;
    }} else if (toStart) {{
      var prevNl = pos > 0 ? val.lastIndexOf('\n', pos - 1) : -1;
      targetPos = prevNl === -1 ? 0 : prevNl + 1;
    }} else {{
      var nextNl = val.indexOf('\n', pos);
      targetPos = nextNl === -1 ? val.length : nextNl;
    }}
    try {{ el.setSelectionRange(targetPos, targetPos, 'none'); }} catch (_) {{}}
    flush(el);
    return;
  }}

  if (el.isContentEditable && window.getSelection) {{
    var sel = window.getSelection();
    if (sel && typeof sel.modify === 'function') {{
      try {{ sel.modify('move', '{sel_dir}', 'lineboundary'); }} catch (_) {{}}
      flush(el);
    }}
  }}
}})();"#
    )
}
