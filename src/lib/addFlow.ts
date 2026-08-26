import type { EntryFormat } from './tauri';

/**
 * Outcome of the Add-button flow decision (preview-dialog spec,
 * "Add flow gating"). Kept pure so the whole matrix is unit-testable
 * without mounting AppShell.
 *
 * `direct-plain.format` is set only by the import flow (importFlow.ts),
 * where the entry format comes from the source's extension rather than the
 * clipboard defaults; the clipboard paths leave it undefined so the entry
 * keeps its historical unset-format behavior.
 */
export type AddAction =
  | { kind: 'empty' }
  | { kind: 'preview'; text: string; format: EntryFormat; plainFallback: string | null }
  | { kind: 'direct-html'; html: string; plainFallback: string | null }
  | { kind: 'direct-plain'; text: string; format?: EntryFormat };

/**
 * Maps the clipboard probe results to the next step of the Add flow.
 *
 * - `html` — the raw `text/html` flavor, or null when the webview cannot
 *   read it (Linux WebKitGTK) or the flavor is absent/blank.
 * - `plain` — the plugin `readText` result; '' means empty or unreadable.
 *
 * With the preview gate enabled, HTML content opens the dialog too — it
 * must not bypass the gate (on WebView2 `navigator.clipboard.read()`
 * succeeds, so without this the dialog would never appear on Windows).
 * The `preview` variant also carries `plainFallback` when the dialog is
 * opened from an auto-detected HTML flavor and a plain flavor exists:
 * markup that yields no readable text then falls back to the plain text,
 * exactly like the ungated direct path. An explicit `html` selector choice
 * in a dialog opened with plain text carries no fallback (`null`) and a
 * failed extraction stays a red error (preview-dialog spec).
 */
export function resolveAddAction(input: {
  html: string | null;
  plain: string;
  previewEnabled: boolean;
  defaultFormat: EntryFormat;
}): AddAction {
  const html = input.html !== null && input.html.trim() ? input.html : null;
  const plain = input.plain.trim() ? input.plain : null;

  if (input.previewEnabled) {
    if (html !== null) {
      return { kind: 'preview', text: html, format: 'html', plainFallback: plain };
    }
    if (plain !== null) {
      return { kind: 'preview', text: plain, format: input.defaultFormat, plainFallback: null };
    }
    return { kind: 'empty' };
  }

  if (html !== null) return { kind: 'direct-html', html, plainFallback: plain };
  if (plain !== null) return { kind: 'direct-plain', text: plain };
  return { kind: 'empty' };
}
