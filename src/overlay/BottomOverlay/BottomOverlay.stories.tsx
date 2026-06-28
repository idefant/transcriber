import type { Meta, StoryObj } from '@storybook/react-vite';

import BottomOverlay from './BottomOverlay';

const meta: Meta<typeof BottomOverlay> = {
  title: 'Overlay/Bottom',
  component: BottomOverlay,
  args: {
    isVisible: true,
    levels: [0.34, 0.76, 0.52],
    onCancel: () => {},
    onClose: () => {},
    onOpenRecord: () => {},
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

export const Error: Story = {
  args: { state: 'error', recordId: 'demo-record' },
};

export const Warning: Story = {
  args: { state: 'warning', recordId: 'demo-record' },
};

export const ErrorWithoutRecord: Story = {
  args: { state: 'error', recordId: null },
};
