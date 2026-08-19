import type { EntryFormat } from './tauri';

/**
 * Outcome of the Add-button flow decision (preview-dialog spec,
 * "Add flow gating"). Kept pure so the whole matrix is unit-testable
 * without mounting AppShell.
 */
export type AddAction =
  | { kind: 'empty' }
  | { kind: 'preview'; text: string; format: EntryFormat }
  | { kind: 'direct-html'; html: string; plainFallback: string | null }
  | { kind: 'direct-plain'; text: string };

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
    if (html !== null) return { kind: 'preview', text: html, format: 'html' };
    if (plain !== null) {
      return { kind: 'preview', text: plain, format: input.defaultFormat };
    }
    return { kind: 'empty' };
  }

  if (html !== null) return { kind: 'direct-html', html, plainFallback: plain };
  if (plain !== null) return { kind: 'direct-plain', text: plain };
  return { kind: 'empty' };
}
