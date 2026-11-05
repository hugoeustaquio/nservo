/// Default example Greasemonkey script created during initialization.
pub const DEFAULT_USER_SCRIPT: &str = r#"// ==UserScript==
// @name         NServo Clean Theme Helper
// @description  Adds dark mode support and custom hotkey bindings to web pages.
// @match        *://*/*
// @run-at       document-end
// @grant        none
// ==/UserScript==

(function() {
    'use strict';
    console.log('[NServo] Greasemonkey helper injected successfully.');
    // Example: Set smooth scrolling preference in DOM
    document.documentElement.style.scrollBehavior = 'smooth';
})();
"#;
