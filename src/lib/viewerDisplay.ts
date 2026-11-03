import type { DisplayMode, EntryFormat, TextEntry } from './tauri';
import { detectFormat } from './detectFormat';

/**
 * The entry fields the viewer's view derivation needs.
 *
 * `format` is the immutable source format recorded at ingestion, while
 * `display_mode` (legacy entries: absent) is the viewer's display choice.
 * Entries written by older builds can carry a `format` that the old viewer
 * toggle overwrote, so the derivation below never trusts it blindly.
 */
export type ViewerEntry = Pick<
  TextEntry,
  'format' | 'display_mode' | 'html_source' | 'original_text'
>;

/** Views offered for a source format: the rendered view and the verbatim
 * text, or only the verbatim text when there is nothing to render. */
export function viewerModesFor(source: EntryFormat): readonly DisplayMode[] {
  return source === 'plain' ? ['plain'] : ['rendered', 'plain'];
}

/**
 * Derive the source format the viewer may render for an entry
 * (text-display spec, "Allowed views follow the source format").
 *
 * - Stored markup wins: HTML rendering is only possible with `html_source`,
 *   so it identifies the true source even when a legacy toggle left
 *   `format` contradicting it.
 * - An explicit `markdown` / `plain` is the ingestion choice and is trusted.
 * - `null` (older builds, the tray path) and a stale `html` without stored
 *   markup fall back to content detection.
 */
export function deriveSourceFormat(entry: ViewerEntry): EntryFormat {
  if (entry.html_source != null) return 'html';
  if (entry.format === 'markdown' || entry.format === 'plain') return entry.format;
  return detectFormat(entry.original_text);
}

/**
 * Resolve the effective display mode: a plain source can only be shown
 * verbatim, so a persisted `rendered` (or the legacy toggle residue) clamps
 * to `plain`; a rendered source defaults to the rendered view.
 */
export function resolveDisplayMode(
  entry: ViewerEntry,
  source: EntryFormat,
): DisplayMode {
  if (source === 'plain') return 'plain';
  return entry.display_mode === 'plain' ? 'plain' : 'rendered';
}
