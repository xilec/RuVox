import { writeText, writeImage } from '@tauri-apps/plugin-clipboard-manager';
import { Image } from '@tauri-apps/api/image';
import { notifications } from '@mantine/notifications';
import { formatError } from './errors';
import { t } from './i18n';
import { commands } from './tauri';
import { resolveUrl } from './urls';

/**
 * Copy helpers for the viewer context menu and the copy-link hotkey
 * (viewer-copy-actions spec). All clipboard writes go through
 * `@tauri-apps/plugin-clipboard-manager`; failures surface as a red
 * notification and leave the clipboard unchanged.
 */

function notifyError(err: unknown): void {
  notifications.show({
    title: t('errors.title'),
    message: formatError(err),
    color: 'red',
  });
}

/** Write text to the clipboard; failures surface as a red notification. */
async function copyText(text: string): Promise<void> {
  try {
    await writeText(text);
  } catch (err) {
    notifyError(err);
  }
}

/** Copy a link URL to the clipboard, verbatim as in the source markup. */
export async function copyLinkAddress(href: string): Promise<void> {
  await copyText(href);
}

/** Copy the current text selection to the clipboard. */
export async function copySelection(text: string): Promise<void> {
  await copyText(text);
}

/** Copy an image URL to the clipboard, verbatim as in the source markup. */
export async function copyImageAddress(src: string): Promise<void> {
  await copyText(src);
}

/**
 * Fetch an image (remote images included) and write its bitmap to the
 * clipboard. The download runs in a Rust command (#231): it validates
 * scheme/content-type/size and keeps the webview free of any blanket
 * arbitrary-host network capability.
 */
export async function copyImageBitmap(src: string): Promise<void> {
  try {
    const bytes = await commands.fetchImageBytes(resolveUrl(src));
    const image = await Image.fromBytes(new Uint8Array(bytes));
    await writeImage(image);
  } catch (err) {
    notifyError(err);
  }
}
