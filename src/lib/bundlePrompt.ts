import type { AvailableEngines, ModelAssetId } from './tauri';

/** Engines whose first run needs a downloadable model bundle. Derived from the
 *  model-asset catalog so a new bundle asset cannot be forgotten here: the
 *  `PROMPTS` record in `BundlePrompt.tsx` is keyed by this type and stops
 *  compiling until the prompt learns to offer the new bundle. */
export type BundlePromptEngine = Exclude<ModelAssetId, `piper:${string}`>;

/** Every engine the first-run prompt can be raised for. `satisfies` ties the
 *  list to `ModelAssetId`: a `piper:*` entry (or a typo) no longer compiles
 *  and an engine dropped from the union is flagged as stale. */
const BUNDLE_ENGINES = [
  'teratts',
  'silero_native',
] as const satisfies readonly BundlePromptEngine[];

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
