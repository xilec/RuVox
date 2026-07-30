/**
 * Neutralize interactive elements inside rendered viewer content so the
 * viewer stays read-only (text-display spec). This is a UX layer on top of
 * DOMPurify sanitization, not a security boundary: elements stay visible but
 * become inoperable. `<a>` elements are kept focusable (the copy-link hotkey
 * needs a focus target) and get a tooltip with the original `href` verbatim —
 * resolving it against the webview origin would produce a meaningless
 * localhost URL for relative links; link clicks and `<details>` toggling are
 * blocked by the delegated click handler in TextViewer.
 *
 * Runs on the mounted viewer container (all display modes), after React has
 * committed `dangerouslySetInnerHTML`.
 */
export function makeInert(root: HTMLElement): void {
  root
    .querySelectorAll('button, select, textarea, input, option, optgroup, fieldset')
    .forEach((el) => {
      el.setAttribute('disabled', '');
      if (el instanceof HTMLElement) {
        el.tabIndex = -1;
      }
    });

  root.querySelectorAll('video, audio').forEach((el) => {
    el.removeAttribute('controls');
  });

  root.querySelectorAll('a[href]').forEach((a) => {
    a.setAttribute('title', a.getAttribute('href') ?? '');
  });
}
