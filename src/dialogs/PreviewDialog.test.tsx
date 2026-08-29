// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/tauri', () => ({
  commands: {
    previewNormalize: vi.fn(),
  },
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: vi.fn() },
}));

import { PreviewDialog } from './PreviewDialog';
import { commands } from '../lib/tauri';
import { openUrl } from '@tauri-apps/plugin-opener';
import { notifications } from '@mantine/notifications';
import { useLocaleStore } from '../stores/locale';

const previewNormalize = vi.mocked(commands.previewNormalize);
const openUrlMock = vi.mocked(openUrl);
const showMock = vi.mocked(notifications.show);

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// jsdom has no matchMedia / ResizeObserver; Mantine needs both.
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});
class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
(globalThis as { ResizeObserver?: unknown }).ResizeObserver ??= ResizeObserverStub;

const TEXT = 'Вызови getUserData() через API';

interface HarnessProps {
  onSynthesize?: () => void;
  onCancel?: () => void;
}

function Harness({ onSynthesize = vi.fn(), onCancel = vi.fn() }: HarnessProps) {
  return (
    <MantineProvider>
      <PreviewDialog
        text={TEXT}
        opened
        onSynthesize={onSynthesize}
        onCancel={onCancel}
      />
    </MantineProvider>
  );
}

describe('PreviewDialog normalization explainer', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    previewNormalize.mockReset().mockResolvedValue({ normalized: 'нормализованный текст' });
    openUrlMock.mockReset().mockResolvedValue(undefined);
    showMock.mockReset();
    useLocaleStore.setState({ locale: 'ru' });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function render() {
    act(() => {
      root.render(<Harness />);
    });
  }

  function helpButton(): HTMLButtonElement | null {
    const el = document.body.querySelector(
      'button[aria-label="Что такое нормализация"]',
    );
    return el instanceof HTMLButtonElement ? el : null;
  }

  it('shows the explainer line on open', () => {
    render();
    expect(document.body.textContent).toContain(
      'Нормализация готовит технический текст к озвучке',
    );
  });

  it('help control toggles the details popover and opens the README link', () => {
    render();
    expect(document.body.textContent).not.toContain('Переписывается всё');

    const button = helpButton();
    expect(button, 'help button with aria-label rendered').toBeTruthy();
    act(() => {
      button!.click();
    });
    expect(document.body.textContent).toContain('Переписывается всё');

    const link = Array.from(document.body.querySelectorAll('a')).find((a) =>
      a.textContent?.includes('Подробнее'),
    );
    expect(link, 'README link rendered').toBeTruthy();
    act(() => {
      link!.click();
    });
    expect(openUrlMock).toHaveBeenCalledWith(
      'https://github.com/xilec/RuVox#нормализация',
    );
    expect(showMock).not.toHaveBeenCalled();
  });

  it('README link follows the active UI language', () => {
    useLocaleStore.setState({ locale: 'en' });
    render();
    const button = document.body.querySelector(
      'button[aria-label="About normalization"]',
    );
    expect(button, 'help button with localized aria-label').toBeTruthy();
    act(() => {
      (button as HTMLButtonElement).click();
    });
    const link = Array.from(document.body.querySelectorAll('a')).find((a) =>
      a.textContent?.includes('README'),
    );
    expect(link, 'README link rendered').toBeTruthy();
    act(() => {
      link!.click();
    });
    expect(openUrlMock).toHaveBeenCalledWith(
      'https://github.com/xilec/RuVox#normalization',
    );
  });

  it('a failed README open surfaces a red error notification', async () => {
    openUrlMock.mockRejectedValueOnce(new Error('boom'));
    render();
    const button = helpButton();
    expect(button).toBeTruthy();
    act(() => {
      button!.click();
    });
    const link = Array.from(document.body.querySelectorAll('a')).find((a) =>
      a.textContent?.includes('Подробнее'),
    );
    expect(link, 'README link rendered').toBeTruthy();
    await act(async () => {
      link!.click();
      await Promise.resolve();
    });
    expect(showMock).toHaveBeenCalledWith(
      expect.objectContaining({ color: 'red', title: 'Ошибка', message: 'boom' }),
    );
  });

  it('first ESC closes the help popover, second ESC cancels the dialog', () => {
    const onCancel = vi.fn();
    act(() => {
      root.render(
        <MantineProvider>
          <PreviewDialog text={TEXT} opened onSynthesize={vi.fn()} onCancel={onCancel} />
        </MantineProvider>,
      );
    });
    const button = helpButton();
    expect(button).toBeTruthy();
    act(() => {
      button!.click();
    });
    expect(document.body.textContent).toContain('Переписывается всё');
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(onCancel).not.toHaveBeenCalled();
    expect(document.body.textContent).not.toContain('Переписывается всё');
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});

describe('PreviewDialog regeneration mode', () => {
  const MARKUP = '<p>Первый</p><p>Второй</p>';
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    previewNormalize.mockReset().mockResolvedValue({ normalized: 'нормализованный текст' });
    openUrlMock.mockReset().mockResolvedValue(undefined);
    showMock.mockReset();
    useLocaleStore.setState({ locale: 'ru' });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  interface RegenOptions {
    text?: string;
    onSynthesize?: (finalText: string, skip: boolean, play: boolean, format: string) => void;
    onCancel?: () => void;
  }

  function renderRegen({
    text = TEXT,
    onSynthesize = vi.fn(),
    onCancel = vi.fn(),
  }: RegenOptions = {}) {
    act(() => {
      root.render(
        <MantineProvider>
          <PreviewDialog
            text={text}
            opened
            mode="regenerate"
            onSynthesize={onSynthesize}
            onCancel={onCancel}
          />
        </MantineProvider>,
      );
    });
    return { onSynthesize, onCancel };
  }

  function confirmButton(): HTMLButtonElement {
    const el = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent === 'Перегенерировать',
    );
    expect(el, 'confirm button labeled «Перегенерировать»').toBeTruthy();
    return el!;
  }

  /** The preview request is debounced by 1 s of real window.setTimeout —
   *  fake timers don't reach it in jsdom, so the suite polls with waitFor
   *  (repo test convention) and waits out the debounce. */
  const DEBOUNCE_WAIT = { timeout: 3000, interval: 50 };

  it('hides edit, the format selector, and the opt-out; keeps the shared controls', () => {
    renderRegen();

    expect(document.body.textContent).not.toContain('Редактировать');
    expect(document.body.textContent).not.toContain('Больше не показывать этот диалог');
    expect(document.body.textContent).not.toContain('Синтезировать');
    expect(
      document.body.querySelector('input[aria-label="Формат источника"]'),
    ).toBeNull();
    expect(document.body.textContent).toContain('Читать сейчас');
    expect(document.body.textContent).toContain('Синхронный скроллинг');
    expect(confirmButton()).toBeTruthy();
  });

  it('normalizes the stored text as-is, without re-extracting markup', async () => {
    renderRegen({ text: MARKUP });

    await vi.waitFor(
      () => expect(previewNormalize).toHaveBeenCalledTimes(1),
      DEBOUNCE_WAIT,
    );
    // In the Add flow this markup would be detected as html and extracted
    // before the preview; regeneration must preview the raw stored text.
    expect(previewNormalize).toHaveBeenCalledWith(MARKUP);
  });

  it('confirming sends the unchanged text with the Read Now switch state', async () => {
    const onSynthesize = vi.fn();
    renderRegen({ onSynthesize });

    // The confirm button is disabled while the debounced normalization is
    // in flight; wait it out, then flip «Read Now» off (it defaults to ON).
    await vi.waitFor(
      () => expect(confirmButton().disabled).toBe(false),
      DEBOUNCE_WAIT,
    );
    const readNow = document.body.querySelector('input[role="switch"]');
    expect(readNow, 'Read Now switch rendered').toBeTruthy();
    act(() => {
      (readNow as HTMLInputElement).click();
    });

    act(() => {
      confirmButton().click();
    });
    expect(onSynthesize).toHaveBeenCalledTimes(1);
    expect(onSynthesize).toHaveBeenCalledWith(TEXT, false, false, 'plain');
  });

  it('ESC cancels without confirming', () => {
    const onSynthesize = vi.fn();
    const onCancel = vi.fn();
    renderRegen({ onSynthesize, onCancel });

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onSynthesize).not.toHaveBeenCalled();
  });
});
