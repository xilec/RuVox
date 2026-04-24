// Tauri command errors come back as plain JSON objects shaped like
// `{ type: "...", message: "..." }` (see CommandError in commands/mod.rs).
// `err instanceof Error` is therefore false and `String(err)` collapses to
// "[object Object]", hiding the real cause in user-facing toasts.
export function formatError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  if (typeof err === 'object' && err !== null) {
    const message = (err as { message?: unknown }).message;
    if (typeof message === 'string') return message;
    try {
      return JSON.stringify(err);
    } catch {
      return Object.prototype.toString.call(err);
    }
  }
  return String(err);
}
