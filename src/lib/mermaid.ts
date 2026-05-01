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
    const fullSource = node.dataset.mermaidSource.trim();
    if (!fullSource) continue;

    // First attempt: full source. If it fails, retry with the part before the
    // first blank line — when a markdown author forgets the closing ```, the
    // mermaid fence absorbs the rest of the document and parsing fails on
    // the trailing prose. Keeping the diagram visible matters more than
    // surfacing the lost prose, which would have been hidden inside the
    // fence anyway.
    let lastError: unknown = null;
    for (const candidate of candidateSources(fullSource)) {
      try {
        await renderInto(node, candidate);
        lastError = null;
        break;
      } catch (e) {
        lastError = e;
      }
    }

    if (lastError !== null) {
      renderError(node, fullSource, lastError);
    }
  }
}

function* candidateSources(source: string): Generator<string> {
  yield source;
  const firstBlock = source.split(/\n[ \t]*\n/, 1)[0];
  if (firstBlock !== source && firstBlock.trim().length > 0) {
    yield firstBlock;
  }
}

async function renderInto(node: HTMLElement, source: string): Promise<void> {
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
  } finally {
    host.remove();
  }
}

function renderError(node: HTMLElement, source: string, error: unknown): void {
  const wrapper = document.createElement('div');
  wrapper.className = 'mermaid-error';
  const hint = document.createElement('div');
  hint.style.cssText = 'color: var(--mantine-color-orange-6, #e67700); font-size: 0.85em; margin-bottom: 0.5em;';
  hint.textContent =
    'Не удалось отрендерить mermaid-диаграмму. Возможно, забыт закрывающий ``` после блока.';
  const pre = document.createElement('pre');
  pre.style.cssText = 'white-space: pre-wrap; margin: 0;';
  pre.textContent = source;
  wrapper.append(hint, pre);
  node.replaceChildren(wrapper);
  // eslint-disable-next-line no-console
  console.error('mermaid render error:', error);
}
