import type { Meta, StoryObj } from '@storybook/react-vite';

import BottomOverlay from './BottomOverlay';

const meta: Meta<typeof BottomOverlay> = {
  title: 'Overlay/Bottom',
  component: BottomOverlay,
  args: {
    isVisible: true,
    levels: [0.6, 0.9, 0.4],
    onCancel: () => {},
  },
};

export default meta;

type Story = StoryObj<typeof BottomOverlay>;

export const Recording: Story = {
  args: { state: 'recording' },
};

export const Transcribing: Story = {
  args: { state: 'transcribing' },
};

export const Processing: Story = {
  args: { state: 'processing' },
};
