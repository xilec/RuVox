// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';
import type { TextEntry } from '../lib/tauri';

const { copyLinkAddress } = vi.hoisted(() => ({ copyLinkAddress: vi.fn() }));

vi.mock('../lib/tauri', () => ({
  commands: {
    getTimestamps: vi.fn().mockResolvedValue([]),
    setEntryFormat: vi.fn().mockResolvedValue(undefined),
  },
  events: {
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackPosition: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
    playbackPaused: vi.fn().mockResolvedValue(() => {}),
  },
}));
vi.mock('../lib/mermaid', () => ({
  renderMermaidIn: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('../lib/viewerCopy', () => ({ copyLinkAddress }));

import { TextViewer } from './TextViewer';

function makeEntry(): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'user maybe_elf',
    normalized_text: null,
    status: 'ready',
    format: 'html',
    html_source:
      '<p><a href="/ru/users/maybe_elf/">maybe_elf</a></p>' +
      '<button>Кнопка</button>',
    source: null,
    created_at: '2026-07-27T00:00:00Z',
    audio_generated_at: null,
    audio_path: null,
    timestamps_path: null,
    duration_sec: null,
    was_regenerated: false,
    generation_count: 0,
    generation: null,
    error_message: null,
  };
}

function makeMermaidEntry(): TextEntry {
  return {
    ...makeEntry(),
    format: 'markdown',
    html_source: null,
    source: null,
    original_text:
      '```mermaid\nflowchart LR\n  A[Node]\n  click A "https://example.com"\n```',
  };
}

describe('TextViewer read-only behavior', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function renderWith(entry: TextEntry | null): void {
    act(() => {
      root.render(
        <MantineProvider>
          <TextViewer entry={entry} />
        </MantineProvider>,
      );
    });
  }

  // Regression: the viewer mounts without an entry (placeholder, no content
  // container), so a click listener attached in an effect with empty deps
  // never lands — links stayed navigable. The listener must attach once an
  // entry appears.
  it('blocks link navigation after an entry appears post-mount', () => {
    root = createRoot(host);
    renderWith(null);
    renderWith(makeEntry());

    const link = host.querySelector('a');
    expect(link).not.toBeNull();
    const event = new MouseEvent('click', { bubbles: true, cancelable: true });
    link?.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it('neutralizes interactive elements in the rendered content', () => {
    root = createRoot(host);
    renderWith(makeEntry());

    expect(host.querySelector('button')?.hasAttribute('disabled')).toBe(true);
    expect(host.querySelector('a')?.getAttribute('title')).toBe(
      '/ru/users/maybe_elf/',
    );
  });

  it('copies the verbatim link href on Ctrl+C with a focused link', () => {
    root = createRoot(host);
    renderWith(makeEntry());

    const link = host.querySelector('a');
    link?.focus();
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'c',
          code: 'KeyC',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(copyLinkAddress).toHaveBeenCalledWith('/ru/users/maybe_elf/');
  });

  // KeyboardEvent.key is layout-dependent ('с' under the Russian layout);
  // matching by physical code keeps the hotkey working in both layouts.
  it('copies the link href on Ctrl+C under the Russian keyboard layout', () => {
    root = createRoot(host);
    renderWith(makeEntry());

    const link = host.querySelector('a');
    link?.focus();
    act(() => {
      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'с',
          code: 'KeyC',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(copyLinkAddress).toHaveBeenCalledWith('/ru/users/maybe_elf/');
  });

  // Regression (#159): the zoom modal is portaled outside the viewer
  // container, so the delegated link-blocking handler does not cover it —
  // links inside the zoomed SVG (mermaid `click` directives emit real <a>
  // elements under securityLevel 'loose') navigated the webview.
  it('blocks link navigation inside the mermaid zoom modal', async () => {
    root = createRoot(host);
    renderWith(makeMermaidEntry());

    // renderMermaidIn is mocked; inject the SVG it would have produced.
    const mermaidDiv = host.querySelector('.mermaid');
    expect(mermaidDiv).not.toBeNull();
    mermaidDiv!.innerHTML =
      '<svg><a href="https://example.com"><text>Node</text></a></svg>';

    // Open the zoom modal by clicking the diagram itself (a non-link area).
    const svg = mermaidDiv!.querySelector('svg')!;
    act(() => {
      svg.dispatchEvent(
        new MouseEvent('click', { bubbles: true, cancelable: true }),
      );
    });
    // Mantine Modal mounts its portal through a transition, so the dialog
    // appears asynchronously rather than synchronously after the click.
    let dialog: Element | null = null;
    await act(async () => {
      await vi.waitFor(() => {
        dialog = document.body.querySelector('[role="dialog"]');
        expect(dialog).not.toBeNull();
      });
    });

    const link = dialog!.querySelector('a');
    expect(link).not.toBeNull();

    const click = new MouseEvent('click', { bubbles: true, cancelable: true });
    act(() => {
      link!.dispatchEvent(click);
    });
    expect(click.defaultPrevented).toBe(true);

    // Middle click fires auxclick, not click — it must be blocked too.
    const auxclick = new MouseEvent('auxclick', {
      bubbles: true,
      cancelable: true,
      button: 1,
    });
    act(() => {
      link!.dispatchEvent(auxclick);
    });
    expect(auxclick.defaultPrevented).toBe(true);
  });
});
