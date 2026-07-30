import { writeText, writeImage } from '@tauri-apps/plugin-clipboard-manager';
import { fetch } from '@tauri-apps/plugin-http';
import { Image } from '@tauri-apps/api/image';
import { notifications } from '@mantine/notifications';
import { formatError } from './errors';
import { resolveUrl } from './urls';

/**
 * Copy helpers for the viewer context menu and the copy-link hotkey
 * (viewer-copy-actions spec). All clipboard writes go through
 * `@tauri-apps/plugin-clipboard-manager`; failures surface as a red
 * notification and leave the clipboard unchanged.
 */

function notifyError(err: unknown): void {
  notifications.show({
    title: 'Ошибка',
    message: formatError(err),
    color: 'red',
  });
}

/** Copy a link URL to the clipboard, verbatim as in the source markup. */
export async function copyLinkAddress(href: string): Promise<void> {
  try {
    await writeText(href);
  } catch (err) {
    notifyError(err);
  }
}

/** Copy the current text selection to the clipboard. */
export async function copySelection(text: string): Promise<void> {
  try {
    await writeText(text);
  } catch (err) {
    notifyError(err);
  }
}

/** Copy an image URL to the clipboard, verbatim as in the source markup. */
export async function copyImageAddress(src: string): Promise<void> {
  try {
    await writeText(src);
  } catch (err) {
    notifyError(err);
  }
}

/**
 * Fetch an image (remote images included, via tauri-plugin-http to bypass
 * webview CORS) and write its bitmap to the clipboard.
 */
export async function copyImageBitmap(src: string): Promise<void> {
  try {
    const response = await fetch(resolveUrl(src));
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const bytes = await response.arrayBuffer();
    const image = await Image.fromBytes(bytes);
    await writeImage(image);
  } catch (err) {
    notifyError(err);
  }
}
