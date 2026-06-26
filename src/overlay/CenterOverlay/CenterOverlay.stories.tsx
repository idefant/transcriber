import type { Meta, StoryObj } from '@storybook/react-vite';

import CenterOverlay from './CenterOverlay';

const meta: Meta<typeof CenterOverlay> = {
  title: 'Overlay/Center',
  component: CenterOverlay,
  args: {
    isVisible: true,
    levels: [0.6, 0.9, 0.4],
    onCancel: () => {},
  },
};

export default meta;

type Story = StoryObj<typeof CenterOverlay>;

export const Recording: Story = {
  args: { state: 'recording' },
};

export const Transcribing: Story = {
  args: { state: 'transcribing' },
};

export const Processing: Story = {
  args: { state: 'processing' },
};
