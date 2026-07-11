import type { Decorator, Preview } from '@storybook/react-vite';

import '../src/overlay/styles.scss';

// Storybook применяет свой собственный шрифт "Nunito Sans" к телу предпросмотра, который
// оверлей наследует через `font: inherit`. Переустанавливаем шрифт окна оверлея на
// корне story, чтобы Storybook соответствовал реальному вебвью `recording_overlay`.
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
