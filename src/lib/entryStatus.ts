/**
 * Shared entry-status presentation: the badge color and the localized label
 * key used by the queue cards and the viewer header (issue #353).
 */

import type { MessageKey } from '../i18n/ru';
import type { EntryStatus } from './tauri';

export function statusBadgeColor(status: EntryStatus): string {
  switch (status) {
    case 'pending':
      return 'gray';
    case 'processing':
      return 'blue';
    case 'ready':
      return 'green';
    case 'playing':
      return 'teal';
    case 'error':
      return 'red';
    case 'cancelled':
      return 'gray';
  }
}

export const STATUS_KEY: Record<EntryStatus, MessageKey> = {
  pending: 'queue.status.pending',
  processing: 'queue.status.processing',
  ready: 'queue.status.ready',
  playing: 'queue.status.playing',
  error: 'queue.status.error',
  cancelled: 'queue.status.cancelled',
};
