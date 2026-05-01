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
    // Render into a temporary off-screen container. Calling `mermaid.render`
    // without a container makes the library inject and clean up its own host
    // element on `document.body`, which is fragile inside Tauri's WebKit (the
    // SVG sometimes never reaches the caller). An explicit container avoids
    // that path entirely.
    const host = document.createElement('div');
    host.setAttribute('aria-hidden', 'true');
    host.style.cssText = 'position:absolute;visibility:hidden;left:-99999px;top:-99999px;';
    document.body.appendChild(host);
    try {
      const { svg, bindFunctions } = await mermaid.render(id, source, host);
      node.innerHTML = svg;
      bindFunctions?.(node);
    } catch (e) {
      // Show the source inside <pre> so line breaks survive HTML whitespace
      // collapsing — the diagram code stays readable while the user (or we)
      // figure out the syntax problem.
      const pre = document.createElement('pre');
      pre.className = 'mermaid-error';
      pre.textContent = source;
      node.replaceChildren(pre);
      // eslint-disable-next-line no-console
      console.error(
        'mermaid render error:',
        e,
        '\n=== SOURCE FED TO mermaid.render ===\n' + JSON.stringify(source),
      );
    } finally {
      host.remove();
    }
  }
}
