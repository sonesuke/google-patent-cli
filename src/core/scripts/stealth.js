// Stealth evasions ported from puppeteer-extra-plugin-stealth
// Minimal set for bot detection avoidance + XHR hang workaround
(function() {
    'use strict';

    // --- navigator.webdriver ---
    try {
        if (navigator.webdriver !== false && navigator.webdriver !== undefined) {
            delete Object.getPrototypeOf(navigator).webdriver;
        }
    } catch(e) {
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
    }

    // --- chrome object ---
    if (!window.chrome) {
        Object.defineProperty(window, 'chrome', {
            writable: true, enumerable: true, configurable: false, value: {}
        });
    }
    window.chrome.runtime = {
        OnInstalledReason: {}, OnRestartRequiredReason: {},
        PlatformArch: {}, PlatformNaclArch: {}, PlatformOs: {},
        RequestUpdateCheckStatus: {},
        connect: null, sendMessage: null, get id() { return undefined; }
    };
    window.chrome.app = {
        isInstalled: false, getDetails: function() { return null; },
        getIsInstalled: function() { return false; }, runningState: function() { return 'cannot_run'; }
    };
    window.chrome.csi = function() { return { onloadT: Date.now(), startE: Date.now() }; };
    window.chrome.loadTimes = function() { return {}; };

    // --- navigator properties ---
    try { Object.defineProperty(navigator, 'languages', { get: () => Object.freeze(['en-US', 'en']) }); } catch(e) {}
    try { Object.defineProperty(navigator, 'vendor', { get: () => 'Google Inc.' }); } catch(e) {}
    try { Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => 4 }); } catch(e) {}

    // --- Suppress dialogs ---
    window.alert = function() {};
    window.confirm = function() { return true; };
    window.prompt = function() { return ''; };

    // --- Force async XHR (critical: prevents webcomponents polyfill hang) ---
    (function() {
        var origOpen = XMLHttpRequest.prototype.open;
        XMLHttpRequest.prototype.open = function(method, url, async, user, pass) {
            return origOpen.call(this, method, url, true, user, pass);
        };
    })();
})();
