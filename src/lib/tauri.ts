import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type Event, type UnlistenFn } from '@tauri-apps/api/event';

export type { UnlistenFn };

// --- Shared types (mirror docs/ipc-contract.md) ---

export type EntryId = string;

export type EntryStatus = 'pending' | 'processing' | 'ready' | 'playing' | 'error';

export interface TextEntry {
  id: EntryId;
  original_text: string;
  normalized_text: string | null;
  status: EntryStatus;
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

export type Theme = 'light' | 'dark' | 'auto';

export type EngineKind = 'piper' | 'silero';

export interface EngineAvailability {
  available: boolean;
  reason: string | null;
}

export interface AvailableEngines {
  piper: EngineAvailability;
  silero: EngineAvailability;
}

export interface UIConfig {
  speaker: string;
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
  /** Active TTS engine. Defaults to "piper" on fresh installs and on configs
   *  that pre-date the engine selector. */
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

  addTextEntry: (text: string, play_when_ready: boolean): Promise<EntryId> =>
    tauriInvoke('add_text_entry', { text, playWhenReady: play_when_ready }),

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

  getTimestamps: (id: EntryId): Promise<WordTimestamp[]> =>
    tauriInvoke('get_timestamps', { id }),

  clearCache: (args: ClearCacheArgs): Promise<ClearCacheResult> =>
    tauriInvoke('clear_cache', { args }),

  getCacheStats: (): Promise<{ total_bytes: number; audio_file_count: number }> =>
    tauriInvoke('get_cache_stats'),

  getCacheDir: (): Promise<string> =>
    tauriInvoke('get_cache_dir'),

  previewNormalize: (text: string): Promise<PreviewNormalizeResult> =>
    tauriInvoke('preview_normalize', { text }),
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
export interface SynthesisProgressPayload { entry_id: EntryId; progress: number; }

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

  synthesisProgress: (cb: (p: SynthesisProgressPayload) => void): Promise<UnlistenFn> =>
    tauriListen<SynthesisProgressPayload>('synthesis_progress', (e) => cb(e.payload)),

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
};
