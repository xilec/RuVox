import mermaid from 'mermaid';

let renderCounter = 0;

function configureMermaid(colorScheme: 'light' | 'dark'): void {
  // Re-initialize on every call so colorScheme changes take effect on re-render.
  mermaid.initialize({
    startOnLoad: false,
    theme: colorScheme === 'dark' ? 'dark' : 'default',
    securityLevel: 'loose',
  });
}

/**
 * Render all `.mermaid` elements within the container.
 *
 * Uses `mermaid.render(id, code)` directly with the element's `textContent`
 * instead of `mermaid.run({ nodes })`. The latter reads `element.innerHTML` and
 * runs `entityDecode` on it, which round-trips the diagram source through DOM
 * serialization — fragile when the source contains characters that markdown-it
 * escaped (`<`, `>`, `&`) and that the renderer then has to un-escape. Reading
 * `textContent` gives us the raw, already-decoded source string in one step.
 */
export async function renderMermaidIn(
  container: HTMLElement,
  colorScheme: 'light' | 'dark',
): Promise<void> {
  configureMermaid(colorScheme);
  const nodes = container.querySelectorAll<HTMLElement>('.mermaid');
  for (const node of Array.from(nodes)) {
    if (node.dataset.mermaidSource === undefined) {
      node.dataset.mermaidSource = node.textContent ?? '';
    }
    const source = node.dataset.mermaidSource.trim();
    if (!source) continue;

    const id = `mermaid-${Date.now().toString(36)}-${renderCounter++}`;
    try {
      const { svg, bindFunctions } = await mermaid.render(id, source);
      node.innerHTML = svg;
      bindFunctions?.(node);
    } catch (e) {
      // Restore the source as plain text so users can see what failed.
      node.textContent = source;
      throw e;
    }
  }
}
