// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { WordTimestamp } from './tauri';
import { applyHighlight, clearHighlight, findActiveTimestamp } from './wordHighlight';

const HIGHLIGHT_CLASS = 'word-highlight';

function ts(start: number, end: number, origStart = 0, origEnd = 0): WordTimestamp {
  return { word: 'w', start, end, original_pos: [origStart, origEnd] };
}

describe('findActiveTimestamp', () => {
  it('returns -1 for an empty timestamp list', () => {
    expect(findActiveTimestamp([], 0)).toBe(-1);
  });

  it('returns -1 when position is before the first timestamp', () => {
    const list = [ts(1, 2)];
    expect(findActiveTimestamp(list, 0)).toBe(-1);
  });

  it('returns -1 when position is after the last timestamp', () => {
    const list = [ts(0, 1)];
    expect(findActiveTimestamp(list, 5)).toBe(-1);
  });

  it('matches the interval start (inclusive lower bound)', () => {
    const list = [ts(1, 2)];
    expect(findActiveTimestamp(list, 1)).toBe(0);
  });

  it('matches strictly inside the interval', () => {
    const list = [ts(1, 2)];
    expect(findActiveTimestamp(list, 1.5)).toBe(0);
  });

  it('excludes the interval end (upper bound is exclusive)', () => {
    const list = [ts(1, 2)];
    expect(findActiveTimestamp(list, 2)).toBe(-1);
  });

  it('at a contiguous boundary, attributes the shared instant to the later interval', () => {
    const list = [ts(0, 1), ts(1, 2)];
    expect(findActiveTimestamp(list, 1)).toBe(1);
  });

  it('returns -1 for a position that falls in a gap between intervals', () => {
    const list = [ts(0, 1), ts(2, 3)];
    expect(findActiveTimestamp(list, 1.5)).toBe(-1);
  });

  it('finds the correct index among several intervals (start, middle, end)', () => {
    const list = [ts(0, 1), ts(1, 2), ts(2, 3), ts(3, 4), ts(4, 5)];
    expect(findActiveTimestamp(list, 0)).toBe(0);
    expect(findActiveTimestamp(list, 2.5)).toBe(2);
    expect(findActiveTimestamp(list, 4.999)).toBe(4);
  });

  it('never matches a zero-duration interval (start === end is unreachable)', () => {
    const list = [ts(1, 1)];
    expect(findActiveTimestamp(list, 1)).toBe(-1);
  });

  it('documents current behavior when timestamps are not sorted ascending', () => {
    // findActiveTimestamp is a binary search: it assumes timestamps are
    // sorted ascending by start. If that invariant is violated, a real
    // match can be missed entirely — here index 2 genuinely contains
    // position 0.5, but the search prunes it away and returns -1.
    // This test pins down current (surprising) behavior; it does not
    // assert that -1 is the "right" answer.
    const list = [ts(100, 101), ts(102, 103), ts(0, 1), ts(104, 105)];
    expect(findActiveTimestamp(list, 0.5)).toBe(-1);
  });
});

describe('DOM helpers (applyHighlight / clearHighlight)', () => {
  let container: HTMLElement;

  beforeEach(() => {
    document.body.innerHTML = '';
    container = document.createElement('div');
    document.body.appendChild(container);
    HTMLElement.prototype.scrollIntoView = vi.fn();
  });

  function addSpan(start: number, end: number, text = 'w'): HTMLElement {
    const span = document.createElement('span');
    span.dataset.origStart = String(start);
    span.dataset.origEnd = String(end);
    span.textContent = text;
    container.appendChild(span);
    return span;
  }

  it('adds the highlight class to the span matching the active timestamp', () => {
    const span0 = addSpan(0, 3, 'foo');
    const span1 = addSpan(4, 7, 'bar');
    const timestamps = [ts(0, 1, 0, 3), ts(1, 2, 4, 7)];

    applyHighlight(container, timestamps, 1, -1);

    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
    expect(span1.classList.contains(HIGHLIGHT_CLASS)).toBe(true);
  });

  it('removes the highlight from the previous span when moving to a new one', () => {
    const span0 = addSpan(0, 3, 'foo');
    const span1 = addSpan(4, 7, 'bar');
    const timestamps = [ts(0, 1, 0, 3), ts(1, 2, 4, 7)];

    applyHighlight(container, timestamps, 0, -1);
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(true);

    applyHighlight(container, timestamps, 1, 0);
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
    expect(span1.classList.contains(HIGHLIGHT_CLASS)).toBe(true);
  });

  it('only removes the previous highlight when idx is -1', () => {
    const span0 = addSpan(0, 3, 'foo');
    const timestamps = [ts(0, 1, 0, 3)];

    applyHighlight(container, timestamps, 0, -1);
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(true);

    applyHighlight(container, timestamps, -1, 0);
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });

  it('does not throw and adds nothing when idx is out of range', () => {
    const span0 = addSpan(0, 3, 'foo');
    const timestamps = [ts(0, 1, 0, 3)];

    expect(() => applyHighlight(container, timestamps, 5, -1)).not.toThrow();
    expect(() => applyHighlight(container, timestamps, -2, -1)).not.toThrow();
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });

  it('does not throw when prevIdx is out of range', () => {
    const span0 = addSpan(0, 3, 'foo');
    const timestamps = [ts(0, 1, 0, 3)];

    expect(() => applyHighlight(container, timestamps, 0, 99)).not.toThrow();
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(true);
  });

  it('falls back to an exact match over a larger containing span', () => {
    // Whole-line span [0, 20) plus an exact word span [5, 9).
    addSpan(0, 20, 'whole line');
    const exact = addSpan(5, 9, 'word');
    const timestamps = [ts(0, 1, 5, 9)];

    applyHighlight(container, timestamps, 0, -1);

    expect(exact.classList.contains(HIGHLIGHT_CLASS)).toBe(true);
  });

  it('falls back to the smallest containing span when no exact match exists', () => {
    const big = addSpan(0, 20, 'paragraph');
    const medium = addSpan(3, 12, 'sentence');
    const timestamps = [ts(0, 1, 5, 9)]; // no span exactly [5, 9)

    applyHighlight(container, timestamps, 0, -1);

    expect(medium.classList.contains(HIGHLIGHT_CLASS)).toBe(true);
    expect(big.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });

  it('breaks ties between equally-sized containing spans by DOM order (first wins)', () => {
    const first = addSpan(0, 10, 'a');
    const second = addSpan(0, 10, 'b');
    const timestamps = [ts(0, 1, 2, 4)];

    applyHighlight(container, timestamps, 0, -1);

    expect(first.classList.contains(HIGHLIGHT_CLASS)).toBe(true);
    expect(second.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });

  it('does nothing when no span matches the requested range', () => {
    const span0 = addSpan(0, 3, 'foo');
    const timestamps = [ts(0, 1, 50, 60)];

    expect(() => applyHighlight(container, timestamps, 0, -1)).not.toThrow();
    expect(span0.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });

  it('scrolls the newly highlighted span into view when it is outside the viewport', () => {
    const span = addSpan(0, 3, 'foo');
    span.getBoundingClientRect = () =>
      ({ top: -100, bottom: -50 }) as DOMRect;
    const timestamps = [ts(0, 1, 0, 3)];

    applyHighlight(container, timestamps, 0, -1);

    expect(span.scrollIntoView).toHaveBeenCalledWith({
      block: 'nearest',
      behavior: 'smooth',
    });
  });

  it('does not scroll when the newly highlighted span is already in the viewport', () => {
    const span = addSpan(0, 3, 'foo');
    span.getBoundingClientRect = () => ({ top: 0, bottom: 10 }) as DOMRect;
    const timestamps = [ts(0, 1, 0, 3)];

    applyHighlight(container, timestamps, 0, -1);

    expect(span.scrollIntoView).not.toHaveBeenCalled();
  });
});

describe('clearHighlight', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('removes the highlight class from every highlighted span', () => {
    const container = document.createElement('div');
    const a = document.createElement('span');
    a.className = HIGHLIGHT_CLASS;
    const b = document.createElement('span');
    b.className = HIGHLIGHT_CLASS;
    const c = document.createElement('span');
    container.append(a, b, c);

    clearHighlight(container);

    expect(a.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
    expect(b.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
    expect(c.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });

  it('is a no-op when nothing is highlighted', () => {
    const container = document.createElement('div');
    const span = document.createElement('span');
    container.appendChild(span);

    expect(() => clearHighlight(container)).not.toThrow();
    expect(span.classList.contains(HIGHLIGHT_CLASS)).toBe(false);
  });
});
