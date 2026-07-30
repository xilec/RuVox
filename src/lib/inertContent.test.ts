// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';

import { makeInert } from './inertContent';
import { resolveUrl } from './urls';

function containerOf(html: string): HTMLElement {
  const div = document.createElement('div');
  div.innerHTML = html;
  return div;
}

describe('makeInert', () => {
  it('disables form controls and removes them from tab order', () => {
    const root = containerOf(
      '<button>go</button><select><option>a</option></select>' +
        '<textarea>t</textarea><input type="text"><fieldset>f</fieldset>',
    );
    makeInert(root);
    for (const el of root.querySelectorAll(
      'button, select, textarea, input, fieldset',
    )) {
      expect(el.hasAttribute('disabled')).toBe(true);
      expect((el as HTMLElement).tabIndex).toBe(-1);
    }
  });

  it('removes controls from video and audio but keeps the elements', () => {
    const root = containerOf(
      '<video controls src="v.mp4"></video><audio controls src="a.mp3"></audio>',
    );
    makeInert(root);
    expect(root.querySelector('video')).not.toBeNull();
    expect(root.querySelector('video')?.hasAttribute('controls')).toBe(false);
    expect(root.querySelector('audio')?.hasAttribute('controls')).toBe(false);
  });

  it('sets a tooltip with the original href verbatim (not resolved)', () => {
    const root = containerOf('<a href="/ru/users/maybe_elf/">l</a>');
    makeInert(root);
    expect(root.querySelector('a')?.getAttribute('title')).toBe(
      '/ru/users/maybe_elf/',
    );
  });

  it('keeps links focusable (copy-link hotkey needs a focus target)', () => {
    const root = containerOf('<a href="https://example.com">l</a>');
    makeInert(root);
    const a = root.querySelector('a');
    expect(a?.hasAttribute('disabled')).toBe(false);
    expect(a?.tabIndex).not.toBe(-1);
  });
});

describe('resolveUrl', () => {
  it('resolves protocol-relative URLs against the document base', () => {
    expect(resolveUrl('//habrastorage.org/x.png', 'https://habr.com/')).toBe(
      'https://habrastorage.org/x.png',
    );
  });

  it('resolves relative URLs against the document base', () => {
    expect(resolveUrl('/img/x.png', 'https://habr.com/post/')).toBe(
      'https://habr.com/img/x.png',
    );
  });

  it('returns absolute URLs unchanged', () => {
    expect(resolveUrl('https://example.com/a', 'https://habr.com/')).toBe(
      'https://example.com/a',
    );
  });
});
