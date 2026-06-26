import type { Decorator, Preview } from '@storybook/react-vite';

import '../src/overlay/styles.scss';

// Storybook applies its own "Nunito Sans" font to the preview body, which the
// overlay inherits via `font: inherit`. Re-assert the overlay window font on the
// story root so Storybook matches the real `recording_overlay` webview.
export const decorators: Decorator[] = [
  (Story) => (
    <div style={{ fontFamily: "'Inter Variable', system-ui, sans-serif" }}>
      <Story />
    </div>
  ),
];

const preview: Preview = {
  parameters: {
    backgrounds: {
      default: 'dark',
      values: [
        { name: 'dark', value: '#18181b' },
        { name: 'light', value: '#f4f4f5' },
      ],
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
};

export default preview;
