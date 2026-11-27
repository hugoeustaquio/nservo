/// Script message handler name used to report in-page search state to GTK4.
pub const SEARCH_MESSAGE_HANDLER: &str = "nservoSearch";

/// UserScript injected into every page load to provide Vim-style in-page search
/// (`window.__nservoSearch`) with match highlighting and `n` / `N` navigation.
pub fn search_init_script() -> &'static str {
    r#"(function() {
  if (window.__nservoSearch) return;

  var MARK_STYLE = 'background:#e0af68 !important;color:#1a1b26 !important;'
    + 'border-radius:2px;padding:0;outline:none !important;';
  var ACTIVE_STYLE = 'background:#ff9e64 !important;color:#1a1b26 !important;'
    + 'border-radius:2px;padding:0;outline:2px solid #f7768e !important;';
  var SKIP_TAGS = {
    SCRIPT: 1, STYLE: 1, NOSCRIPT: 1, TEXTAREA: 1, INPUT: 1,
    SELECT: 1, SVG: 1, CANVAS: 1, IFRAME: 1, TEMPLATE: 1
  };

  var state = { query: '', marks: [], index: -1 };

  function post(payload) {
    var h = window.servoGtk && window.servoGtk.messageHandlers
      && window.servoGtk.messageHandlers.nservoSearch;
    if (h) h.postMessage(payload);
  }

  function escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function clearMarks() {
    var existing = document.querySelectorAll('mark[data-nservo-search]');
    var parents = [];
    for (var i = 0; i < existing.length; i++) {
      var mark = existing[i];
      var parent = mark.parentNode;
      if (!parent) continue;
      parent.replaceChild(document.createTextNode(mark.textContent || ''), mark);
      if (parents.indexOf(parent) === -1) parents.push(parent);
    }
    for (var j = 0; j < parents.length; j++) {
      try { parents[j].normalize(); } catch (_) {}
    }
    state.marks = [];
    state.index = -1;
  }

  function isSkipped(el) {
    if (!el || el.nodeType !== 1) return true;
    if (SKIP_TAGS[el.tagName]) return true;
    if (el.closest && el.closest('#nservo-hints-root, #nservo-inspector-highlight')) {
      return true;
    }
    try {
      var cs = window.getComputedStyle(el);
      if (cs && (cs.display === 'none' || cs.visibility === 'hidden')) return true;
    } catch (_) {}
    return false;
  }

  function scrollToMark(mark) {
    if (!mark) return;
    var rect = mark.getBoundingClientRect();
    if (window.__nservoScroll) {
      var curX = window.scrollX || window.pageXOffset || 0;
      var curY = window.scrollY || window.pageYOffset || 0;
      var vw = window.innerWidth || 800;
      var vh = window.innerHeight || 600;
      var tx = (rect.left >= 0 && rect.right <= vw) ? curX : Math.max(0, curX + rect.left - vw * 0.25);
      var ty = Math.max(0, curY + rect.top - vh * 0.42);
      window.__nservoScroll.to(tx, ty);
    } else if (mark.scrollIntoView) {
      mark.scrollIntoView({ block: 'center', inline: 'nearest' });
    }
  }

  function setActive(idx) {
    if (state.index >= 0 && state.index < state.marks.length) {
      var prev = state.marks[state.index];
      prev.removeAttribute('data-nservo-search-active');
      prev.setAttribute('style', MARK_STYLE);
    }
    state.index = idx;
    var cur = state.marks[idx];
    if (cur) {
      cur.setAttribute('data-nservo-search-active', '1');
      cur.setAttribute('style', ACTIVE_STYLE);
      scrollToMark(cur);
    }
  }

  function runSearch(query, pickLast) {
    clearMarks();
    state.query = query || '';
    if (!state.query || !document.body) {
      post({ type: 'results', query: state.query, current: 0, total: 0, wrapped: false });
      return;
    }

    var sensitive = state.query !== state.query.toLowerCase();
    var re = new RegExp(escapeRegex(state.query), sensitive ? 'g' : 'gi');
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null);
    var hits = [];
    var node;
    while ((node = walker.nextNode())) {
      var text = node.nodeValue;
      if (!text || !text.trim() || isSkipped(node.parentElement)) continue;
      re.lastIndex = 0;
      var ranges = [];
      var m;
      while ((m = re.exec(text)) !== null) {
        if (m[0].length === 0) break;
        ranges.push([m.index, m.index + m[0].length]);
      }
      if (ranges.length > 0) hits.push({ node: node, ranges: ranges });
    }

    for (var i = 0; i < hits.length; i++) {
      var item = hits[i];
      var src = item.node.nodeValue || '';
      var parent = item.node.parentNode;
      if (!parent) continue;
      var frag = document.createDocumentFragment();
      var pos = 0;
      for (var r = 0; r < item.ranges.length; r++) {
        var start = item.ranges[r][0];
        var end = item.ranges[r][1];
        if (start > pos) frag.appendChild(document.createTextNode(src.slice(pos, start)));
        var mark = document.createElement('mark');
        mark.setAttribute('data-nservo-search', '1');
        mark.setAttribute('style', MARK_STYLE);
        mark.textContent = src.slice(start, end);
        frag.appendChild(mark);
        state.marks.push(mark);
        pos = end;
      }
      if (pos < src.length) frag.appendChild(document.createTextNode(src.slice(pos)));
      parent.replaceChild(frag, item.node);
    }

    var total = state.marks.length;
    if (total === 0) {
      post({ type: 'results', query: state.query, current: 0, total: 0, wrapped: false });
      return;
    }

    var initial = 0;
    if (pickLast) {
      initial = total - 1;
    } else {
      for (var k = 0; k < total; k++) {
        if (state.marks[k].getBoundingClientRect().top >= -4) {
          initial = k;
          break;
        }
      }
    }
    setActive(initial);
    post({ type: 'results', query: state.query, current: initial + 1, total: total, wrapped: false });
  }

  function step(delta, fallbackQuery) {
    var query = fallbackQuery || state.query;
    if (!query) return;
    var stale = state.marks.length === 0
      || state.query !== query
      || !state.marks[0].isConnected;
    if (stale) {
      runSearch(query, delta < 0);
      return;
    }
    var total = state.marks.length;
    var prev = state.index < 0 ? 0 : state.index;
    var next = ((prev + delta) % total + total) % total;
    var wrapped = total > 1 && ((delta > 0 && next <= prev) || (delta < 0 && next >= prev));
    setActive(next);
    post({ type: 'results', query: state.query, current: next + 1, total: total, wrapped: wrapped });
  }

  function clear() {
    clearMarks();
    state.query = '';
    post({ type: 'cleared' });
  }

  window.__nservoSearch = { search: runSearch, step: step, clear: clear };
})();"#
}

/// Script to lazily bootstrap `__nservoSearch` if needed, highlight all occurrences
/// of `query`, and jump to the first match.
pub fn search_query_script(query: &str) -> String {
    let encoded = serde_json::to_string(query).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "if (!window.__nservoSearch) {{ {} }}\n\
         if (window.__nservoSearch) {{ window.__nservoSearch.search({encoded}, false); }}",
        search_init_script()
    )
}

/// Script to lazily bootstrap `__nservoSearch` if needed and jump to the next
/// (`delta > 0`) or previous (`delta < 0`) search match.
pub fn search_step_script(delta: i32, fallback_query: &str) -> String {
    let encoded = serde_json::to_string(fallback_query).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "if (!window.__nservoSearch) {{ {} }}\n\
         if (window.__nservoSearch) {{ window.__nservoSearch.step({delta}, {encoded}); }}",
        search_init_script()
    )
}

/// Script to remove all in-page search highlights.
pub fn clear_search_script() -> &'static str {
    "if (window.__nservoSearch) { window.__nservoSearch.clear(); }"
}
