// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { writeText, writeImage, httpFetch, fromBytes, notify } = vi.hoisted(
  () => ({
    writeText: vi.fn(),
    writeImage: vi.fn(),
    httpFetch: vi.fn(),
    fromBytes: vi.fn(),
    notify: vi.fn(),
  }),
);

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText,
  writeImage,
}));
vi.mock('@tauri-apps/plugin-http', () => ({ fetch: httpFetch }));
vi.mock('@tauri-apps/api/image', () => ({
  Image: { fromBytes },
}));
vi.mock('@mantine/notifications', () => ({
  notifications: { show: notify },
}));

import {
  copyImageAddress,
  copyImageBitmap,
  copyLinkAddress,
  copySelection,
} from './viewerCopy';

beforeEach(() => {
  vi.clearAllMocks();
});

describe('copyLinkAddress', () => {
  it('writes the original href verbatim, without resolving', async () => {
    await copyLinkAddress('/ru/users/maybe_elf/');
    expect(writeText).toHaveBeenCalledWith('/ru/users/maybe_elf/');
  });
});

describe('copySelection', () => {
  it('writes the selected text to the clipboard', async () => {
    await copySelection('getUserData');
    expect(writeText).toHaveBeenCalledWith('getUserData');
  });

  it('shows an error notification when the write fails', async () => {
    writeText.mockRejectedValueOnce(new Error('denied'));
    await copySelection('x');
    expect(notify).toHaveBeenCalledWith(
      expect.objectContaining({ color: 'red' }),
    );
  });
});

describe('copyImageAddress', () => {
  it('writes the original src verbatim, without resolving', async () => {
    await copyImageAddress('//habrastorage.org/x.png');
    expect(writeText).toHaveBeenCalledWith('//habrastorage.org/x.png');
  });
});

describe('copyImageBitmap', () => {
  it('fetches the image and writes the bitmap to the clipboard', async () => {
    const bytes = new Uint8Array([1, 2, 3]).buffer;
    httpFetch.mockResolvedValueOnce({
      ok: true,
      arrayBuffer: () => Promise.resolve(bytes),
    });
    const image = { rid: 42 };
    fromBytes.mockResolvedValueOnce(image);

    await copyImageBitmap('https://habrastorage.org/x.png');

    expect(httpFetch).toHaveBeenCalledWith('https://habrastorage.org/x.png');
    expect(fromBytes).toHaveBeenCalledWith(bytes);
    expect(writeImage).toHaveBeenCalledWith(image);
    expect(notify).not.toHaveBeenCalled();
  });

  it('shows an error notification and skips the write on HTTP failure', async () => {
    httpFetch.mockResolvedValueOnce({ ok: false, status: 404 });

    await copyImageBitmap('https://habrastorage.org/missing.png');

    expect(writeImage).not.toHaveBeenCalled();
    expect(notify).toHaveBeenCalledWith(
      expect.objectContaining({ color: 'red' }),
    );
  });

  it('shows an error notification and skips the write on network failure', async () => {
    httpFetch.mockRejectedValueOnce(new Error('offline'));

    await copyImageBitmap('https://habrastorage.org/x.png');

    expect(writeImage).not.toHaveBeenCalled();
    expect(notify).toHaveBeenCalledWith(
      expect.objectContaining({ color: 'red' }),
    );
  });
});
