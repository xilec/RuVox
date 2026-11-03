// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import { ModalsProvider } from '@mantine/modals';
import { Notifications } from '@mantine/notifications';
import type {
  AvailableEngines,
  BundleDownloadFinishedPayload,
  BundleDownloadProgressPayload,
  UIConfig,
} from '../lib/tauri';

/** Command/event stubs shared with the `../lib/tauri` mock below. The proxy
 *  materializes a resolving stub for any command the mounted tree calls, so
 *  the test only spells out the handful whose results or calls it asserts
 *  on. */
const { commands, events, mocks, eventHandlers } = vi.hoisted(() => ({
  commands: {} as Record<string, ReturnType<typeof vi.fn>>,
  events: {} as Record<string, ReturnType<typeof vi.fn>>,
  mocks: {
    updateConfig: vi.fn(),
    downloadTerattsBundle: vi.fn(),
    downloadSileroNativeBundle: vi.fn(),
    showNotification: vi.fn(),
  },
  eventHandlers: {
    finished: null as
      | null
      | ((payload: BundleDownloadFinishedPayload) => void),
    progress: null as
      | null
      | ((payload: BundleDownloadProgressPayload) => void),
  },
}));

vi.mock('@mantine/notifications', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@mantine/notifications')>();
  return { ...actual, notifications: { show: mocks.showNotification } };
});

vi.mock('../lib/tauri', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/tauri')>();
  const lazy = (
    target: Record<string, unknown>,
    fallback: () => unknown,
  ): Record<string, unknown> =>
    new Proxy(target, {
      get(obj, prop) {
        if (typeof prop !== 'string') return undefined;
        if (!(prop in obj)) obj[prop] = vi.fn().mockImplementation(fallback);
        return obj[prop];
      },
    });
  return {
    ...actual,
    commands: lazy(commands, () => Promise.resolve(undefined)),
    events: lazy(events, () => Promise.resolve(() => {})),
  };
});

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn().mockResolvedValue(() => {}),
  }),
}));

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  readText: vi.fn().mockResolvedValue(''),
}));

vi.mock('../lib/mermaid', () => ({
  renderMermaidIn: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../lib/viewerCopy', () => ({
  copyLinkAddress: vi.fn(),
  copySelection: vi.fn(),
  copyImageAddress: vi.fn(),
  copyImageBitmap: vi.fn(),
}));

import { AppShell } from './AppShell';

const CONFIG: UIConfig = {
  speaker: 'aidar',
  sample_rate: 24000,
  speech_rate: 1.0,
  notify_on_ready: true,
  notify_on_error: true,
  text_format: 'plain',
  max_cache_size_mb: 500,
  code_block_mode: 'brief',
  english_reading: 'auto',
  reading_width: 'narrow',
  reading_scale: 100,
  theme: 'auto',
  language: 'ru',
  window_geometry: null,
  preview_dialog_enabled: true,
  hide_to_tray_on_close: false,
  // The new default: a fresh config starts on TeraTTSv2 even before its
  // bundle exists.
  engine: 'teratts',
  piper_voice: 'ruslan',
  teratts_voice: 'ru_m5',
};

function availability(overrides: Partial<AvailableEngines> = {}): AvailableEngines {
  return {
    piper: { available: true, reason: null, reads_latin_natively: false },
    silero: { available: false, reason: { code: 'silero.uv_missing' }, reads_latin_natively: false },
    silero_native: {
      available: false,
      reason: { code: 'native.bundle_missing' },
      reads_latin_natively: false,
    },
    teratts: {
      available: false,
      reason: { code: 'teratts.bundle_missing' },
      reads_latin_natively: true,
    },
    ...overrides,
  };
}

/** Text of the first-run prompt, scoped to its own dialog so the rest of the
 *  shell cannot satisfy an assertion. */
function promptText(): string {
  const dialog = [...document.querySelectorAll<HTMLElement>('[role="dialog"]')].find(
    (candidate) =>
      candidate.textContent?.includes('Остаться на Piper') ||
      candidate.textContent?.includes('Движок по умолчанию'),
  );
  return dialog?.textContent ?? '';
}

function findButton(label: string): HTMLElement | null {
  return (
    [...document.querySelectorAll('button')].find(
      (candidate) => candidate.textContent?.trim() === label,
    ) ?? null
  );
}

describe('first-run bundle prompt', () => {
  let host: HTMLDivElement | null = null;
  let root: Root | null = null;

  async function renderShell(): Promise<void> {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    await act(async () => {
      root!.render(
        <MantineProvider>
          <ModalsProvider>
            <AppShell />
          </ModalsProvider>
          <Notifications />
        </MantineProvider>,
      );
      // Let the mount-time config load and availability probe settle.
      await Promise.resolve();
    });
  }

  async function unmountShell(): Promise<void> {
    if (root) {
      await act(async () => {
        root!.unmount();
        await Promise.resolve();
      });
      root = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  }

  beforeEach(() => {
    for (const key of Object.keys(commands)) delete commands[key];
    for (const key of Object.keys(events)) delete events[key];
    commands.getConfig = vi.fn().mockResolvedValue(CONFIG);
    commands.getEntries = vi.fn().mockResolvedValue([]);
    commands.getAvailableEngines = vi.fn().mockResolvedValue(availability());
    commands.updateConfig = mocks.updateConfig.mockResolvedValue(undefined);
    commands.downloadTerattsBundle = mocks.downloadTerattsBundle.mockResolvedValue(undefined);
    commands.downloadSileroNativeBundle = mocks.downloadSileroNativeBundle.mockResolvedValue(
      undefined,
    );
    events.bundleDownloadStarted = vi.fn().mockResolvedValue(() => {});
    events.bundleDownloadProgress = vi.fn(
      (handler: (payload: BundleDownloadProgressPayload) => void) => {
        eventHandlers.progress = handler;
        return Promise.resolve(() => {});
      },
    );
    events.bundleDownloadFinished = vi.fn(
      (handler: (payload: BundleDownloadFinishedPayload) => void) => {
        eventHandlers.finished = handler;
        return Promise.resolve(() => {});
      },
    );
    eventHandlers.finished = null;
    eventHandlers.progress = null;
  });

  afterEach(async () => {
    await unmountShell();
    vi.clearAllMocks();
  });

  it('a fresh config opens the prompt naming TeraTTSv2 and its bundle', async () => {
    await renderShell();

    await vi.waitFor(() => {
      expect(promptText()).toContain('Движок по умолчанию — TeraTTSv2');
    });
    expect(promptText()).toContain('~410 МБ');
    expect(promptText()).toContain('Скачать (~410 МБ)');
    expect(promptText()).toContain('Остаться на Piper');
    expect(promptText()).toContain('Быстрее и легче: Silero (нативный) (~230 МБ) — в Настройках.');
  });

  it('declining persists nothing and the prompt returns on the next launch', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Остаться на Piper')!.click();
      await Promise.resolve();
    });

    expect(mocks.updateConfig).not.toHaveBeenCalled();
    expect(mocks.downloadTerattsBundle).not.toHaveBeenCalled();

    // Next launch: the same persisted config still lacks the bundle, so the
    // prompt is raised again.
    await unmountShell();
    await renderShell();
    await vi.waitFor(() => {
      expect(promptText()).toContain('Движок по умолчанию — TeraTTSv2');
    });
  });

  it('a successful download activates TeraTTSv2 without a restart', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Скачать (~410 МБ)')!.click();
      await Promise.resolve();
    });
    expect(mocks.downloadTerattsBundle).toHaveBeenCalledTimes(1);

    await act(async () => {
      eventHandlers.finished!({ engine: 'teratts', ok: true });
      await Promise.resolve();
    });

    expect(mocks.updateConfig).toHaveBeenCalledWith({ engine: 'teratts' });
    expect(mocks.showNotification).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Движок TeraTTSv2 готов', color: 'green' }),
    );
  });

  it('ignores another engine’s download events', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      eventHandlers.finished!({ engine: 'silero_native', ok: true });
      await Promise.resolve();
    });

    expect(mocks.updateConfig).not.toHaveBeenCalled();
    expect(promptText()).toContain('Движок по умолчанию — TeraTTSv2');
  });

  it('existing Silero Native users are offered the Silero bundle, not TeraTTS', async () => {
    commands.getConfig = vi.fn().mockResolvedValue({ ...CONFIG, engine: 'silero_native' });
    await renderShell();

    await vi.waitFor(() => {
      expect(promptText()).toContain('Скачать движок Silero?');
    });
    expect(promptText()).toContain('Скачать (~230 МБ)');
    expect(promptText()).not.toContain('Движок по умолчанию — TeraTTSv2');
  });

  it('shows the current file and the overall percent while downloading', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Скачать (~410 МБ)')!.click();
      await Promise.resolve();
    });

    await act(async () => {
      eventHandlers.progress!({
        engine: 'teratts',
        file: 'ru_m5.onnx',
        file_idx: 1,
        total_files: 2,
        downloaded_bytes: 10,
        total_bytes: 20,
        skipped: false,
      });
      await Promise.resolve();
    });

    expect(promptText()).toContain('Скачивается: ru_m5.onnx');
    const bar = document.querySelector('[role="dialog"] [role="progressbar"]');
    expect(bar?.getAttribute('aria-valuenow')).toBe('75');
  });

  it('shows a failed download inline and offers a retry', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Скачать (~410 МБ)')!.click();
      await Promise.resolve();
    });
    await act(async () => {
      eventHandlers.finished!({ engine: 'teratts', ok: false, message: 'Нет места на диске' });
      await Promise.resolve();
    });

    expect(promptText()).toContain('Нет места на диске');
    const retry = findButton('Скачать (~410 МБ)');
    expect(retry).toBeTruthy();
    expect(retry!.hasAttribute('disabled')).toBe(false);

    await act(async () => {
      retry!.click();
      await Promise.resolve();
    });
    expect(mocks.downloadTerattsBundle).toHaveBeenCalledTimes(2);
  });

  it('activates the engine when the download finishes after the prompt closed', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Скачать (~410 МБ)')!.click();
      await Promise.resolve();
    });

    // Escape closes the prompt; the backend download keeps running.
    await act(async () => {
      document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await Promise.resolve();
    });
    await vi.waitFor(() => expect(promptText()).toBe(''));

    await act(async () => {
      eventHandlers.finished!({ engine: 'teratts', ok: true });
      await Promise.resolve();
    });

    expect(mocks.updateConfig).toHaveBeenCalledWith({ engine: 'teratts' });
    expect(mocks.showNotification).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Движок TeraTTSv2 готов', color: 'green' }),
    );
  });

  it('reports a failed download after the prompt closed', async () => {
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Скачать (~410 МБ)')!.click();
      await Promise.resolve();
    });
    await act(async () => {
      findButton('Остаться на Piper')!.click();
      await Promise.resolve();
    });
    await vi.waitFor(() => expect(promptText()).toBe(''));

    await act(async () => {
      eventHandlers.finished!({ engine: 'teratts', ok: false, message: 'Сервер недоступен' });
      await Promise.resolve();
    });

    expect(mocks.updateConfig).not.toHaveBeenCalled();
    expect(mocks.showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Не удалось скачать бандл',
        message: 'Сервер недоступен',
        color: 'red',
      }),
    );
  });

  it('reports a downloaded bundle that failed to activate', async () => {
    mocks.updateConfig.mockRejectedValueOnce({ type: 'config_error', code: 'config.unknown' });
    await renderShell();
    await vi.waitFor(() => expect(promptText()).not.toBe(''));

    await act(async () => {
      findButton('Скачать (~410 МБ)')!.click();
      await Promise.resolve();
    });
    await act(async () => {
      eventHandlers.finished!({ engine: 'teratts', ok: true });
      await Promise.resolve();
    });

    expect(mocks.showNotification).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Бандл скачан, но движок не запустился',
        color: 'red',
      }),
    );
    expect(mocks.showNotification).not.toHaveBeenCalledWith(
      expect.objectContaining({ color: 'green' }),
    );
  });
});
