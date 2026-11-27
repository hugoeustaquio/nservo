use servo_gtk::WebView;

use crate::core::BrowserAction;

/// Default vertical/horizontal step in CSS pixels for `j`, `k`, `h`, and `l`.
pub const SCROLL_STEP_PX: i32 = 120;

/// Script message handler name used to report page scroll geometry to GTK4.
pub const SCROLL_MESSAGE_HANDLER: &str = "nservoScroll";

/// User stylesheet hiding default page scrollbars so the GTK4 overlay
/// scrollbar acts as the single clean indicator, and enabling smooth anchors.
pub fn scroll_hide_css() -> &'static str {
    "html { scrollbar-width: none !important; scroll-behavior: smooth; }\n\
     html::-webkit-scrollbar { display: none !important; }"
}

/// UserScript injected into every page load to provide snappy time-based easing
/// scroll (`window.__nservoScroll`) and throttled scroll-metric reporting to GTK4.
pub fn scroll_init_script() -> &'static str {
    r#"(function() {
  if (window.__nservoScroll) return;

  var animId = null;
  var targetX = null;
  var targetY = null;
  var lastTime = 0;
  var cachedBounds = null;
  var boundsTime = 0;
  var boundsDirty = true;

  function nowMs() {
    return (typeof performance !== 'undefined' && performance.now)
      ? performance.now()
      : Date.now();
  }

  function instantScrollTo(x, y) {
    try {
      window.scrollTo({ left: x, top: y, behavior: 'instant' });
    } catch (_) {
      window.scrollTo(x, y);
    }
  }

  function scrollPos() {
    return {
      x: window.scrollX || window.pageXOffset || 0,
      y: window.scrollY || window.pageYOffset || 0
    };
  }

  // Query viewport/scroll dimensions once on the root scrolling element
  // (avoiding redundant document.body fragment-tree walks in standards mode)
  // and cache briefly so rapid scrolls and animation frames avoid extra reflows.
  function docBounds(force) {
    var now = nowMs();
    if (!force && !boundsDirty && cachedBounds && (now - boundsTime) < 400) {
      return cachedBounds;
    }
    var de = document.documentElement;
    var bd = document.body;
    var root = (document.compatMode === 'BackCompat' && bd) ? bd : (de || bd);
    var viewW = window.innerWidth || (root ? root.clientWidth : 0) || 0;
    var viewH = window.innerHeight || (root ? root.clientHeight : 0) || 0;
    var scrollW = Math.max(root ? root.scrollWidth : 0, viewW);
    var scrollH = Math.max(root ? root.scrollHeight : 0, viewH);
    cachedBounds = { viewW: viewW, viewH: viewH, scrollW: scrollW, scrollH: scrollH };
    boundsTime = now;
    boundsDirty = false;
    return cachedBounds;
  }

  function clamp(v, lo, hi) {
    return v < lo ? lo : (v > hi ? hi : v);
  }

  function stopAnim() {
    if (animId !== null) {
      cancelAnimationFrame(animId);
      animId = null;
    }
    targetX = null;
    targetY = null;
    lastTime = 0;
  }

  function stepDelta(delta, alpha) {
    var abs = Math.abs(delta);
    if (abs <= 3) return delta;
    var step = Math.round(abs * alpha);
    var clamped = Math.min(abs, Math.max(6, step));
    return (delta > 0 ? 1 : -1) * clamped;
  }

  function advanceFrame(dt) {
    if (targetX === null || targetY === null) {
      animId = null;
      return;
    }
    var pos = scrollPos();
    var dx = targetX - pos.x;
    var dy = targetY - pos.y;
    if (Math.abs(dx) <= 3 && Math.abs(dy) <= 3) {
      instantScrollTo(targetX, targetY);
      stopAnim();
      scheduleReport(true);
      return;
    }
    var clampedDt = clamp(dt, 8, 50);
    var alpha = 1 - Math.exp(-clampedDt * 0.048);
    var nx = Math.round(pos.x + stepDelta(dx, alpha));
    var ny = Math.round(pos.y + stepDelta(dy, alpha));
    instantScrollTo(nx, ny);
    scheduleReport(false);
    animId = requestAnimationFrame(stepAnim);
  }

  function stepAnim(ts) {
    var now = typeof ts === 'number' ? ts : nowMs();
    var dt = lastTime > 0 ? (now - lastTime) : 16;
    lastTime = now;
    advanceFrame(dt);
  }

  function animateTo(x, y) {
    var b = docBounds(false);
    targetX = clamp(x, 0, Math.max(0, b.scrollW - b.viewW));
    targetY = clamp(y, 0, Math.max(0, b.scrollH - b.viewH));
    if (animId === null) {
      lastTime = nowMs();
      advanceFrame(16);
    }
  }

  function animateBy(dx, dy) {
    var pos = scrollPos();
    var baseX = targetX !== null ? targetX : pos.x;
    var baseY = targetY !== null ? targetY : pos.y;
    animateTo(baseX + dx, baseY + dy);
  }

  function jumpTo(x, y) {
    stopAnim();
    instantScrollTo(x, y);
    scheduleReport(true);
  }

  var reportScheduled = false;
  var lastReportAt = 0;
  var lastKey = '';
  function emitReport() {
    reportScheduled = false;
    var h = window.servoGtk && window.servoGtk.messageHandlers
      && window.servoGtk.messageHandlers.nservoScroll;
    if (!h) return;
    var pos = scrollPos();
    var b = docBounds(false);
    var m = {
      x: pos.x, y: pos.y,
      viewW: b.viewW, viewH: b.viewH,
      scrollW: b.scrollW, scrollH: b.scrollH
    };
    var key = Math.round(m.x) + ',' + Math.round(m.y) + ','
      + Math.round(m.viewW) + ',' + Math.round(m.viewH) + ','
      + Math.round(m.scrollW) + ',' + Math.round(m.scrollH);
    if (key === lastKey) return;
    lastKey = key;
    lastReportAt = nowMs();
    h.postMessage(m);
  }

  function scheduleReport(immediate) {
    if (reportScheduled) return;
    if (!immediate && (nowMs() - lastReportAt) < 60) return;
    reportScheduled = true;
    requestAnimationFrame(emitReport);
  }

  function invalidateBounds() {
    boundsDirty = true;
    scheduleReport(false);
  }

  window.addEventListener('scroll', function() { scheduleReport(false); }, { passive: true });
  window.addEventListener('resize', invalidateBounds, { passive: true });
  window.addEventListener('load', invalidateBounds, { passive: true });
  window.addEventListener('touchstart', stopAnim, { passive: true });

  if (typeof ResizeObserver !== 'undefined' && document.documentElement) {
    var ro = new ResizeObserver(function() {
      boundsDirty = true;
      if (animId === null) scheduleReport(false);
    });
    ro.observe(document.documentElement);
  }

  setTimeout(invalidateBounds, 150);
  setTimeout(invalidateBounds, 600);

  window.__nservoScroll = {
    by: animateBy,
    to: animateTo,
    toTop: function() {
      jumpTo(0, 0);
    },
    toBottom: function() {
      var b = docBounds(true);
      jumpTo(scrollPos().x, Math.max(0, b.scrollH - b.viewH));
    },
    jumpY: function(y) {
      jumpTo(scrollPos().x, y);
    },
    jumpX: function(x) {
      jumpTo(x, scrollPos().y);
    },
    report: emitReport
  };
})();"#
}

/// Script to smoothly scroll by `(dx, dy)` CSS pixels.
pub fn smooth_scroll_by_script(dx: f64, dy: f64) -> String {
    format!(
        "if (window.__nservoScroll) {{ window.__nservoScroll.by({dx}, {dy}); }} \
         else {{ window.scrollBy({dx}, {dy}); }}"
    )
}

/// Script to jump directly to vertical offset `y` (used while dragging the scrollbar).
pub fn jump_scroll_y_script(y: f64) -> String {
    format!(
        "if (window.__nservoScroll) {{ window.__nservoScroll.jumpY({y}); }} \
         else {{ window.scrollTo(window.scrollX || 0, {y}); }}"
    )
}

/// Script to jump directly to horizontal offset `x` (used while dragging the scrollbar).
pub fn jump_scroll_x_script(x: f64) -> String {
    format!(
        "if (window.__nservoScroll) {{ window.__nservoScroll.jumpX({x}); }} \
         else {{ window.scrollTo({x}, window.scrollY || 0); }}"
    )
}

/// Compute the pixel distance for a directional scroll repeated `count` times.
pub fn scroll_step_distance(count: u32) -> f64 {
    f64::from(SCROLL_STEP_PX) * f64::from(count.max(1))
}

/// Build the JavaScript snippet for a scroll-related [`BrowserAction`], if applicable.
pub fn scroll_action_script(action: &BrowserAction) -> Option<String> {
    match *action {
        BrowserAction::ScrollDown(count) => {
            Some(smooth_scroll_by_script(0.0, scroll_step_distance(count)))
        }
        BrowserAction::ScrollUp(count) => {
            Some(smooth_scroll_by_script(0.0, -scroll_step_distance(count)))
        }
        BrowserAction::ScrollLeft(count) => {
            Some(smooth_scroll_by_script(-scroll_step_distance(count), 0.0))
        }
        BrowserAction::ScrollRight(count) => {
            Some(smooth_scroll_by_script(scroll_step_distance(count), 0.0))
        }
        BrowserAction::ScrollTop => Some(
            "if (window.__nservoScroll) { window.__nservoScroll.toTop(); } \
             else { window.scrollTo(0, 0); }"
                .to_string(),
        ),
        BrowserAction::ScrollBottom => Some(
            "if (window.__nservoScroll) { window.__nservoScroll.toBottom(); } else { \
             window.scrollTo(0, Math.max(\
                 document.body ? document.body.scrollHeight : 0, \
                 document.documentElement ? document.documentElement.scrollHeight : 0\
             )); }"
                .to_string(),
        ),
        _ => None,
    }
}

/// Apply a scroll-related [`BrowserAction`] to `web_view`, returning `true` if handled.
pub fn try_apply(web_view: &WebView, action: &BrowserAction) -> bool {
    let Some(script) = scroll_action_script(action) else {
        return false;
    };
    web_view.evaluate_javascript(&script);
    true
}
