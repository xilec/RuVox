import { useComputedColorScheme } from '@mantine/core';
// Both themes are inlined as strings (?inline) so exactly one <style> element
// is rendered, chosen by the computed color scheme — a static import would pin
// the highlight palette to one scheme regardless of the app theme (ui spec:
// code-highlight theme follows color scheme).
import githubLight from 'highlight.js/styles/github.css?inline';
import githubDark from 'highlight.js/styles/github-dark.css?inline';

/**
 * Renders the highlight.js stylesheet matching Mantine's computed color
 * scheme. Mounted inside MantineProvider; swaps without reload when the
 * scheme changes (including `auto` following the OS).
 */
export function CodeTheme() {
  const scheme = useComputedColorScheme('light');
  return <style>{scheme === 'dark' ? githubDark : githubLight}</style>;
}
