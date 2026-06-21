# Screenshot Testing

Use Playwright for visual checks after layout, theme, modal, navigation, or Ant Design token changes.

## Viewports

Check desktop widths:

- `1200px`
- `1440px`
- `1700px`

Use `900px` height unless the tested scenario needs a taller viewport.

## Workflow

1. Start the app with `npm run dev -- --host 127.0.0.1 --port 5173`.
2. Open the app with Playwright Chromium.
3. Capture screenshots for the touched flows in both light and dark themes.
4. Check browser console warnings/errors during the run.
5. Verify that `document.body.scrollWidth` is not greater than `document.body.clientWidth`.
6. Save temporary screenshots only into `ui-audit-artifacts/`.
7. Keep temporary Playwright scripts in `.codex/`; these folders are ignored by Git and ESLint.

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
- Page content does not stretch awkwardly at `1700px`.
- No horizontal page scroll.
- Text does not overlap controls.
- Icon-only buttons have accessible labels and tooltips when applicable.
- Temporary screenshots and scripts are not committed.
