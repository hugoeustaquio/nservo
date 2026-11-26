/// Script message handler name registered on each tab's `UserContentManager`.
pub const INSPECTOR_MESSAGE_HANDLER: &str = "nservoInspector";

/// UserScript injected into every page load to capture `console.*` output and
/// uncaught JS errors without interfering with `servo-gtk`'s internal IPC prefix.
pub fn inspector_init_script() -> &'static str {
    r#"(function() {
  if (window.__nservoInspector) return;

  var IPC_PREFIX = '__servo_gtk_msg__';
  var sending = false;

  function post(payload) {
    if (sending) return;
    var h = window.servoGtk && window.servoGtk.messageHandlers
      && window.servoGtk.messageHandlers.nservoInspector;
    if (!h) return;
    sending = true;
    try { h.postMessage(payload); } catch (_) {}
    sending = false;
  }

  function formatValue(val, depth) {
    depth = depth || 0;
    if (val === null) return 'null';
    if (val === undefined) return 'undefined';
    var t = typeof val;
    if (t === 'string') return depth === 0 ? val : JSON.stringify(val);
    if (t === 'number' || t === 'boolean' || t === 'bigint' || t === 'symbol') return String(val);
    if (t === 'function') return '[Function ' + (val.name || 'anonymous') + ']';
    if (val instanceof Error) return val.stack || (val.name + ': ' + val.message);
    if (val && val.nodeType === 1) {
      var s = '<' + val.tagName.toLowerCase();
      if (val.id) s += '#' + val.id;
      if (typeof val.className === 'string' && val.className.trim()) {
        s += '.' + val.className.trim().split(/\s+/).slice(0, 2).join('.');
      }
      return s + '>';
    }
    try {
      var seen = new WeakSet();
      var json = JSON.stringify(val, function(_k, v) {
        if (typeof v === 'bigint') return String(v) + 'n';
        if (typeof v === 'function') return '[Function ' + (v.name || 'anonymous') + ']';
        if (v && typeof v === 'object') {
          if (seen.has(v)) return '[Circular]';
          seen.add(v);
        }
        return v;
      });
      if (json && json.length > 1500) json = json.slice(0, 1500) + '...';
      return json || String(val);
    } catch (_) {
      try { return String(val); } catch (_e) { return '[Unprintable]'; }
    }
  }

  ['log', 'info', 'warn', 'error', 'debug'].forEach(function(level) {
    var orig = console[level];
    if (typeof orig !== 'function') return;
    console[level] = function() {
      var first = arguments.length > 0 ? arguments[0] : null;
      var isInternalIpc = typeof first === 'string' && first.indexOf(IPC_PREFIX) === 0;
      if (!isInternalIpc && !sending) {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
          parts.push(formatValue(arguments[i], 0));
        }
        post({ kind: 'console', level: level, text: parts.join(' ') });
      }
      return orig.apply(console, arguments);
    };
  });

  window.addEventListener('error', function(ev) {
    var loc = ev.filename ? (' (' + ev.filename + ':' + ev.lineno + ':' + ev.colno + ')') : '';
    var msg = (ev.error && ev.error.stack) || ((ev.message || 'Uncaught Error') + loc);
    post({ kind: 'console', level: 'error', text: msg });
  });

  window.addEventListener('unhandledrejection', function(ev) {
    post({
      kind: 'console',
      level: 'error',
      text: 'Unhandled Promise Rejection: ' + formatValue(ev.reason, 0)
    });
  });

  window.__nservoInspector = { post: post, formatValue: formatValue };
})();"#
}

/// Evaluate a JavaScript expression in the page's global scope and post its
/// formatted result (or exception) back to `nservoInspector`.
pub fn eval_in_page_script(expression: &str) -> String {
    let encoded = serde_json::to_string(expression).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"(function() {{
  var ins = window.__nservoInspector;
  var post = ins ? ins.post : function(p) {{
    var h = window.servoGtk && window.servoGtk.messageHandlers
      && window.servoGtk.messageHandlers.nservoInspector;
    if (h) h.postMessage(p);
  }};
  var fmt = ins ? ins.formatValue : function(v) {{
    try {{ return typeof v === 'string' ? JSON.stringify(v) : String(v); }} catch (_) {{ return '[Unprintable]'; }}
  }};
  var code = {encoded};
  try {{
    var result = (0, eval)(code);
    var text = (typeof result === 'string') ? JSON.stringify(result) : fmt(result, 0);
    post({{ kind: 'console', level: 'result', text: text }});
  }} catch (err) {{
    var msg = (err && err.stack) ? err.stack : String(err);
    post({{ kind: 'console', level: 'error', text: msg }});
  }}
  var host = document.body || document.documentElement;
  if (host) {{
    var prev = host.style.opacity;
    host.style.opacity = '0.999';
    setTimeout(function() {{ host.style.opacity = prev; }}, 25);
  }}
}})();"#
    )
}

/// Walk the visible DOM tree (capped for responsiveness) and post a flattened
/// list of elements back to `nservoInspector`.
pub fn snapshot_dom_script() -> &'static str {
    r#"(function() {
  var h = window.servoGtk && window.servoGtk.messageHandlers
    && window.servoGtk.messageHandlers.nservoInspector;
  if (!h || !document.documentElement) return;

  var MAX_NODES = 250;
  var MAX_DEPTH = 12;
  var nodes = [];
  var refs = [];

  function directText(el) {
    var out = '';
    for (var i = 0; i < el.childNodes.length; i++) {
      if (el.childNodes[i].nodeType === 3) {
        out += el.childNodes[i].nodeValue || '';
      }
    }
    out = out.replace(/\s+/g, ' ').trim();
    return out.length > 48 ? out.slice(0, 48) + '...' : out;
  }

  function walk(el, depth) {
    if (!el || nodes.length >= MAX_NODES || depth > MAX_DEPTH) return;
    if (el.id === 'nservo-hint-overlay' || el.id === 'nservo-inspector-highlight') return;
    var index = refs.length;
    refs.push(el);
    nodes.push({
      index: index,
      depth: depth,
      tag: el.tagName ? el.tagName.toLowerCase() : 'node',
      id: el.id || '',
      classes: typeof el.className === 'string' ? el.className.trim() : '',
      text: directText(el)
    });
    var children = el.children || [];
    for (var i = 0; i < children.length && nodes.length < MAX_NODES; i++) {
      walk(children[i], depth + 1);
    }
  }

  walk(document.documentElement, 0);
  window.__nservoDomNodes = refs;
  h.postMessage({ kind: 'dom_tree', nodes: nodes });
})();"#
}

/// Inspect the `index`-th element from the most recent DOM snapshot, highlight
/// its bounding box on the page, and post attributes + computed styles back.
pub fn inspect_element_script(index: usize) -> String {
    format!(
        r#"(function() {{
  var h = window.servoGtk && window.servoGtk.messageHandlers
    && window.servoGtk.messageHandlers.nservoInspector;
  var refs = window.__nservoDomNodes || [];
  var el = refs[{index}];
  if (!h || !el) return;

  var sel = el.tagName.toLowerCase();
  if (el.id) sel += '#' + el.id;
  if (typeof el.className === 'string' && el.className.trim()) {{
    sel += '.' + el.className.trim().split(/\s+/).join('.');
  }}

  var attrs = [];
  if (el.attributes) {{
    for (var i = 0; i < el.attributes.length; i++) {{
      var a = el.attributes[i];
      attrs.push([a.name, a.value]);
    }}
  }}

  var r = el.getBoundingClientRect();
  var rect = Math.round(r.width) + 'x' + Math.round(r.height)
    + ' @ (' + Math.round(r.left) + ', ' + Math.round(r.top) + ')';

  var cs = window.getComputedStyle(el);
  var props = [
    'display', 'position', 'width', 'height', 'margin', 'padding',
    'color', 'background-color', 'font-family', 'font-size', 'z-index', 'opacity'
  ];
  var styles = [];
  for (var j = 0; j < props.length; j++) {{
    styles.push([props[j], cs.getPropertyValue(props[j])]);
  }}

  var html = (el.outerHTML || '').slice(0, 600);
  if ((el.outerHTML || '').length > 600) html += '...';

  var old = document.getElementById('nservo-inspector-highlight');
  if (old) old.remove();
  var host = document.body || document.documentElement;
  if (host && r.width > 0 && r.height > 0) {{
    var box = document.createElement('div');
    box.id = 'nservo-inspector-highlight';
    box.style.cssText = 'position:fixed;pointer-events:none;z-index:2147483646;'
      + 'border:2px solid #7aa2f7;background:rgba(122,162,247,0.22);box-sizing:border-box;'
      + 'left:' + r.left + 'px;top:' + r.top + 'px;'
      + 'width:' + r.width + 'px;height:' + r.height + 'px;';
    host.appendChild(box);
    void host.offsetHeight;
    setTimeout(function() {{ box.style.opacity = '0.99'; }}, 20);
    setTimeout(function() {{ box.style.opacity = '1'; }}, 60);
  }}

  h.postMessage({{
    kind: 'element_details',
    selector: sel,
    rect: rect,
    attributes: attrs,
    styles: styles,
    outer_html: html
  }});
}})();"#
    )
}

/// Remove any active DOM inspector highlight overlay from the page.
pub fn clear_highlight_script() -> &'static str {
    r#"(function() {
  var old = document.getElementById('nservo-inspector-highlight');
  if (old) {
    old.remove();
    var host = document.body || document.documentElement;
    if (host) {
      var prev = host.style.opacity;
      host.style.opacity = '0.999';
      setTimeout(function() { host.style.opacity = prev; }, 25);
    }
  }
})();"#
}
