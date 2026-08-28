import type { EntryFormat } from './tauri';

/**
 * Content-only source-format classification behind the preview dialog's
 * «Авто» mode (preview-dialog spec, "Source format auto-detection").
 *
 * Deliberately biased against `html` false positives: reading tags aloud or
 * rendering garbage is the costlier mistake, and an explicit selector choice
 * remains one click away, so the html signal is deliberately precise —
 * markup is delimited by tags.
 */

/** Well-formed tags anchored at the text edges: `<` or `</` + a letter,
 *  optional attributes, then `>`. `a < b` (whitespace after `<`) and `<>`
 *  (no letter) never match. */
const TAG_AT_START = /^<\/?[a-zA-Z][^<>]*>/;
const TAG_AT_END = /<\/?[a-zA-Z][^<>]*>$/;

/** List-item lines required before list density counts as a markdown signal. */
export const MARKDOWN_MIN_LIST_LINES = 3;
/** Inline links required before link density counts as a markdown signal. */
export const MARKDOWN_MIN_INLINE_LINKS = 2;

const HTML_PREFIX = /^<!doctype\s+html|^<html[\s>]/i;
const ATX_HEADING = /^#{1,6} \S/im;
const FENCED_CODE = /^(```|~~~)/m;
const LIST_LINE = /^[ \t]*(?:[-*+]|\d+[.)]) /;
const INLINE_LINK = /\[[^\]\n]+\]\([^)\n]+\)/g;

/** Whitespace and zero-width characters stripped at the text edges before
 *  the boundary tests (web-sourced pastes carry zero-width spaces). */
const INVISIBLE_EDGE = /^[\s\u200B-\u200D\uFEFF]+|[\s\u200B-\u200D\uFEFF]+$/g;

function countMatches(text: string, pattern: RegExp): number {
  return (text.match(pattern) ?? []).length;
}

/**
 * Classify text as the source format it should be ingested as:
 * `html` for a document prefix or for text delimited by tags (both edges are
 * tags after trimming invisible characters), `markdown` for strong
 * structural signals (heading, fenced code, list or link density), `plain`
 * otherwise — including empty text, where the choice is moot.
 */
export function detectFormat(text: string): EntryFormat {
  const trimmed = text.replace(INVISIBLE_EDGE, '');
  if (!trimmed) return 'plain';
  if (HTML_PREFIX.test(trimmed)) return 'html';
  if (TAG_AT_START.test(trimmed) && TAG_AT_END.test(trimmed)) return 'html';
  if (ATX_HEADING.test(trimmed) || FENCED_CODE.test(trimmed)) return 'markdown';
  const listLines = trimmed.split('\n').filter((line) => LIST_LINE.test(line)).length;
  if (listLines >= MARKDOWN_MIN_LIST_LINES) return 'markdown';
  if (countMatches(trimmed, INLINE_LINK) >= MARKDOWN_MIN_INLINE_LINKS) return 'markdown';
  return 'plain';
}
