// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ModelStatus } from '../lib/tauri';

const { getModelStatus } = vi.hoisted(() => ({
  getModelStatus: vi.fn<() => Promise<ModelStatus>>(),
}));

vi.mock('../lib/tauri', () => ({
  commands: {
    getModelStatus,
    installModelAsset: vi.fn().mockResolvedValue(undefined),
    deleteModelAsset: vi.fn().mockResolvedValue(0),
    skipModelUpdate: vi.fn().mockResolvedValue(undefined),
    downloadPiperVoice: vi.fn().mockResolvedValue(undefined),
  },
  events: {
    bundleDownloadStarted: () => Promise.resolve(() => {}),
    bundleDownloadProgress: () => Promise.resolve(() => {}),
    bundleDownloadFinished: () => Promise.resolve(() => {}),
    voiceDownloadStarted: () => Promise.resolve(() => {}),
    voiceDownloadProgress: () => Promise.resolve(() => {}),
    voiceDownloadFinished: () => Promise.resolve(() => {}),
  },
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: vi.fn() },
}));

import { ModelAssetsDialog } from './ModelAssets';

const status: ModelStatus = {
  total_bytes: 1234,
  channel_error: null,
  checked_at: null,
  assets: [
    {
      id: 'silero_native',
      kind: 'bundle',
      installed: true,
      active: true,
      installed_version: 'v5_ru',
      model_id: 'v5_ru',
      upstream_revision: null,
      bundled_voices: ['aidar', 'baya'],
      installed_bytes: 1000,
      latest_version: 'v5_ru-2',
      latest_bytes: 1100,
      update_available: true,
      skipped: false,
      deletable: false,
      error: null,
    },
    {
      id: 'piper:ruslan',
      kind: 'voice',
      installed: false,
      active: false,
      installed_version: null,
      model_id: 'ruslan',
      upstream_revision: null,
      installed_bytes: 0,
      latest_version: null,
      latest_bytes: null,
      update_available: false,
      skipped: false,
      deletable: false,
      error: null,
    },
  ],
};

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  getModelStatus.mockResolvedValue(status);
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
  vi.clearAllMocks();
});

describe('ModelAssetsDialog', () => {
  it('shows the unified fields and explains the active-bundle restriction', async () => {
    act(() => {
      root.render(
        <MantineProvider>
          <ModelAssetsDialog opened onClose={() => {}} onAvailabilityChanged={() => {}} />
        </MantineProvider>,
      );
    });

    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Silero (нативный)');
    });

    const text = document.body.textContent ?? '';
    expect(text).toContain('Silero (нативный)');
    expect(text).toContain('v5_ru-2');
    expect(document.querySelectorAll('[data-active-icon]').length).toBe(1);
    expect(text).toContain('Голоса Piper');
    expect(document.querySelectorAll('[data-quality-bar]').length).toBeGreaterThanOrEqual(2);

    const expander = [...document.querySelectorAll('button')].find((button) =>
      (button.textContent ?? '').includes('Голоса Piper'),
    );
    expect(expander, 'Piper expander rendered').toBeTruthy();
    act(() => expander!.click());
    expect(document.body.textContent).toContain('Скачать');
  });

  it('shows a visible result after checking for updates', async () => {
    act(() => {
      root.render(
        <MantineProvider>
          <ModelAssetsDialog opened onClose={() => {}} onAvailabilityChanged={() => {}} />
        </MantineProvider>,
      );
    });

    await vi.waitFor(() => {
      expect(document.body.textContent).toContain('Silero (нативный)');
    });

    const button = [...document.querySelectorAll('button')].find((item) =>
      (item.textContent ?? '').includes('Проверить обновления'),
    );
    expect(button, 'check button rendered').toBeTruthy();
    act(() => button!.click());

    await vi.waitFor(() => {
      expect(getModelStatus).toHaveBeenCalledWith(true);
      expect(document.body.textContent).toContain('Доступно обновлений: 1');
    });
  });
});
