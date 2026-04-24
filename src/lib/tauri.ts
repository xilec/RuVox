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
  edited_text: string | null;
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

export interface UIConfig {
  speaker: string;
  sample_rate: number;
  speech_rate: number;
  hotkey_read_now: string;
  hotkey_read_later: string;
  notify_on_ready: boolean;
  notify_on_error: boolean;
  text_format: string;
  history_days: number;
  audio_max_files: number;
  audio_regenerated_hours: number;
  max_cache_size_mb: number;
  auto_cleanup_days: number;
  code_block_mode: string;
  read_operators: boolean;
  theme: Theme;
  player_hotkeys: Record<string, string>;
  window_geometry: [number, number, number, number] | null;
  preview_dialog_enabled: boolean;
  preview_threshold: number;
}

export type UIConfigPatch = Partial<UIConfig>;

export interface PreviewNormalizeResult {
  normalized: string;
}

// --- Commands (frontend → backend) ---

export const commands = {
  addClipboardEntry: (play_when_ready: boolean): Promise<EntryId> =>
    tauriInvoke('add_clipboard_entry', { playWhenReady: play_when_ready }),

  getEntries: (): Promise<TextEntry[]> =>
    tauriInvoke('get_entries'),

  getEntry: (id: EntryId): Promise<TextEntry | null> =>
    tauriInvoke('get_entry', { id }),

  deleteEntry: (id: EntryId): Promise<void> =>
    tauriInvoke('delete_entry', { id }),

  deleteAudio: (id: EntryId): Promise<void> =>
    tauriInvoke('delete_audio', { id }),

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

  getTimestamps: (id: EntryId): Promise<WordTimestamp[]> =>
    tauriInvoke('get_timestamps', { id }),

  clearCache: (force: boolean): Promise<{ deleted_files: number; freed_bytes: number }> =>
    tauriInvoke('clear_cache', { force }),

  getCacheStats: (): Promise<{ total_bytes: number; audio_file_count: number }> =>
    tauriInvoke('get_cache_stats'),

  updateEntryEditedText: (id: EntryId, edited: string | null): Promise<void> =>
    tauriInvoke('update_entry_edited_text', { id, edited }),

  previewNormalize: (text: string): Promise<PreviewNormalizeResult> =>
    tauriInvoke('preview_normalize', { text }),
};

// --- Events (backend → frontend) ---

export interface EntryUpdatedPayload { entry: TextEntry; }
export interface PlaybackPositionPayload { position_sec: number; entry_id: EntryId; duration_sec: number | null; }
export interface PlaybackStartedPayload { entry_id: EntryId; duration_sec: number | null; }
export interface PlaybackPausedPayload { entry_id: EntryId; position_sec: number; }
export interface PlaybackFinishedPayload { entry_id: EntryId; }
export interface ModelErrorPayload { message: string; }
export interface TtsErrorPayload { entry_id: EntryId; message: string; }
export interface SynthesisProgressPayload { entry_id: EntryId; progress: number; }

export const events = {
  entryUpdated: (cb: (p: EntryUpdatedPayload) => void): Promise<UnlistenFn> =>
    tauriListen<EntryUpdatedPayload>('entry_updated', (e: Event<EntryUpdatedPayload>) => cb(e.payload)),

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

  synthesisProgress: (cb: (p: SynthesisProgressPayload) => void): Promise<UnlistenFn> =>
    tauriListen<SynthesisProgressPayload>('synthesis_progress', (e) => cb(e.payload)),

  trayReadNow: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('tray_read_now', () => cb()),

  trayReadLater: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('tray_read_later', () => cb()),

  trayOpenSettings: (cb: () => void): Promise<UnlistenFn> =>
    tauriListen<Record<string, never>>('tray_open_settings', () => cb()),
};
