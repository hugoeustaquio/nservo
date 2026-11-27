/// Start interactive element-picker ("Find") mode inside the page.
///
/// Highlights hovered elements under the pointer and intercepts the next click
/// to capture a DOM snapshot guaranteed to contain the clicked element and its
/// full ancestor chain, posting `{ kind: 'element_picked', nodes, selected_index }`.
pub fn start_element_picker_script() -> &'static str {
    r#"(function() {
  if (typeof window.__nservoPickerStop === 'function') {
    window.__nservoPickerStop();
  }
  var h = window.servoGtk && window.servoGtk.messageHandlers
    && window.servoGtk.messageHandlers.nservoInspector;
  if (!h || !document.documentElement) return;

  var HIGHLIGHT_ID = 'nservo-inspector-highlight';
  var prevCursor = document.documentElement.style.cursor;
  document.documentElement.style.cursor = 'crosshair';
  var lastHover = null;

  function drawHighlight(el) {
    var old = document.getElementById(HIGHLIGHT_ID);
    if (!el || el.nodeType !== 1 || el.id === HIGHLIGHT_ID) {
      if (old) old.remove();
      return;
    }
    var r = el.getBoundingClientRect();
    if (r.width <= 0 && r.height <= 0) {
      if (old) old.remove();
      return;
    }
    var box = old || document.createElement('div');
    if (!old) {
      box.id = HIGHLIGHT_ID;
      var host = document.body || document.documentElement;
      if (host) host.appendChild(box);
    }
    box.style.cssText = 'position:fixed;pointer-events:none;z-index:2147483646;'
      + 'border:2px solid #7aa2f7;background:rgba(122,162,247,0.22);box-sizing:border-box;'
      + 'left:' + r.left + 'px;top:' + r.top + 'px;'
      + 'width:' + r.width + 'px;height:' + r.height + 'px;';
  }

  function directText(el) {
    var out = '';
    for (var i = 0; i < el.childNodes.length; i++) {
      if (el.childNodes[i].nodeType === 3) out += el.childNodes[i].nodeValue || '';
    }
    out = out.replace(/\s+/g, ' ').trim();
    return out.length > 48 ? out.slice(0, 48) + '...' : out;
  }

  function buildSnapshotWithTarget(target) {
    var MAX_NODES = 250;
    var MAX_DEPTH = 14;
    var path = [];
    var cur = target;
    while (cur && cur.nodeType === 1) {
      path.push(cur);
      cur = cur.parentElement;
    }
    path.reverse();

    var nodes = [];
    var refs = [];
    var selectedIndex = -1;

    function addNode(el, depth) {
      var idx = refs.length;
      refs.push(el);
      nodes.push({
        index: idx,
        depth: depth,
        tag: el.tagName ? el.tagName.toLowerCase() : 'node',
        id: el.id || '',
        classes: typeof el.className === 'string' ? el.className.trim() : '',
        text: directText(el)
      });
      if (el === target) selectedIndex = idx;
    }

    function walk(el, depth) {
      if (!el || el.id === HIGHLIGHT_ID || el.id === '__nservo_hint_overlay__') return;
      var onPath = path.indexOf(el) !== -1;
      if (!onPath && (nodes.length >= MAX_NODES || depth > MAX_DEPTH)) return;
      addNode(el, depth);
      var children = el.children || [];
      for (var i = 0; i < children.length; i++) {
        var child = children[i];
        if (nodes.length < MAX_NODES || path.indexOf(child) !== -1) {
          walk(child, depth + 1);
        }
      }
    }

    walk(document.documentElement, 0);
    if (selectedIndex < 0 && target && target.nodeType === 1) {
      addNode(target, Math.max(0, path.length - 1));
    }
    window.__nservoDomNodes = refs;
    return { nodes: nodes, selectedIndex: Math.max(0, selectedIndex) };
  }

  function resolveTarget(ev) {
    var el = ev.target;
    if (el && el.nodeType === 3) el = el.parentElement;
    if (el && el.id === HIGHLIGHT_ID && document.elementFromPoint) {
      el.style.display = 'none';
      el = document.elementFromPoint(ev.clientX, ev.clientY);
      if (el && el.nodeType === 3) el = el.parentElement;
    }
    return (el && el.nodeType === 1) ? el : document.documentElement;
  }

  function cleanup() {
    document.removeEventListener('mousemove', onMove, true);
    document.removeEventListener('click', onClick, true);
    document.removeEventListener('mousedown', onDown, true);
    document.removeEventListener('keydown', onKey, true);
    document.documentElement.style.cursor = prevCursor;
    window.__nservoPickerStop = null;
  }

  function onMove(ev) {
    var el = resolveTarget(ev);
    if (el === lastHover) return;
    lastHover = el;
    drawHighlight(el);
  }

  function onDown(ev) {
    ev.preventDefault();
    ev.stopPropagation();
    ev.stopImmediatePropagation();
  }

  function onClick(ev) {
    ev.preventDefault();
    ev.stopPropagation();
    ev.stopImmediatePropagation();
    var target = resolveTarget(ev);
    cleanup();
    drawHighlight(target);
    var snap = buildSnapshotWithTarget(target);
    h.postMessage({
      kind: 'element_picked',
      nodes: snap.nodes,
      selected_index: snap.selectedIndex
    });
  }

  function onKey(ev) {
    if (ev.key === 'Escape') {
      ev.preventDefault();
      ev.stopPropagation();
      cleanup();
      var old = document.getElementById(HIGHLIGHT_ID);
      if (old) old.remove();
      h.postMessage({ kind: 'picker_cancelled' });
    }
  }

  window.__nservoPickerStop = cleanup;
  document.addEventListener('mousemove', onMove, true);
  document.addEventListener('mousedown', onDown, true);
  document.addEventListener('click', onClick, true);
  document.addEventListener('keydown', onKey, true);
})();"#
}

/// Cancel an active element-picker session inside the page.
pub fn stop_element_picker_script() -> &'static str {
    r#"(function() {
  if (typeof window.__nservoPickerStop === 'function') {
    window.__nservoPickerStop();
  }
})();"#
}
