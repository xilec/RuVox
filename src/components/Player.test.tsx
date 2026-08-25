// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MantineProvider } from '@mantine/core';

const {
  getConfig,
  setSpeed,
  setVolume,
  showNotification,
} = vi.hoisted(() => ({
  getConfig: vi.fn(),
  setSpeed: vi.fn().mockResolvedValue(undefined),
  setVolume: vi.fn().mockResolvedValue(undefined),
  showNotification: vi.fn(),
}));

vi.mock('@mantine/notifications', () => ({
  notifications: { show: showNotification },
}));

vi.mock('../lib/tauri', () => ({
  MIN_SPEED: 0.5,
  MAX_SPEED: 3.0,
  clampSpeed: (v: number) => Math.min(3.0, Math.max(0.5, v)),
  commands: {
    getConfig,
    setSpeed,
    setVolume,
    playEntry: vi.fn().mockResolvedValue(undefined),
    pausePlayback: vi.fn().mockResolvedValue(undefined),
    resumePlayback: vi.fn().mockResolvedValue(undefined),
    seekTo: vi.fn().mockResolvedValue(undefined),
  },
  events: {
    playbackStarted: vi.fn().mockResolvedValue(() => {}),
    playbackPaused: vi.fn().mockResolvedValue(() => {}),
    playbackStopped: vi.fn().mockResolvedValue(() => {}),
    playbackFinished: vi.fn().mockResolvedValue(() => {}),
    playbackPosition: vi.fn().mockResolvedValue(() => {}),
    entryUpdated: vi.fn().mockResolvedValue(() => {}),
  },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

// jsdom has no matchMedia; Mantine providers need it.
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

import { Player } from './Player';

describe('Player speed restore', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    getConfig.mockReset();
    setSpeed.mockClear();
    showNotification.mockClear();
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function speedInput(): HTMLInputElement {
    const input = host.querySelector<HTMLInputElement>(
      'input[aria-label="Скорость воспроизведения"]',
    );
    expect(input).not.toBeNull();
    return input!;
  }

  async function renderPlayer(): Promise<void> {
    await act(async () => {
      root.render(
        <MantineProvider>
          <Player />
        </MantineProvider>,
      );
      // Flush the startup getConfig effect and its follow-up setSpeed.
      await Promise.resolve();
      await Promise.resolve();
    });
  }

  it('restores the persisted speed into the UI and the backend', async () => {
    getConfig.mockResolvedValue({ speech_rate: 2.7 });
    await renderPlayer();

    expect(speedInput().value).toContain('2.7');
    expect(setSpeed).toHaveBeenCalledWith(2.7);
  });

  it('clamps an out-of-range persisted value before applying it', async () => {
    getConfig.mockResolvedValue({ speech_rate: 9.9 });
    await renderPlayer();

    expect(setSpeed).toHaveBeenCalledWith(3.0);
  });

  it('keeps 1.0x and does not touch the backend when config loading fails', async () => {
    getConfig.mockRejectedValue(new Error('no storage'));
    await renderPlayer();

    expect(setSpeed).not.toHaveBeenCalled();
    expect(showNotification).not.toHaveBeenCalled();
  });

  it('rolls the speed back and notifies when setSpeed rejects on a user change', async () => {
    getConfig.mockResolvedValue({ speech_rate: 1.0 });
    await renderPlayer();
    setSpeed.mockRejectedValueOnce(new Error('ipc down'));

    await act(async () => {
      speedInput().dispatchEvent(
        new WheelEvent('wheel', { deltaY: -100, bubbles: true }),
      );
      await Promise.resolve();
      await Promise.resolve();
    });

    // Wheel up proposed 1.1; the rejected IPC call must revert the UI to
    // the last committed value.
    expect(setSpeed).toHaveBeenCalledWith(1.1);
    await vi.waitFor(() => {
      expect(showNotification).toHaveBeenCalledWith(
        expect.objectContaining({ color: 'red' }),
      );
    });
    expect(speedInput().value).toContain('1.0');
  });
});
