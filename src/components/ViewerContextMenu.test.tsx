// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { useRef } from 'react';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/viewerCopy', () => ({
  copyLinkAddress: vi.fn(),
  copySelection: vi.fn(),
  copyImageAddress: vi.fn(),
  copyImageBitmap: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

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
(globalThis as { ResizeObserver?: unknown }).ResizeObserver ??=
  ResizeObserverStub;

import { ViewerContextMenu } from './ViewerContextMenu';

function Harness() {
  const ref = useRef<HTMLDivElement>(null);
  return (
    <MantineProvider>
      <div ref={ref}>
        <a href="/ru/post/1">ссылка</a>
        <img src="//habrastorage.org/x.png" alt="" />
        <p>обычный текст</p>
      </div>
      <ViewerContextMenu containerRef={ref} />
    </MantineProvider>
  );
}

async function rightClick(target: Element): Promise<MouseEvent> {
  const event = new MouseEvent('contextmenu', {
    bubbles: true,
    cancelable: true,
    clientX: 10,
    clientY: 10,
  });
  // The Mantine dropdown mounts into the portal asynchronously (floating-ui
  // positions it over several frames) — dispatch in a sync act, then flush
  // real time in a separate act before asserting on items.
  act(() => {
    target.dispatchEvent(event);
  });
  await act(async () => {
    await new Promise((r) => setTimeout(r, 50));
  });
  return event;
}

function menuItems(): string[] {
  return Array.from(document.body.querySelectorAll('[role="menuitem"]')).map(
    (el) => el.textContent ?? '',
  );
}

describe('ViewerContextMenu', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    act(() => {
      root.render(<Harness />);
    });
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  it('opens the menu with the link item on a link', async () => {
    const event = await rightClick(host.querySelector('a') as Element);
    expect(event.defaultPrevented).toBe(true);
    expect(menuItems()).toEqual(['Скопировать адрес ссылки']);
  });

  it('opens the menu with both image items on an image', async () => {
    await rightClick(host.querySelector('img') as Element);
    expect(menuItems()).toEqual([
      'Скопировать изображение',
      'Скопировать адрес изображения',
    ]);
  });

  it('does not open a menu on plain content without a selection', async () => {
    const event = await rightClick(host.querySelector('p') as Element);
    expect(event.defaultPrevented).toBe(false);
    expect(menuItems()).toEqual([]);
  });

  it('ignores a selection made outside the viewer container', async () => {
    const outside = document.createElement('div');
    outside.textContent = 'чужое выделение';
    document.body.appendChild(outside);
    const range = document.createRange();
    range.selectNodeContents(outside);
    const sel = window.getSelection();
    sel?.removeAllRanges();
    sel?.addRange(range);

    const event = await rightClick(host.querySelector('p') as Element);
    expect(event.defaultPrevented).toBe(false);
    expect(menuItems()).toEqual([]);

    outside.remove();
    sel?.removeAllRanges();
  });
});
