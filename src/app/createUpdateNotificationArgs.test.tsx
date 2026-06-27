import { isValidElement } from 'react';
import { describe, expect, it, vi } from 'vitest';

import {
  createUpdateNotificationArgs,
  updateNotificationKey,
} from './createUpdateNotificationArgs';

describe('createUpdateNotificationArgs', () => {
  it('enables the built-in countdown progress bar with hover pause', () => {
    const onDownload = vi.fn();
    const t = ((key: string, options?: { version?: string }) => {
      if (key === 'settings.about.download') {
        return 'Download';
      }

      if (key === 'settings.about.updateAvailable') {
        return `Update ${options?.version} is available`;
      }

      return key;
    }) as never;

    const args = createUpdateNotificationArgs({
      info: { version: '1.2.3' },
      onDownload,
      t,
    });

    expect(args.duration).toBe(10);
    expect(args.key).toBe(updateNotificationKey);
    expect(args.pauseOnHover).toBe(true);
    expect(args.placement).toBe('bottomRight');
    expect(args.showProgress).toBe(true);
    expect(args.title).toBe('Update 1.2.3 is available');
    expect(isValidElement(args.actions)).toBe(true);
  });
});
