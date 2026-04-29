import { useEffect, type DependencyList } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';

export type EventSubscription = () => Promise<UnlistenFn>;

/**
 * Subscribe to multiple Tauri events; auto-unsubscribes on unmount.
 *
 * The factories close over current state — pass `deps` so the effect
 * re-subscribes when the captured values change, mirroring `useEffect` rules.
 */
export function useTauriEvents(
  subscriptions: EventSubscription[],
  deps: DependencyList = [],
): void {
  useEffect(() => {
    const pending = subscriptions.map((sub) => sub());
    return () => {
      pending.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
