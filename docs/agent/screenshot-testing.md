# Screenshot Testing

Use Playwright to capture and review the UI after layout, theme, modal, navigation, or Ant Design token changes.

The app runs inside Tauri's WebView2 (Chromium), so screenshots must be taken from the **real running app**, not from the bare Vite server in a standalone browser. A standalone browser lacks the Tauri APIs (`window.__TAURI__`, IPC) and does not reflect real behavior. Playwright attaches to the running WebView2 over the Chrome DevTools Protocol (CDP).

## Enabling the debug endpoint

Screenshots require a CDP endpoint on the running app. Ask the developer to start the app with WebView2 remote debugging — never start it yourself:

```bash
npm run dev:tauri:debug
```

This sets `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`. A plain `.env` entry does not work: WebView2 reads this variable from the real process environment, which `.env` does not provide to the Tauri app process.

Confirm the endpoint is up before attaching:

```bash
curl -s http://localhost:9222/json/list
```

## Attaching

Two equivalent ways to drive the running app:

- **Playwright MCP** — configured in the project `.mcp.json` with `--cdp-endpoint http://localhost:9222`. The MCP server is loaded at agent-session start (it requires approval on first run), then drive the app with its `browser_*` tools.
- **connectOverCDP script** — `chromium.connectOverCDP('http://localhost:9222')`, then pick the page whose URL is the app route (for example `/history`), not the `Recording Overlay`. Keep temporary scripts in `.codex/` (gitignored). Do not call `browser.close()` — over CDP it can close the user's app; let the process exit instead.

## Workflow

1. Attach to the running app over CDP (see above).
2. Select the main app page; for theme checks, toggle the theme.
3. Capture full-window and per-element screenshots for the touched flows in both light and dark themes.
4. Check browser console warnings/errors during the run.
5. Verify that `document.body.scrollWidth` is not greater than `document.body.clientWidth`.
6. Save temporary screenshots only into `ui-audit-artifacts/` (gitignored).

The app window is resizable (minimum 1080×600). To review wider layouts, resize the window before capturing.

## Required Scenarios

For app-wide visual changes, capture:

- history page;
- history page with the details panel open;
- dictionary page;
- settings modal in the general tab;
- settings modal in the providers tab;
- provider add/edit modal;
- Speech-to-Text settings;
- post-processing settings.

For theme changes, repeat at least:

- settings modal in dark mode;
- history page with the details panel open in dark mode;
- dictionary page in dark mode.

## Review Checklist

- No black text on dark backgrounds.
- Sidebar and modal menu backgrounds match the surrounding surface.
- Selected and hover states are visible but not visually aggressive.
- Page content does not stretch awkwardly when the window is wide.
- No horizontal page scroll.
- Text does not overlap controls.
- Icon-only buttons have accessible labels and tooltips when applicable.
- Temporary screenshots and scripts are not committed.
