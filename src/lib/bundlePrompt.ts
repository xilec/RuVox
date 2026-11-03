import type { AvailableEngines } from './tauri';

/** Engines whose first run needs a downloadable model bundle. */
export type BundlePromptEngine = 'teratts' | 'silero_native';

/** Every engine the first-run prompt can be raised for. */
const BUNDLE_ENGINES: readonly BundlePromptEngine[] = ['teratts', 'silero_native'];

function isBundleEngine(engine: string): engine is BundlePromptEngine {
  return (BUNDLE_ENGINES as readonly string[]).includes(engine);
}

/**
 * First-run prompt decision: the engine whose model bundle the persisted
 * engine needs while the availability probe reports it missing, or `null`
 * when no prompt applies. `piper` has no bundle, `silero` (ttsd) needs the
 * Python stack rather than a bundle, and unknown values never prompt. A
 * persisted engine is never migrated, so this is also the engine the prompt
 * activates once the bundle is on disk (ui spec: first-run model bundle
 * prompt).
 */
export function bundlePromptEngine(
  config: { engine: string },
  availability: AvailableEngines,
): BundlePromptEngine | null {
  const engine = config.engine;
  if (!isBundleEngine(engine)) return null;
  return availability[engine].available ? null : engine;
}
