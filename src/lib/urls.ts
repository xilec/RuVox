/**
 * Resolve a possibly relative or protocol-relative URL (`/path`,
 * `//host/path`) against the document base so copied/shown URLs are always
 * absolute. Returns the input unchanged when it cannot be parsed.
 */
export function resolveUrl(url: string, base: string = document.baseURI): string {
  try {
    return new URL(url, base).href;
  } catch {
    return url;
  }
}
