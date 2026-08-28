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
