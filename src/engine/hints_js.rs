/// Name of the `servo-gtk` `UserContentManager` script message handler for hints.
pub const HINT_MESSAGE_HANDLER: &str = "nservoHints";

/// JavaScript executed in the page when entering Hint mode (`f` or `F`).
///
/// When `new_tab` is `false` (`f`), hints all visible links, buttons, and inputs
/// in the viewport. When `new_tab` is `true` (`F`), hints navigable links only
/// and opens the selected link in a new browser tab.
pub fn activate_hints_script(new_tab: bool) -> String {
    let new_tab_js = if new_tab { "true" } else { "false" };
    let accent = if new_tab { "#7aa2f7" } else { "#e0af68" };
    let tint = if new_tab {
        "rgba(122,162,247,0.16)"
    } else {
        "rgba(224,175,104,0.14)"
    };
    format!(
        r#"(function () {{
  var OVERLAY_ID = '__nservo_hint_overlay__';
  var old = document.getElementById(OVERLAY_ID);
  if (old) old.remove();
  window.__nservoHints = [];
  window.__nservoHintsNewTab = {new_tab_js};

  function notify(payload) {{
    if (window.servoGtk && window.servoGtk.messageHandlers && window.servoGtk.messageHandlers.nservoHints) {{
      window.servoGtk.messageHandlers.nservoHints.postMessage(payload);
    }}
  }}

  var selector = {new_tab_js}
    ? 'a[href],area[href]'
    : [
        'a[href]', 'a[onclick]', 'a[jsaction]', 'a[tabindex]:not([tabindex="-1"])',
        'area[href]', 'button', 'input:not([type="hidden"])', 'textarea', 'select', 'summary',
        '[role="link"]', '[role="button"]', '[role="tab"]', '[role="menuitem"]',
        '[role="textbox"]', '[role="searchbox"]', '[role="combobox"]',
        '[contenteditable="true"]', '[contenteditable=""]'
      ].join(',');

  function collectNodes(root, out) {{
    if (!root || !root.querySelectorAll) return;
    var found = root.querySelectorAll(selector);
    for (var i = 0; i < found.length; i++) out.push(found[i]);
    var all = root.querySelectorAll('*');
    for (var j = 0; j < all.length; j++) {{
      if (all[j].shadowRoot) collectNodes(all[j].shadowRoot, out);
    }}
  }}

  function rectInView(r, vw, vh) {{
    return r && r.width >= 2 && r.height >= 2
      && r.bottom > 0 && r.right > 0 && r.top < vh && r.left < vw;
  }}

  function findVisibleRect(el, vw, vh) {{
    var r = el.getBoundingClientRect();
    if (rectInView(r, vw, vh)) return r;
    if (el.getClientRects) {{
      var rects = el.getClientRects();
      for (var k = 0; k < rects.length; k++) {{
        if (rectInView(rects[k], vw, vh)) return rects[k];
      }}
    }}
    var kids = el.querySelectorAll ? el.querySelectorAll('*') : [];
    var limit = Math.min(kids.length, 12);
    for (var c = 0; c < limit; c++) {{
      var kr = kids[c].getBoundingClientRect();
      if (rectInView(kr, vw, vh)) return kr;
    }}
    return null;
  }}

  var nodes = [];
  collectNodes(document, nodes);
  var vw = window.innerWidth || document.documentElement.clientWidth || 0;
  var vh = window.innerHeight || document.documentElement.clientHeight || 0;
  var targets = [];

  for (var i = 0; i < nodes.length; i++) {{
    var el = nodes[i];
    if (el.disabled) continue;
    var style = window.getComputedStyle(el);
    if (!style || style.display === 'none' || style.visibility === 'hidden' || parseFloat(style.opacity || '1') <= 0) {{
      continue;
    }}
    var rect = findVisibleRect(el, vw, vh);
    if (!rect) continue;
    targets.push({{ el: el, rect: rect }});
  }}

  if (targets.length === 0) {{
    notify({{ event: 'empty' }});
    return;
  }}

  var alphabet = 'abcdefghijklmnopqrstuvwxyz';
  var base = alphabet.length;
  var len = 1;
  var cap = base;
  while (cap < targets.length) {{
    len += 1;
    cap *= base;
  }}

  function makeLabel(idx) {{
    var chars = new Array(len);
    for (var p = len - 1; p >= 0; p--) {{
      chars[p] = alphabet[idx % base];
      idx = Math.floor(idx / base);
    }}
    return chars.join('');
  }}

  var container = document.createElement('div');
  container.id = OVERLAY_ID;
  container.style.cssText = 'position:fixed;left:0;top:0;width:0;height:0;z-index:2147483647;pointer-events:none;';

  var items = [];
  for (var j = 0; j < targets.length; j++) {{
    var label = makeLabel(j);
    var t = targets[j];
    var r = t.rect;

    var group = document.createElement('div');
    group.style.cssText = 'position:fixed;left:0;top:0;pointer-events:none;';

    var outline = document.createElement('div');
    outline.style.cssText =
      'position:fixed;box-sizing:border-box;pointer-events:none;' +
      'left:' + Math.round(r.left) + 'px;' +
      'top:' + Math.round(r.top) + 'px;' +
      'width:' + Math.round(r.width) + 'px;' +
      'height:' + Math.round(r.height) + 'px;' +
      'border:1px solid {accent};' +
      'background:{tint};' +
      'border-radius:2px;';

    var bx = Math.max(0, Math.min(vw - 24, Math.round(r.left)));
    var by = Math.max(0, Math.min(vh - 20, Math.round(r.top)));
    var badge = document.createElement('div');
    badge.textContent = label;
    badge.style.cssText =
      'position:fixed;pointer-events:none;' +
      'left:' + bx + 'px;' +
      'top:' + by + 'px;' +
      'background:{accent};color:#16161e;' +
      'border:1px solid #16161e;border-radius:3px;' +
      'font:bold 12px/1 monospace;padding:2px 5px;' +
      'box-shadow:0 1px 3px rgba(0,0,0,0.45);';

    group.appendChild(outline);
    group.appendChild(badge);
    container.appendChild(group);
    items.push({{ label: label, el: t.el, group: group, badge: badge }});
  }}

  var host = document.body || document.documentElement;
  host.appendChild(container);
  window.__nservoHints = items;
  notify({{ event: 'shown', count: items.length }});
}})();"#
    )
}

/// JavaScript executed as the user types hint characters (or presses Backspace).
pub fn filter_hints_script(typed: &str) -> String {
    let typed_json = serde_json::to_string(typed).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"(function () {{
  var OVERLAY_ID = '__nservo_hint_overlay__';
  var typed = {typed_json};
  var items = window.__nservoHints || [];

  function notify(payload) {{
    if (window.servoGtk && window.servoGtk.messageHandlers && window.servoGtk.messageHandlers.nservoHints) {{
      window.servoGtk.messageHandlers.nservoHints.postMessage(payload);
    }}
  }}

  function clearAll() {{
    var overlay = document.getElementById(OVERLAY_ID);
    if (overlay) overlay.remove();
    window.__nservoHints = [];
  }}

  function isTextInput(el) {{
    if (!el) return false;
    var tag = (el.tagName || '').toLowerCase();
    if (tag === 'textarea' || tag === 'select') return true;
    if (el.isContentEditable) return true;
    var role = (el.getAttribute && el.getAttribute('role') || '').toLowerCase();
    if (role === 'textbox' || role === 'searchbox' || role === 'combobox') return true;
    if (tag === 'input') {{
      var type = (el.getAttribute('type') || 'text').toLowerCase();
      var nonText = ['button', 'submit', 'reset', 'checkbox', 'radio', 'image', 'file', 'color', 'range'];
      return nonText.indexOf(type) === -1;
    }}
    return false;
  }}

  var exact = null;
  var remaining = 0;

  for (var i = 0; i < items.length; i++) {{
    var item = items[i];
    if (item.label === typed) {{
      exact = item;
      break;
    }}
    if (item.label.indexOf(typed) === 0) {{
      item.group.style.display = '';
      item.badge.textContent = item.label.slice(typed.length) || item.label;
      remaining += 1;
    }} else {{
      item.group.style.display = 'none';
    }}
  }}

  if (exact) {{
    var el = exact.el;
    var openNewTab = !!window.__nservoHintsNewTab;
    clearAll();
    if (openNewTab) {{
      var anchor = (el.closest && el.closest('a[href],area[href]')) || el;
      var href = (anchor && typeof anchor.href === 'string') ? anchor.href : '';
      if (href && href.toLowerCase().indexOf('javascript:') !== 0) {{
        notify({{ event: 'open_new_tab', url: href }});
        return;
      }}
    }}
    var isInput = isTextInput(el);
    try {{ el.focus(); }} catch (_) {{}}
    try {{ el.click(); }} catch (_) {{}}
    if (isInput && typeof el.value === 'string' && typeof el.setSelectionRange === 'function') {{
      try {{ el.setSelectionRange(el.value.length, el.value.length); }} catch (_) {{}}
    }}
    notify({{ event: 'activated', isInput: isInput }});
    return;
  }}

  if (remaining === 0) {{
    clearAll();
    notify({{ event: 'cancelled' }});
    return;
  }}
}})();"#
    )
}

/// JavaScript executed when cancelling Hint mode (`Esc` or tab switch).
pub fn clear_hints_script() -> &'static str {
    r#"(function () {
  var overlay = document.getElementById('__nservo_hint_overlay__');
  if (overlay) overlay.remove();
  window.__nservoHints = [];
  window.__nservoHintsNewTab = false;
})();"#
}
