import type { EntryFormat } from './tauri';
import { sanitizeHtml } from './html';
import { extractTextForTts } from './htmlText';

/**
 * How a submitted text is ingested (preview-dialog spec, reused by the
 * paste/Add HTML paths):
 * - `direct`: ingest the text as-is and persist `format` on the entry;
 * - `html`: ingest the extracted text with `format: "html"` and the
 *   sanitized markup as `html_source`;
 * - `reject`: the html choice yielded no readable text — create no entry.
 */
export type IngestAction =
  | { kind: 'direct'; text: string; format: EntryFormat }
  | { kind: 'html'; text: string; htmlSource: string }
  | { kind: 'reject' };

/** Single home for the sanitize + extract ingest decision. */
export function resolveIngest(text: string, format: EntryFormat): IngestAction {
  if (format !== 'html') return { kind: 'direct', text, format };
  const htmlSource = sanitizeHtml(text);
  const extracted = extractTextForTts(htmlSource);
  if (!extracted.trim()) return { kind: 'reject' };
  return { kind: 'html', text: extracted, htmlSource };
}
