/**
 * Import flow decision layer (text-import spec, #224).
 *
 * Maps an imported source — a local file's decoded text or a fetched page —
 * to the same `AddAction` the clipboard Add flow produces, so AppShell has
 * one executor for every entry point (drag & drop, split-button menu, URL
 * dialog). Pure and unit-testable without mounting components; failures are
 * thrown as `CommandError`-shaped objects that the existing
 * `formatError`/notification chain localizes by code.
 *
 * Format routing: for files the extension is authoritative (.md → markdown,
 * .html/.htm → html, .txt → plain). For URLs — until #241 ships a full
 * content detector — the Content-Type plus a lightweight markup sniff pick
 * between the HTML extraction path and plain text.
 */
import type { AddAction } from './addFlow';
import { previewTextFor } from './html';
import type { EntryFormat } from './tauri';

/** Extensions import accepts, mirroring the backend allowlist. */
export const IMPORTABLE_FILE_EXTENSIONS = ['txt', 'md', 'html', 'htm'] as const;

/**
 * SPA-shell detection thresholds. A fetched page whose *extracted* text is
 * shorter than {@link SPA_MIN_TEXT_CHARS} while its raw markup carries at
 * least {@link SPA_MIN_SCRIPT_TAGS} script tags AND either a known
 * framework mount point or no text at all is reported as JS-rendered.
 * Short server-rendered pages without script bundles stay accepted; partial
 * SSR (some text, many scripts, nonstandard mount id) is deliberately not
 * flagged (accepted false-negative — design.md Risks).
 */
export const SPA_MIN_TEXT_CHARS = 500;
export const SPA_MIN_SCRIPT_TAGS = 2;

const MOUNT_POINT_ID_RE =
  /<\w+[^>]*\bid=["'](?:root|app|__next|__nuxt|q-app|mount|wrapper|main)["']/i;

/** Same shape subset of backend `CommandError` that `formatError`
 *  localizes by code; thrown by this module for import-specific failures.
 *  An Error subclass so `only-throw-error` stays honest, while the wire
 *  fields (`type`/`code`/`params`) keep catalog localization identical to
 *  backend-coded errors. */
export class CodedImportError extends Error {
  readonly type = 'internal' as const;
  readonly code: string;
  readonly params: string[];

  constructor(code: string, params: string[] = []) {
    super(code);
    this.name = 'CodedImportError';
    this.code = code;
    this.params = params;
  }
}

function codedImportError(code: string, params: string[] = []): CodedImportError {
  return new CodedImportError(code, params);
}

/** What is being imported; mirrors `resolveAddAction`'s probe-input style. */
export type ImportSource =
  | { kind: 'file'; fileName: string; text: string }
  | { kind: 'url'; body: string; contentType: string | null };

export interface ImportOptions {
  /** The `preview_dialog_enabled` gate value; imports respect it exactly
   *  like the Add button (preview-dialog spec, "Add flow gating"). */
  previewEnabled: boolean;
}

/** Lowercase trailing extension of a filename ('' when it has none). */
function rawExtension(fileName: string): string {
  const dot = fileName.lastIndexOf('.');
  return dot < 0 ? '' : fileName.slice(dot + 1);
}

/** Extension → format mapping; authoritative for file imports
 *  (spec scenario "Extension decides for files"). Throws the coded error
 *  for extensions outside the allowlist so every entry point reports it
 *  uniformly. */
function formatForFileName(fileName: string): EntryFormat {
  const ext = rawExtension(fileName).toLowerCase();
  switch (ext) {
    case 'md':
      return 'markdown';
    case 'html':
    case 'htm':
      return 'html';
    case 'txt':
      return 'plain';
    default:
      throw codedImportError('import.unsupported_extension', [ext || fileName]);
  }
}

function countScriptTags(markup: string): number {
  return markup.match(/<script\b/gi)?.length ?? 0;
}

/** Raw-markup + extracted-text heuristic deciding whether a fetched page is
 *  a JS-rendered shell (spec "JS-rendered page detection"). */
function looksLikeSpaShell(rawMarkup: string, extractedText: string): boolean {
  if (extractedText.length >= SPA_MIN_TEXT_CHARS) return false;
  if (countScriptTags(rawMarkup) < SPA_MIN_SCRIPT_TAGS) return false;
  // Framework mount point in the markup, or literally nothing readable —
  // covers shells whose mount div uses a name outside the known list.
  return MOUNT_POINT_ID_RE.test(rawMarkup) || extractedText.trim().length === 0;
}

/** Whether a fetched body goes through HTML extraction (`true`) or is read
 *  as plain text under today's interim routing (full auto-detection arrives
 *  with #241). A missing Content-Type falls back to a markup sniff of the
 *  body itself. */
function routesAsHtml(contentType: string | null, body: string): boolean {
  const ct = (contentType ?? '').toLowerCase();
  if (ct.includes('html') || ct.endsWith('+xml')) return true;
  if (ct.startsWith('text/') || ct === '') return /^\s*</.test(body);
  return false;
}

/** Shared gate split: enabled → preview with the routed format preselected;
 *  disabled → direct ingestion honoring the same format. HTML keeps its
 *  existing direct executor shape so extraction failure handling stays in
 *  one place. */
function routed(format: EntryFormat, text: string, o: ImportOptions): AddAction {
  if (o.previewEnabled) return { kind: 'preview', text, format, plainFallback: null };
  if (format === 'html') return { kind: 'direct-html', html: text, plainFallback: null };
  return { kind: 'direct-plain', text, format };
}

/**
 * Map an imported source to the next step of the shared ingestion flow.
 * Throws {@link CodedImportError}-shaped objects for unsupported files,
 * JS-rendered pages, and pages yielding no text — the caller surfaces them
 * through `formatError` notifications instead of opening any dialog
 * (preview-dialog spec: failed imports never open the preview).
 */
export function resolveImport(source: ImportSource, o: ImportOptions): AddAction {
  if (source.kind === 'file') {
    const format = formatForFileName(source.fileName);
    return routed(format, source.text, o);
  }

  const raw = source.body;
  if (!routesAsHtml(source.contentType, raw)) {
    return routed('plain', raw, o);
  }
  const extracted = previewTextFor(raw, 'html');
  if (looksLikeSpaShell(raw, extracted)) {
    throw codedImportError('import.spa_unsupported');
  }
  if (!extracted.trim()) {
    throw codedImportError('import.empty_page');
  }
  return routed('html', raw, o);
}
