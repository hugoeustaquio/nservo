use servo_gtk::WebView;

use crate::core::BrowserAction;

/// Default vertical/horizontal step in CSS pixels for `j`, `k`, `h`, and `l`.
pub const SCROLL_STEP_PX: i32 = 80;

/// Script message handler name used to report page scroll geometry to GTK4.
pub const SCROLL_MESSAGE_HANDLER: &str = "nservoScroll";

/// User stylesheet hiding default page scrollbars so the GTK4 overlay
/// scrollbar acts as the single clean indicator, and enabling smooth anchors.
pub fn scroll_hide_css() -> &'static str {
    "html { scrollbar-width: none !important; scroll-behavior: smooth; }\n\
     html::-webkit-scrollbar { display: none !important; }"
}

/// UserScript injected into every page load to provide smooth easing scroll
/// (`window.__nservoScroll`) and throttled scroll-metric reporting to GTK4.
pub fn scroll_init_script() -> &'static str {
    r#"(function() {
  if (window.__nservoScroll) return;

  var animId = null;
  var targetX = null;
  var targetY = null;

  function docMetrics() {
    var de = document.documentElement;
    var bd = document.body;
    var viewW = window.innerWidth || (de ? de.clientWidth : 0) || 0;
    var viewH = window.innerHeight || (de ? de.clientHeight : 0) || 0;
    var scrollW = Math.max(de ? de.scrollWidth : 0, bd ? bd.scrollWidth : 0, viewW);
    var scrollH = Math.max(de ? de.scrollHeight : 0, bd ? bd.scrollHeight : 0, viewH);
    var x = window.scrollX || window.pageXOffset || 0;
    var y = window.scrollY || window.pageYOffset || 0;
    return { x: x, y: y, viewW: viewW, viewH: viewH, scrollW: scrollW, scrollH: scrollH };
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
  }

  function stepDelta(delta) {
    if (Math.abs(delta) < 1.5) return delta;
    var step = Math.round(Math.abs(delta) * 0.28);
    return (delta > 0 ? 1 : -1) * Math.max(1, step);
  }

  function stepAnim() {
    if (targetX === null || targetY === null) {
      animId = null;
      return;
    }
    var m = docMetrics();
    var dx = targetX - m.x;
    var dy = targetY - m.y;
    if (Math.abs(dx) < 1.5 && Math.abs(dy) < 1.5) {
      window.scrollTo(targetX, targetY);
      stopAnim();
      scheduleReport();
      return;
    }
    var nx = Math.round(m.x + stepDelta(dx));
    var ny = Math.round(m.y + stepDelta(dy));
    window.scrollTo(nx, ny);
    scheduleReport();
    animId = requestAnimationFrame(stepAnim);
  }

  function animateTo(x, y) {
    var m = docMetrics();
    targetX = clamp(x, 0, Math.max(0, m.scrollW - m.viewW));
    targetY = clamp(y, 0, Math.max(0, m.scrollH - m.viewH));
    if (animId === null) {
      animId = requestAnimationFrame(stepAnim);
    }
  }

  function animateBy(dx, dy) {
    var m = docMetrics();
    var baseX = targetX !== null ? targetX : m.x;
    var baseY = targetY !== null ? targetY : m.y;
    animateTo(baseX + dx, baseY + dy);
  }

  function jumpTo(x, y) {
    stopAnim();
    window.scrollTo(x, y);
    scheduleReport();
  }

  var reportScheduled = false;
  var lastKey = '';
  function emitReport() {
    reportScheduled = false;
    var h = window.servoGtk && window.servoGtk.messageHandlers
      && window.servoGtk.messageHandlers.nservoScroll;
    if (!h) return;
    var m = docMetrics();
    var key = Math.round(m.x) + ',' + Math.round(m.y) + ','
      + Math.round(m.viewW) + ',' + Math.round(m.viewH) + ','
      + Math.round(m.scrollW) + ',' + Math.round(m.scrollH);
    if (key === lastKey) return;
    lastKey = key;
    h.postMessage(m);
  }

  function scheduleReport() {
    if (reportScheduled) return;
    reportScheduled = true;
    requestAnimationFrame(emitReport);
  }

  window.addEventListener('scroll', scheduleReport, { passive: true });
  window.addEventListener('resize', scheduleReport, { passive: true });
  window.addEventListener('load', scheduleReport, { passive: true });
  window.addEventListener('touchstart', stopAnim, { passive: true });

  var mutTimer = null;
  if (typeof MutationObserver !== 'undefined' && document.documentElement) {
    var mo = new MutationObserver(function() {
      if (mutTimer !== null) return;
      mutTimer = setTimeout(function() {
        mutTimer = null;
        scheduleReport();
      }, 150);
    });
    mo.observe(document.documentElement, { childList: true, subtree: true });
  }

  setTimeout(scheduleReport, 150);
  setTimeout(scheduleReport, 600);

  window.__nservoScroll = {
    by: animateBy,
    to: animateTo,
    toBottom: function() {
      var m = docMetrics();
      animateTo(m.x, Math.max(0, m.scrollH - m.viewH));
    },
    jumpY: function(y) {
      var m = docMetrics();
      jumpTo(m.x, y);
    },
    jumpX: function(x) {
      var m = docMetrics();
      jumpTo(x, m.y);
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

/// Apply a scroll-related [`BrowserAction`] to `web_view`, returning `true` if handled.
pub fn try_apply(web_view: &WebView, action: &BrowserAction) -> bool {
    match action {
        BrowserAction::ScrollDown => scroll_down(web_view),
        BrowserAction::ScrollUp => scroll_up(web_view),
        BrowserAction::ScrollLeft => scroll_left(web_view),
        BrowserAction::ScrollRight => scroll_right(web_view),
        BrowserAction::ScrollTop => scroll_top(web_view),
        BrowserAction::ScrollBottom => scroll_bottom(web_view),
        _ => return false,
    }
    true
}

pub fn scroll_down(web_view: &WebView) {
    web_view.evaluate_javascript(&smooth_scroll_by_script(0.0, f64::from(SCROLL_STEP_PX)));
}

pub fn scroll_up(web_view: &WebView) {
    web_view.evaluate_javascript(&smooth_scroll_by_script(0.0, f64::from(-SCROLL_STEP_PX)));
}

pub fn scroll_left(web_view: &WebView) {
    web_view.evaluate_javascript(&smooth_scroll_by_script(f64::from(-SCROLL_STEP_PX), 0.0));
}

pub fn scroll_right(web_view: &WebView) {
    web_view.evaluate_javascript(&smooth_scroll_by_script(f64::from(SCROLL_STEP_PX), 0.0));
}

pub fn scroll_top(web_view: &WebView) {
    web_view.evaluate_javascript(
        "if (window.__nservoScroll) { window.__nservoScroll.to(0, 0); } \
         else { window.scrollTo(0, 0); }",
    );
}

pub fn scroll_bottom(web_view: &WebView) {
    web_view.evaluate_javascript(
        "if (window.__nservoScroll) { window.__nservoScroll.toBottom(); } else { \
         window.scrollTo(0, Math.max(\
             document.body ? document.body.scrollHeight : 0, \
             document.documentElement ? document.documentElement.scrollHeight : 0\
         )); }",
    );
}
