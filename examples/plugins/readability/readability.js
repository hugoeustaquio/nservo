(function () {
  var OVERLAY_ID = 'nservo-readability-overlay';
  var ACTIVE_CLS = 'nservo-readability-active';

  function exitReader() {
    var existing = document.getElementById(OVERLAY_ID);
    if (existing) existing.remove();
    document.documentElement.classList.remove(ACTIVE_CLS);
    if (document.body) document.body.classList.remove(ACTIVE_CLS);
    var savedY = window.__nservoReadabilityPrevY;
    delete window.__nservoReadabilityPrevY;
    if (typeof savedY === 'number') window.scrollTo(0, savedY);
    if (window.__nservoScroll && window.__nservoScroll.report) {
      setTimeout(window.__nservoScroll.report, 50);
    }
  }

  if (document.getElementById(OVERLAY_ID)) {
    exitReader();
    return;
  }

  function textLen(el) {
    return (el && el.textContent ? el.textContent.replace(/\s+/g, ' ').trim().length : 0);
  }

  function linkDensity(el) {
    var total = Math.max(1, textLen(el));
    var links = el.querySelectorAll('a');
    var linkChars = 0;
    for (var i = 0; i < links.length; i++) linkChars += textLen(links[i]);
    return linkChars / total;
  }

  function scoreNode(el) {
    var tag = el.tagName ? el.tagName.toLowerCase() : '';
    var cls = ((el.className || '') + ' ' + (el.id || '')).toLowerCase();
    if (/nav|footer|header|sidebar|comment|menu|banner|promo|ad-|ads/.test(cls)) return -500;
    var score = tag === 'article' ? 40 : (tag === 'main' ? 30 : 0);
    var paras = el.querySelectorAll('p');
    for (var i = 0; i < paras.length; i++) {
      var len = textLen(paras[i]);
      if (len >= 40) score += 5 + Math.min(len / 40, 20);
    }
    return score * (1 - Math.min(linkDensity(el), 0.85));
  }

  function findBestContainer() {
    var candidates = document.querySelectorAll('article, main, [role="main"], section, div');
    var best = null;
    var bestScore = 10;
    for (var i = 0; i < candidates.length; i++) {
      var s = scoreNode(candidates[i]);
      if (s > bestScore) {
        bestScore = s;
        best = candidates[i];
      }
    }
    return best || document.body;
  }

  function sanitizeClone(root) {
    var clone = root.cloneNode(true);
    var junk = clone.querySelectorAll(
      'script, style, noscript, iframe, nav, footer, aside, form, button, svg, [aria-hidden="true"]'
    );
    for (var i = 0; i < junk.length; i++) junk[i].remove();
    var all = clone.querySelectorAll('*');
    for (var j = 0; j < all.length; j++) {
      all[j].removeAttribute('style');
      all[j].removeAttribute('class');
    }
    return clone;
  }

  var source = findBestContainer();
  if (!source || !document.body) return;

  window.__nservoReadabilityPrevY = window.scrollY || window.pageYOffset || 0;

  var h1 = document.querySelector('article h1, main h1, h1');
  var title = (h1 && h1.textContent.trim()) || document.title || location.hostname;
  var meta = location.hostname + (location.pathname && location.pathname !== '/' ? location.pathname : '');

  var overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;

  var wrap = document.createElement('article');
  wrap.className = 'nservo-reader-article';

  var topbar = document.createElement('div');
  topbar.className = 'nservo-reader-topbar';
  var hostSpan = document.createElement('span');
  hostSpan.textContent = meta;
  var closeBtn = document.createElement('button');
  closeBtn.type = 'button';
  closeBtn.textContent = 'Exit Reader';
  closeBtn.addEventListener('click', exitReader);
  topbar.appendChild(hostSpan);
  topbar.appendChild(closeBtn);

  var heading = document.createElement('h1');
  heading.textContent = title;

  var body = document.createElement('div');
  body.className = 'nservo-reader-body';
  body.appendChild(sanitizeClone(source));

  wrap.appendChild(topbar);
  wrap.appendChild(heading);
  wrap.appendChild(body);
  overlay.appendChild(wrap);

  document.documentElement.classList.add(ACTIVE_CLS);
  document.body.classList.add(ACTIVE_CLS);
  document.body.appendChild(overlay);
  window.scrollTo(0, 0);
  if (window.__nservoScroll && window.__nservoScroll.report) {
    setTimeout(window.__nservoScroll.report, 50);
  }
})();
