import type { BundleDownloadProgressPayload } from './tauri';

/**
 * Overall bundle-download percent from one `bundle_download_progress`
 * payload: files completed so far plus the fraction of the current file
 * (`skipped` files count as complete). Single home for the rule — both the
 * Settings dialog and the first-run prompt render progress from it.
 */
export function bundleDownloadPercent(p: BundleDownloadProgressPayload): number {
  const fileFraction = p.skipped ? 1 : p.total_bytes > 0 ? p.downloaded_bytes / p.total_bytes : 0;
  return Math.min(100, ((p.file_idx + fileFraction) / Math.max(1, p.total_files)) * 100);
}
