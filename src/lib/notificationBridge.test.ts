import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

// The bridge registers one handler per backend event; the test drives those
// handlers directly instead of running a Tauri runtime.
const { handlers, notificationsMock } = vi.hoisted(() => ({
  handlers: new Map<string, (payload: unknown) => void>(),
  notificationsMock: { show: vi.fn(), update: vi.fn(), hide: vi.fn() },
}));

vi.mock('@mantine/notifications', () => ({ notifications: notificationsMock }));
vi.mock('./i18n', () => ({ t: (key: string) => key }));
vi.mock('./tauri', () => ({
  events: new Proxy(
    {},
    {
      get: (_target, name: string) => (handler: (payload: unknown) => void) => {
        handlers.set(name, handler);
        return Promise.resolve(() => {});
      },
    },
  ),
}));

import { setupNotificationBridge } from './notificationBridge';

const entryEvent = (status: string, generationCount: number) => ({
  entry: {
    id: 'entry-1',
    status,
    original_text: 'Текст',
    generation_count: generationCount,
  },
});

const fireEntryUpdated = (status: string, generationCount: number) => {
  const handler = handlers.get('entryUpdated');
  expect(handler).toBeDefined();
  handler!(entryEvent(status, generationCount));
};

let cleanup: () => void = () => {};

beforeAll(async () => {
  cleanup = await setupNotificationBridge();
});

afterAll(() => {
  cleanup();
});

beforeEach(() => {
  notificationsMock.show.mockClear();
  notificationsMock.update.mockClear();
  notificationsMock.hide.mockClear();
});

describe('synthesis toast', () => {
  it('announces a completion only when a new generation was baked', () => {
    fireEntryUpdated('processing', 2);
    expect(notificationsMock.show).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'synth-entry-1', loading: true }),
    );

    fireEntryUpdated('ready', 3);

    expect(notificationsMock.update).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'synth-entry-1',
        title: 'notify.synthesis.done.title',
        color: 'green',
      }),
    );
    expect(notificationsMock.hide).not.toHaveBeenCalled();
  });

  it('stays quiet when a regeneration restored the previous recording', () => {
    // A cancelled or failed regeneration flips the entry back to `ready` with
    // the old audio: the generation counter did not move, so no fresh
    // recording exists and the spinner must simply disappear.
    fireEntryUpdated('processing', 2);

    fireEntryUpdated('ready', 2);

    expect(notificationsMock.update).not.toHaveBeenCalled();
    expect(notificationsMock.hide).toHaveBeenCalledWith('synth-entry-1');
  });

  it('hides the spinner on a failure', () => {
    fireEntryUpdated('processing', 2);

    fireEntryUpdated('error', 2);

    expect(notificationsMock.update).not.toHaveBeenCalled();
    expect(notificationsMock.hide).toHaveBeenCalledWith('synth-entry-1');
  });
});
