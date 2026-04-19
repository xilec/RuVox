import mermaid from 'mermaid';

let initialized = false;

function initMermaid(): void {
  if (initialized) return;
  mermaid.initialize({
    startOnLoad: false,
    theme: 'default',
    securityLevel: 'loose',
  });
  initialized = true;
}

/** Render all .mermaid elements within the given container. */
export async function renderMermaidIn(
  container: HTMLElement,
  colorScheme: 'light' | 'dark',
): Promise<void> {
  initMermaid();
  mermaid.initialize({
    startOnLoad: false,
    theme: colorScheme === 'dark' ? 'dark' : 'default',
    securityLevel: 'loose',
  });
  const nodes = container.querySelectorAll<HTMLElement>('.mermaid');
  if (nodes.length === 0) return;
  // mermaid.run() re-renders nodes that haven't been processed yet.
  // Reset processed nodes so re-render works on colorScheme change.
  nodes.forEach((n) => {
    n.removeAttribute('data-processed');
  });
  await mermaid.run({ nodes: Array.from(nodes) });
}
