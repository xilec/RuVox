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

import { ViewerContextMenu } from './ViewerContextMenu';

function Harness({
  onAddToDictionary,
}: {
  onAddToDictionary?: (word: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  return (
    <MantineProvider>
      <div ref={ref}>
        <a href="/ru/post/1">ссылка</a>
        <img src="//habrastorage.org/x.png" alt="" />
        <p>вызови getUserData() сейчас</p>
      </div>
      <ViewerContextMenu
        containerRef={ref}
        onAddToDictionary={onAddToDictionary}
      />
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

function clickMenuItem(label: string): void {
  const item = Array.from(
    document.body.querySelectorAll('[role="menuitem"]'),
  ).find((el) => el.textContent === label);
  if (!item) throw new Error(`menu item ${label} not found`);
  act(() => (item as HTMLElement).click());
}

/** Select `phrase` inside `root`, mirroring a user dragging over the text. */
function selectPhrase(root: HTMLElement, phrase: string): void {
  const selection = window.getSelection();
  selection?.removeAllRanges();
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node: Node | null = walker.nextNode();
  while (node !== null) {
    const text = node.textContent ?? '';
    const start = text.indexOf(phrase);
    if (start >= 0) {
      const range = document.createRange();
      range.setStart(node, start);
      range.setEnd(node, start + phrase.length);
      selection?.addRange(range);
      return;
    }
    node = walker.nextNode();
  }
  throw new Error(`selection target not found: ${phrase}`);
}

describe('ViewerContextMenu', () => {
  let host: HTMLDivElement;
  let root: Root;
  let onAddToDictionary: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onAddToDictionary = vi.fn();
    window.getSelection()?.removeAllRanges();
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    act(() => {
      root.render(<Harness onAddToDictionary={onAddToDictionary} />);
    });
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    window.getSelection()?.removeAllRanges();
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

  it('offers the dictionary item before the copy actions for a Latin token', async () => {
    selectPhrase(host, 'getUserData');
    await rightClick(host.querySelector('p') as Element);
    expect(menuItems()).toEqual(['В словарь', 'Копировать']);

    clickMenuItem('В словарь');
    expect(onAddToDictionary).toHaveBeenCalledWith('getUserData');
  });

  it('offers the dictionary item for a Cyrillic token', async () => {
    selectPhrase(host, 'сейчас');
    await rightClick(host.querySelector('p') as Element);
    expect(menuItems()).toEqual(['В словарь', 'Копировать']);
  });

  it('hides the dictionary item for a multi-word selection', async () => {
    selectPhrase(host, 'вызови getUserData');
    await rightClick(host.querySelector('p') as Element);
    expect(menuItems()).toEqual(['Копировать']);
  });

  it('hides the dictionary item for a token with punctuation', async () => {
    selectPhrase(host, 'getUserData()');
    await rightClick(host.querySelector('p') as Element);
    expect(menuItems()).toEqual(['Копировать']);
  });

  it('hides the dictionary item when no callback is provided', async () => {
    act(() => {
      root.render(<Harness />);
    });
    selectPhrase(host, 'getUserData');
    await rightClick(host.querySelector('p') as Element);
    expect(menuItems()).toEqual(['Копировать']);
  });
});
