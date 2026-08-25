import type { MessageKey } from './ru';

/**
 * EN catalog — must define every key of the RU source catalog (the type
 * annotation below fails to compile when a key is missing or misspelled).
 */
export const en: Record<MessageKey, string> = {
  // ── Generic ─────────────────────────────────────────────────────────────
  'common.ok': 'OK',
  'common.cancel': 'Cancel',
  'common.delete': 'Delete',
  'common.save': 'Save',
  'common.error': 'Error',

  // ── Error title / generic fallbacks ─────────────────────────────────────
  'errors.title': 'Error',
  'errors.generic.not_found': 'Item not found',
  'errors.generic.storage_error': 'Storage error',
  'errors.generic.synthesis_error': 'Speech synthesis error',
  'errors.generic.playback_error': 'Playback error',
  'errors.generic.config_error': 'Invalid setting value',
  'errors.generic.internal': 'Internal error',

  // ── Backend error codes (CommandError.code) ─────────────────────────────
  'errors.entry.not_found': 'Entry {0} not found',
  'errors.entry.id_invalid': 'Invalid entry id: {0}',
  'errors.entry.not_ready': 'Entry {0} is not ready for playback (status: {1})',
  'errors.entry.cannot_cancel': 'Entry {0} cannot be cancelled (status: {1})',
  'errors.input.empty': 'There is no text to add',
  'errors.input.too_long':
    'The text is too long for the {0} engine (max {1} characters); shorten it or switch to Silero in Settings',
  'errors.clipboard.unavailable': 'Could not open the clipboard',
  'errors.clipboard.empty': 'The clipboard contains no text',
  'errors.clipboard.task_panicked': 'Internal error while reading the clipboard',
  'errors.audio.missing': 'Audio file for entry {0} not found',
  'errors.playback.load_failed': 'Failed to load audio',
  'errors.playback.play_failed': 'Failed to start playback',
  'errors.playback.pause_failed': 'Failed to pause playback',
  'errors.playback.resume_failed': 'Failed to resume playback',
  'errors.playback.stop_failed': 'Failed to stop playback',
  'errors.playback.seek_failed': 'Seek failed',
  'errors.playback.set_speed_failed': 'Failed to change playback speed',
  'errors.playback.set_volume_failed': 'Failed to change volume',
  'errors.speed.out_of_range': 'Speed {0} is outside the allowed range [0.5–3.0]',
  'errors.volume.out_of_range': 'Volume {0} is outside the allowed range [0.0–1.0]',
  'errors.config.engine_switch_failed': 'Failed to switch the synthesis engine',
  'errors.synthesis.failed': 'Speech synthesis error',
  'errors.synthesis.in_progress': 'Entry {0} is already being synthesized',
  'errors.storage.failure': 'Storage error',
  'errors.pipeline.panicked': 'Internal text normalization error',
  'errors.engines.probe_panicked': 'Internal error while probing available engines',
  'errors.cache.task_panicked': 'Internal error during cache cleanup',
  'errors.logs.dir_resolve_failed': 'Could not resolve the log directory',
  'errors.logs.dir_create_failed': 'Could not create the log directory',
  'errors.image.url_invalid': 'Invalid image URL: {0}',
  'errors.image.url_scheme_unsupported': 'URL scheme {0} is not supported for images',
  'errors.image.fetch_failed': 'Failed to download the image',
  'errors.image.fetch_failed.http': 'Failed to download the image (HTTP {0})',
  'errors.image.read_failed': 'Failed to read the image',
  'errors.image.not_image': 'The server returned something other than an image ({0})',
  'errors.image.no_content_type': 'The server did not send a content type',
};
