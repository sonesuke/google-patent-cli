# Chromium Headless Freeze Investigation

## Problem

Google Patents search page (`https://patents.google.com/?q=...`) freezes indefinitely when loaded in headless Chromium 138 on Linux/ARM container. The page never finishes loading.

## Root Cause

Google Patents uses Polymer framework with `webcomponents-lite.min.js` (deprecated, unmaintained since 2018).

### Freeze Mechanism

1. `webcomponents-lite.min.js` uses HTML Imports polyfill
2. HTML Imports polyfill fetches `search-app-vulcanized.html` via **synchronous XHR** (`XMLHttpRequest.open(method, url, false)`)
3. The polyfill calls `send()` and immediately busy-waits on `responseText`
4. In Chromium 138 headless on Linux/ARM, this synchronous XHR hangs indefinitely
5. The JS thread is blocked waiting for the XHR response that never arrives

### Why XHR async patch doesn't work

We tried forcing `XMLHttpRequest.open()` to always use `async=true`:

```javascript
var origOpen = XMLHttpRequest.prototype.open;
XMLHttpRequest.prototype.open = function(method, url, async, user, pass) {
    return origOpen.call(this, method, url, true, user, pass);
};
```

This does NOT fix the freeze because:
- The polyfill calls `xhr.send()` then immediately reads `xhr.responseText` (blocking expectation)
- With async=true, the response arrives via callback (which needs the JS thread)
- But the JS thread is blocked by the polyfill's busy-wait loop
- **Deadlock**: polyfill waits for response → response callback needs JS thread → JS thread is blocked by polyfill

## Platform Differences

| Platform | Chromium Version | Headless | Result |
|---|---|---|---|
| Mac (user's machine) | 149 | Yes/No | Works |
| Linux/ARM container | 138 | Yes | Freezes |

**Hypothesis**: Chromium fixed or improved sync XHR handling in versions between 138-149. Chromium 138's headless mode on Linux may handle sync XHR differently than newer versions or than the Mac build.

## Current Workaround: `navigate_safe`

Implemented in `chrome-cdp` library (`chrome-cdp/src/page.rs`):

```
1. Disable JavaScript via CDP (Emulation.setScriptExecutionDisabled)
2. Navigate to URL (JS won't execute, so polyfill never runs)
3. Wait for HTML to load
4. Use DOM CDP commands to remove polyfill elements (no JS needed)
5. Re-enable JavaScript
6. Fetch search results via /xhr/query API endpoint
```

This completely bypasses the polyfill freeze by never allowing the problematic JavaScript to execute during page load.

### API Approach

After `navigate_safe`, search results are fetched via Google Patents internal API:
- Endpoint: `/xhr/query?url=<search_url>`
- Returns JSON with `results.cluster[].result[].patent` structure
- No page rendering needed - pure JSON API

## Chromium Version in nixpkgs

- `nixos-24.11` branch: Chromium **138.0.7204.49** (same as current container)
- `nixos-unstable` branch: Chromium **148-149** (expected to fix the freeze)

### Action Taken (2026-04-12)

Changed `flake.nix` from `nixos-24.11` to `nixos-unstable`. Requires rebuild on host:

```bash
nix flake lock --update-input nixpkgs
mise run build
mise run up
```

## Verification Plan

After upgrading to Chromium 148+:
1. Test if normal `page.goto()` works without `navigate_safe` on search pages
2. If yes, consider making `navigate_safe` a fallback for older Chromium versions
3. If no, keep `navigate_safe` as the primary approach

## Files Modified

- `chrome-cdp/src/page.rs` — Added `navigate_safe()`, `capture_screenshot()`, `send_command()`
- `src/core/patent_search.rs` — Uses `navigate_safe` + `/xhr/query` API
- `src/core/scripts/stealth.js` — Created but **unused** (XHR patch is embedded in `CdpPage::new()`)
- `flake.nix` — Changed to `nixos-unstable`

## Open Items

- [ ] Rebuild Docker image with nixos-unstable and verify Chromium version
- [ ] Test if newer Chromium resolves the freeze without `navigate_safe`
- [ ] Commit and push changes to both `chrome-cdp` and `google-patent-cli` repos
- [ ] Clean up `stealth.js` if it remains unused
