// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/tauri', () => ({
  commands: {
    readTextFile: vi.fn(),
  },
  IMPORT_ENCODING_NAMES: [
    'UTF-8',
    'UTF-16LE',
    'UTF-16BE',
    'windows-1251',
    'IBM866',
    'ISO-8859-5',
    'KOI8-R',
    'KOI8-U',
    'x-mac-cyrillic',
    'windows-1250',
    'windows-1252',
    'ISO-8859-1',
    'ISO-8859-15',
  ],
}));

import { EncodingDialog } from './EncodingDialog';
import { commands } from '../lib/tauri';

const readTextFile = vi.mocked(commands.readTextFile);

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

const DETECTED = { text: 'кракозябры вместо кириллицы', encoding: 'windows-1251' };

interface HarnessProps {
  onConfirm: (result: { text: string; encoding: string }) => void;
  onCancel: () => void;
}

function Harness({ onConfirm, onCancel }: HarnessProps) {
  return (
    <MantineProvider>
      <EncodingDialog
        opened
        path="/tmp/notes.txt"
        initial={DETECTED}
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    </MantineProvider>
  );
}

describe('EncodingDialog', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    readTextFile.mockReset();
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function render(onConfirm: HarnessProps['onConfirm'], onCancel: HarnessProps['onCancel']) {
    act(() => {
      root.render(<Harness onConfirm={onConfirm} onCancel={onCancel} />);
    });
  }

  function previewTextarea(): HTMLTextAreaElement | null {
    const el = document.body.querySelector('textarea[aria-label]');
    return el instanceof HTMLTextAreaElement ? el : null;
  }

  function clickButton(label: string) {
    const target = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent === label && !b.hasAttribute('disabled'),
    );
    expect(target, `button "${label}" rendered and enabled`).toBeTruthy();
    act(() => {
      (target as HTMLButtonElement).click();
    });
  }

  it('preselects the detected encoding and previews the decoded text', () => {
    render(vi.fn(), vi.fn());
    // The Select renders as a read-only text input whose value mirrors the
    // chosen label (Mantine jsdom markup has no role="combobox").
    const selectInput = Array.from(document.body.querySelectorAll<HTMLInputElement>('input')).find(
      (i) => i.value === DETECTED.encoding,
    );
    expect(selectInput, 'select preselected with the detected encoding').toBeTruthy();
    expect(previewTextarea()?.value).toContain('кракозябры');
    expect(readTextFile).not.toHaveBeenCalled();
  });

  it('confirm hands the current decode back for the normal import flow', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(onConfirm, onCancel);
    clickButton('Продолжить');
    expect(onConfirm).toHaveBeenCalledWith(DETECTED);
    expect(onCancel).not.toHaveBeenCalled();
    expect(readTextFile).not.toHaveBeenCalled();
  });

  it('cancel aborts without any re-decode call', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(onConfirm, onCancel);
    clickButton('Отмена');
    expect(onCancel).toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
    expect(readTextFile).not.toHaveBeenCalled();
  });
});
