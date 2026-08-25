import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type Event, type UnlistenFn } from '@tauri-apps/api/event';

export type { UnlistenFn };

// --- Shared types (mirror openspec/specs/ipc-commands/spec.md) ---

export type EntryId = string;

export type EntryStatus = 'pending' | 'processing' | 'ready' | 'playing' | 'error';

export type EntryFormat = 'plain' | 'markdown' | 'html';

/** Narrow a free-form string (config value, Select event) to EntryFormat. */
export function toEntryFormat(
  v: string | null | undefined,
  fallback: EntryFormat = 'markdown',
): EntryFormat {
  return v === 'plain' || v === 'markdown' || v === 'html' ? v : fallback;
}

export interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;
  status: EntryStatus;
  /** Display format persisted for this entry; null = never chosen, the
   * viewer falls back to its default mode. */
  format: EntryFormat | null;
  /** Sanitized HTML kept for rendering in HTML mode; set only for
   * HTML-ingested entries (their original_text is the extracted TTS text). */
  html_source: string | null;
  created_at: string;               // ISO 8601
  audio_generated_at: string | null;
  audio_path: string | null;
  timestamps_path: string | null;
  duration_sec: number | null;
  was_regenerated: boolean;
  error_message: string | null;
}

export interface WordTimestamp {
  word: string;
  start: number;
  end: number;
  original_pos: [number, number];
}

type Theme = 'light' | 'dark' | 'auto';

export type EngineKind = 'piper' | 'silero' | 'silero_native';

interface EngineAvailability {
  /** Whether the engine can be selected from the UI. Phase 3 of #42 wires
   *  this to a runtime probe of the ttsd / Python stack; in Phase 2 Silero
   *  is unconditionally `false` and Piper is unconditionally `true`. */
  available: boolean;
  /** Russian-language explanation shown in a tooltip / Alert when
   *  `available` is `false`. Null when the engine is available. */
  reason: string | null;
}

export interface AvailableEngines {
  piper: EngineAvailability;
  silero: EngineAvailability;
  silero_native: EngineAvailability;
}

export interface UIConfig {
  speaker: string;
  /** Shared across engines. Global default 24000 — the native Silero
   *  engine's own default; Piper ignores the field (output rate is fixed
   *  by the voice model). */
  sample_rate: number;
  speech_rate: number;
  notify_on_ready: boolean;
  notify_on_error: boolean;
  text_format: string;
  max_cache_size_mb: number;
  code_block_mode: string;
  read_operators: boolean;
  theme: Theme;
  player_hotkeys: Record<string, string>;
  window_geometry: [number, number, number, number] | null;
  preview_dialog_enabled: boolean;
  /** Active TTS engine. Defaults to "silero_native" on fresh installs and on
   *  configs that pre-date the engine selector. */
  engine: EngineKind;
  /** Active Piper voice id (e.g. "ruslan", "irina"). See piperVoices.ts. */
  piper_voice: string;
}

export type UIConfigPatch = Partial<UIConfig>;

export interface PreviewNormalizeResult {
  normalized: string;
}

export type CleanupMode =
  | { mode: 'size_limit'; target_mb: number }
  | { mode: 'all' };

export interface ClearCacheArgs {
  mode: CleanupMode;
  delete_texts: boolean;
}

export interface ClearCacheResult {
  deleted_files: number;
  deleted_entries: number;
  freed_bytes: number;
}

// --- Commands (frontend → backend) ---

export const commands = {
  addClipboardEntry: (play_when_ready: boolean): Promise<EntryId> =>
    tauriInvoke('add_clipboard_entry', { playWhenReady: play_when_ready }),

  addTextEntry: (
    text: string,
    play_when_ready: boolean,
    format?: EntryFormat,
    html_source?: string,
  ): Promise<EntryId> =>
    tauriInvoke('add_text_entry', {
      text,
      playWhenReady: play_when_ready,
      format: format ?? null,
      htmlSource: html_source ?? null,
    }),

  getEntries: (): Promise<TextEntry[]> =>
    tauriInvoke('get_entries'),

  getEntry: (id: EntryId): Promise<TextEntry | null> =>
    tauriInvoke('get_entry', { id }),

  deleteEntry: (id: EntryId): Promise<void> =>
    tauriInvoke('delete_entry', { id }),

  deleteAudio: (id: EntryId): Promise<void> =>
    tauriInvoke('delete_audio', { id }),

  regenerateEntry: (id: EntryId): Promise<void> =>
    tauriInvoke('regenerate_entry', { id }),

  setEntryFormat: (id: EntryId, format: EntryFormat): Promise<void> =>
    tauriInvoke('set_entry_format', { id, format }),

  cancelSynthesis: (id: EntryId): Promise<void> =>
    tauriInvoke('cancel_synthesis', { id }),

  playEntry: (id: EntryId): Promise<void> =>
    tauriInvoke('play_entry', { id }),

  pausePlayback: (): Promise<void> =>
    tauriInvoke('pause_playback'),

  resumePlayback: (): Promise<void> =>
    tauriInvoke('resume_playback'),

  stopPlayback: (): Promise<void> =>
    tauriInvoke('stop_playback'),

  /** Destroy the mpv subprocess before the updater runs the installer
   * (#211): the installer force-kills the app, so the exit-time cleanup
   * never runs and the orphaned mpv.exe would lock the install dir. */
  shutdownPlayerForUpdate: (): Promise<void> =>
    tauriInvoke('shutdown_player_for_update'),

  seekTo: (position_sec: number): Promise<void> =>
    tauriInvoke('seek_to', { positionSec: position_sec }),

  setSpeed: (speed: number): Promise<void> =>
    tauriInvoke('set_speed', { speed }),

  setVolume: (volume: number): Promise<void> =>
    tauriInvoke('set_volume', { volume }),

  getConfig: (): Promise<UIConfig> =>
    tauriInvoke('get_config'),

  updateConfig: (patch: UIConfigPatch): Promise<void> =>
    tauriInvoke('update_config', { patch }),

  getAvailableEngines: (): Promise<AvailableEngines> =>
    tauriInvoke('get_available_engines'),

  downloadPiperVoice: (voice_id: string): Promise<void> =>
    tauriInvoke('download_piper_voice', { voiceId: voice_id }),

  downloadSileroNativeBundle: (): Promise<void> =>
    tauriInvoke('download_silero_native_bundle'),

  getTimestamps: (id: EntryId): Promise<WordTimestamp[]> =>
    tauriInvoke('get_timestamps', { id }),

  clearCache: (args: ClearCacheArgs): Promise<ClearCacheResult> =>
    tauriInvoke('clear_cache', { args }),

  getCacheStats: (): Promise<{ total_bytes: number; audio_file_count: number }> =>
    tauriInvoke('get_cache_stats'),

  getCacheDir: (): Promise<string> =>
    tauriInvoke('get_cache_dir'),

  getLogDir: (): Promise<string> =>
    tauriInvoke('get_log_dir'),

  previewNormalize: (text: string): Promise<PreviewNormalizeResult> =>
    tauriInvoke('preview_normalize', { text }),

  /** Raw bytes of a remote image for the viewer's "Copy image" action.
   * Fetched by a Rust command (scheme/content-type/size validated there) —
   * the webview holds no arbitrary-host http capability (#231). */
  fetchImageBytes: (url: string): Promise<number[]> =>
    tauriInvoke('fetch_image_bytes', { url }),
};

// --- Events (backend → frontend) ---

export interface EntryUpdatedPayload { entry: TextEntry; }
export interface EntryRemovedPayload { id: EntryId; }
export interface PlaybackPositionPayload { position_sec: number; entry_id: EntryId; duration_sec: number | null; }
export interface PlaybackStartedPayload { entry_id: EntryId; duration_sec: number | null; }
export interface PlaybackPausedPayload { entry_id: EntryId; position_sec: number; }
export interface PlaybackFinishedPayload { entry_id: EntryId; }
export interface ModelErrorPayload { message: string; }
export interface TtsErrorPayload { entry_id: EntryId; message: string; }
export interface TtsFatalPayload { message: string; }

export interface VoiceDownloadStartedPayload {
  engine: 'piper';
  voice: string;
}
export interface VoiceDownloadProgressPayload {
  engine: 'piper';
  voice: string;
  /** "json" or "onnx". */
  file_kind: string;
  file_idx: number;
  total_files: number;
  downloaded_bytes: number;
  /** Server-supplied content-length; null when unknown. */
  total_bytes: number | null;
  /** Set when the file was already on disk and download was skipped. */
  skipped?: boolean;
}
export interface VoiceDownloadFinishedPayload {
  engine: 'piper';
  voice: string;
  ok: boolean;
  /** Russian-language failure message, present when ok=false. */
  message?: string;
}

export interface BundleDownloadStartedPayload {
  engine: 'silero_native';
}
export interface BundleDownloadProgressPayload {
  engine: 'silero_native';
  /** Bundle-relative file path from the manifest. */
  file: string;
  file_idx: number;
  total_files: number;
  downloaded_bytes: number;
  /** Expected size from the manifest. */
  total_bytes: number;
  /** Set when the file was already on disk and valid (sha256 match). */
  skipped?: boolean;
}
export interface BundleDownloadFinishedPayload {
  engine: 'silero_native';
  ok: boolean;
  /** Russian-language failure message, present when ok=false. */
  message?: string;
}

export const events = {
  entryUpdated: (cb: (p: EntryUpdatedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<EntryUpdatedPayload>('entry_updated', (e: Event<EntryUpdatedPayload>) => cb(e.payload)),

  entryRemoved: (cb: (p: EntryRemovedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<EntryRemovedPayload>('entry_removed', (e: Event<EntryRemovedPayload>) => cb(e.payload)),

  playbackPosition: (cb: (p: PlaybackPositionPayload) => void): Promise<UnlistenFn> =>
    tauriListen<PlaybackPositionPayload>('playback_position', (e) => cb(e.payload)),

  playbackStarted: (cb: (p: PlaybackStartedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<PlaybackStartedPayload>('playback_started', (e) => cb(e.payload)),

  playbackPaused: (cb: (p: PlaybackPausedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<PlaybackPausedPayload>('playback_paused', (e) => cb(e.payload)),

  playbackStopped: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('playback_stopped', () => cb()),

  playbackFinished: (cb: (p: PlaybackFinishedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<PlaybackFinishedPayload>('playback_finished', (e) => cb(e.payload)),

  modelLoading: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('model_loading', () => cb()),

  modelLoaded: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('model_loaded', () => cb()),

  modelError: (cb: (p: ModelErrorPayload) => void): Promise<UnlistenFn> =>
    tauriListen<ModelErrorPayload>('model_error', (e) => cb(e.payload)),

  ttsError: (cb: (p: TtsErrorPayload) => void): Promise<UnlistenFn> =>
    tauriListen<TtsErrorPayload>('tts_error', (e) => cb(e.payload)),

  ttsdRestarting: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('ttsd_restarting', () => cb()),

  ttsFatal: (cb: (p: TtsFatalPayload) => void): Promise<UnlistenFn> =>
    tauriListen<TtsFatalPayload>('tts_fatal', (e) => cb(e.payload)),

  trayReadNow: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('tray_read_now', () => cb()),

  trayReadLater: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('tray_read_later', () => cb()),

  voiceDownloadStarted: (cb: (p: VoiceDownloadStartedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<VoiceDownloadStartedPayload>('voice_download_started', (e) => cb(e.payload)),

  voiceDownloadProgress: (cb: (p: VoiceDownloadProgressPayload) => void): Promise<UnlistenFn> =>
    tauriListen<VoiceDownloadProgressPayload>('voice_download_progress', (e) => cb(e.payload)),

  voiceDownloadFinished: (cb: (p: VoiceDownloadFinishedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<VoiceDownloadFinishedPayload>('voice_download_finished', (e) => cb(e.payload)),

  bundleDownloadStarted: (cb: (p: BundleDownloadStartedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<BundleDownloadStartedPayload>('bundle_download_started', (e) => cb(e.payload)),

  bundleDownloadProgress: (cb: (p: BundleDownloadProgressPayload) => void): Promise<UnlistenFn> =>
    tauriListen<BundleDownloadProgressPayload>('bundle_download_progress', (e) => cb(e.payload)),

  bundleDownloadFinished: (cb: (p: BundleDownloadFinishedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<BundleDownloadFinishedPayload>('bundle_download_finished', (e) => cb(e.payload)),
};
