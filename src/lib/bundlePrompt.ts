import type { AvailableEngines, UIConfig } from './tauri';

/**
 * First-run prompt decision: offer the one-time Silero Native bundle
 * download only when the persisted engine is `silero_native` and the
 * availability probe reports the bundle missing. Users who explicitly
 * picked another engine never see the prompt, and once the bundle is on
 * disk the probe flips to available and the prompt stops appearing
 * (ui spec: first-run bundle prompt).
 */
export function shouldOfferBundleDownload(
  config: Pick<UIConfig, 'engine'>,
  availability: AvailableEngines,
): boolean {
  return config.engine === 'silero_native' && !availability.silero_native.available;
}
