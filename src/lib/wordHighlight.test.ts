// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { WordTimestamp } from './tauri';
import { applyHighlight, clearHighlight, findActiveTimestamp } from './wordHighlight';

const HIGHLIGHT_CLASS = 'word-highlight';

function ts(start: number, end: number, origStart = 0, origEnd = 0): WordTimestamp {
  return { word: 'w', start, end, original_pos: [origStart, origEnd] };
}

describe('findActiveTimestamp', () => {
  const severalIntervals = [ts(0, 1), ts(1, 2), ts(2, 3), ts(3, 4), ts(4, 5)];

  it.each([
    {
      name: 'returns -1 for an empty timestamp list',
      list: [] as WordTimestamp[],
      pos: 0,
      expected: -1,
    },
    {
      name: 'returns -1 when position is before the first timestamp',
      list: [ts(1, 2)],
      pos: 0,
      expected: -1,
    },
    {
      name: 'returns -1 when position is after the last timestamp',
      list: [ts(0, 1)],
      pos: 5,
      expected: -1,
    },
    {
      name: 'matches the interval start (inclusive lower bound)',
      list: [ts(1, 2)],
      pos: 1,
      expected: 0,
    },
    {
      name: 'matches strictly inside the interval',
      list: [ts(1, 2)],
      pos: 1.5,
      expected: 0,
    },
    {
      name: 'excludes the interval end (upper bound is exclusive)',
      list: [ts(1, 2)],
      pos: 2,
      expected: -1,
    },
    {
      name: 'at a contiguous boundary, attributes the shared instant to the later interval',
      list: [ts(0, 1), ts(1, 2)],
      pos: 1,
      expected: 1,
    },
    {
      name: 'returns -1 for a position that falls in a gap between intervals',
      list: [ts(0, 1), ts(2, 3)],
      pos: 1.5,
      expected: -1,
    },
    {
      name: 'finds the interval at the start among several intervals',
      list: severalIntervals,
      pos: 0,
      expected: 0,
    },
    {
      name: 'finds the interval in the middle among several intervals',
      list: severalIntervals,
      pos: 2.5,
      expected: 2,
    },
    {
      name: 'finds the interval at the end among several intervals',
      list: severalIntervals,
      pos: 4.999,
      expected: 4,
    },
    {
      name: 'never matches a zero-duration interval (start === end is unreachable)',
      list: [ts(1, 1)],
      pos: 1,
      expected: -1,
    },
    {
      // findActiveTimestamp is a binary search: it assumes timestamps are
      // sorted ascending by start. If that invariant is violated, a real
      // match can be missed entirely — here index 2 genuinely contains
      // position 0.5, but the search prunes it away and returns -1.
      // This case pins down current (surprising) behavior; it does not
      // assert that -1 is the "right" answer.
      name: 'documents current behavior when timestamps are not sorted ascending',
      list: [ts(100, 101), ts(102, 103), ts(0, 1), ts(104, 105)],
      pos: 0.5,
      expected: -1,
    },
  ])('$name', ({ list, pos, expected }) => {
    expect(findActiveTimestamp(list, pos)).toBe(expected);
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
