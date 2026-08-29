// Shared vitest setup, wired via `test.setupFiles` in vite.config.ts.
//
// jsdom lacks several browser APIs that Mantine components touch at render
// time (matchMedia, ResizeObserver); React's `act` also wants an explicit
// opt-in flag. Stub them here once instead of re-declaring the same
// preamble in every component test file.
//
// Pure unit tests run in the node environment, where `window` is undefined
// — the DOM-dependent parts must stay no-ops there.

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

// jsdom has no ResizeObserver; Mantine's ScrollArea needs it.
class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
(globalThis as { ResizeObserver?: unknown }).ResizeObserver ??=
  ResizeObserverStub;

if (typeof window !== 'undefined') {
  // jsdom has no matchMedia; Mantine providers need it.
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}
