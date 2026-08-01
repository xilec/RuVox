import type { WordTimestamp } from './tauri';

const HIGHLIGHT_CLASS = 'word-highlight';

/**
 * Binary search: find the index of the timestamp active at `positionSec`
 * (active when `start <= positionSec < end`).
 *
 * `timestamps` MUST be sorted by `start` ascending and non-overlapping —
 * the search silently misses matches otherwise. All producers (ttsd, piper,
 * silero-native) emit sorted, contiguous lists; the sorted half of the
 * invariant is asserted in dev builds where timestamps are loaded (see
 * {@link debugAssertSortedTimestamps}).
 *
 * A position in a gap between words resolves to the closest upcoming word:
 * by the loop invariant every word before `lo` has `end <= positionSec`
 * and every word from `lo` on has `start > positionSec`, so the next word
 * lights up ahead of its interval instead of the highlight blinking out
 * during the pause. Returns -1 only when the position is at or past the
 * last word's `end` (or the list is empty).
 */
export function findActiveTimestamp(
  timestamps: WordTimestamp[],
  positionSec: number,
): number {
  if (timestamps.length === 0) return -1;

  let lo = 0;
  let hi = timestamps.length - 1;

  while (lo <= hi) {
    const mid = (lo + hi) >>> 1;
    const ts = timestamps[mid];
    if (positionSec < ts.start) {
      hi = mid - 1;
    } else if (positionSec >= ts.end) {
      lo = mid + 1;
    } else {
      return mid;
    }
  }

  // Gap between words — highlight the closest upcoming word.
  return lo < timestamps.length ? lo : -1;
}

/**
 * Dev-only assertion of the sorted-by-`start` half of the invariant
 * {@link findActiveTimestamp} relies on (the other half — non-overlapping
 * intervals — is guaranteed by all current producers and not checked here).
 * Logs a console assertion for every
 * out-of-order pair; no-op in production builds. Call once per timestamp
 * load — the list does not change between `playback_position` events, so
 * checking per event would be waste.
 */
export function debugAssertSortedTimestamps(
  timestamps: WordTimestamp[],
): void {
  if (!import.meta.env.DEV) return;
  for (let i = 1; i < timestamps.length; i++) {
    console.assert(
      timestamps[i].start >= timestamps[i - 1].start,
      `word timestamps must be sorted by start; ` +
        `entry at index ${i} starts at ${timestamps[i].start} ` +
        `after ${timestamps[i - 1].start}`,
    );
  }
}

/**
 * Find a span in `container` whose [data-orig-start, data-orig-end] range
 * contains the given character offsets. Prefers an exact match; falls back
 * to any span whose range fully contains [origStart, origEnd).
 */
function findSpanByOrigPos(
  container: HTMLElement,
  origStart: number,
  origEnd: number,
): HTMLElement | null {
  const spans = container.querySelectorAll<HTMLElement>('[data-orig-start]');
  let bestSpan: HTMLElement | null = null;
  let bestSize = Infinity;

  for (const span of spans) {
    const spanStart = parseInt(span.dataset.origStart ?? '', 10);
    const spanEnd = parseInt(span.dataset.origEnd ?? '', 10);
    if (isNaN(spanStart) || isNaN(spanEnd)) continue;

    // Exact match
    if (spanStart === origStart && spanEnd === origEnd) {
      return span;
    }

    // Containment: span covers the whole word range
    if (spanStart <= origStart && spanEnd >= origEnd) {
      const size = spanEnd - spanStart;
      if (size < bestSize) {
        bestSize = size;
        bestSpan = span;
      }
    }
  }

  return bestSpan;
}

/**
 * Update the highlighted word in `container`.
 *
 * Removes the highlight class from the previously highlighted span and adds it
 * to the span corresponding to `timestamps[idx]`. If `idx` is -1, only removes.
 */
export function applyHighlight(
  container: HTMLElement,
  timestamps: WordTimestamp[],
  idx: number,
  prevIdx: number,
): void {
  // Remove highlight from previous span
  if (prevIdx >= 0 && prevIdx < timestamps.length) {
    const [prevStart, prevEnd] = timestamps[prevIdx].original_pos;
    const prevSpan = findSpanByOrigPos(container, prevStart, prevEnd);
    prevSpan?.classList.remove(HIGHLIGHT_CLASS);
  }

  if (idx < 0 || idx >= timestamps.length) return;

  const [origStart, origEnd] = timestamps[idx].original_pos;
  const span = findSpanByOrigPos(container, origStart, origEnd);
  if (!span) return;

  span.classList.add(HIGHLIGHT_CLASS);

  // Scroll into view only when the span is outside the visible area
  const rect = span.getBoundingClientRect();
  const inViewport =
    rect.top >= 0 &&
    rect.bottom <= (window.innerHeight || document.documentElement.clientHeight);
  if (!inViewport) {
    span.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }
}

/**
 * Remove all word-highlight marks from `container`.
 */
export function clearHighlight(container: HTMLElement): void {
  const highlighted = container.querySelectorAll<HTMLElement>(
    `.${HIGHLIGHT_CLASS}`,
  );
  for (const el of highlighted) {
    el.classList.remove(HIGHLIGHT_CLASS);
  }
}
