import { useEffect, useRef, useState } from 'react';
import { Alert, Button, Group, Modal, Progress, Stack, Text } from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { commands, events } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import { bundleDownloadPercent } from '../lib/bundleDownload';
import type { MessageKey } from '../i18n/ru';
import type { BundlePromptEngine } from '../lib/bundlePrompt';

interface BundlePromptProps {
  opened: boolean;
  /** Engine whose bundle to offer; `null` renders no prompt. */
  engine: BundlePromptEngine | null;
  onClose: () => void;
}

interface BundleProgress {
  file: string;
  percent: number;
}

/** Everything that varies between the bundle engines; the mechanics are shared. */
interface BundlePromptCopy {
  title: MessageKey;
  body: MessageKey;
  downloadLabel: MessageKey;
  /** One-line pointer to the lighter alternative (TeraTTSv2 only). */
  alternative: MessageKey | null;
  readyTitle: MessageKey;
  readyMessage: MessageKey;
  start: () => Promise<void>;
}

const PROMPTS: Record<BundlePromptEngine, BundlePromptCopy> = {
  teratts: {
    title: 'bundle.prompt.teratts.title',
    body: 'bundle.prompt.teratts.body',
    downloadLabel: 'bundle.prompt.teratts.download',
    alternative: 'bundle.prompt.teratts.alternative',
    readyTitle: 'bundle.ready.teratts.title',
    readyMessage: 'bundle.ready.teratts.message',
    start: () => commands.downloadTerattsBundle(),
  },
  silero_native: {
    title: 'bundle.prompt.silero_native.title',
    body: 'bundle.prompt.silero_native.body',
    downloadLabel: 'bundle.prompt.silero_native.download',
    alternative: null,
    readyTitle: 'bundle.ready.silero_native.title',
    readyMessage: 'bundle.ready.silero_native.message',
    start: () => commands.downloadSileroNativeBundle(),
  },
};

/**
 * First-run prompt shown when the persisted engine needs a model bundle that
 * is not downloaded yet: offer the one-time download or keep running on the
 * built-in Piper engine for this run (ui spec: first-run model bundle
 * prompt). The engine, size, download command and activation call come from
 * PROMPTS; the progress, error and decline mechanics are shared. Declining
 * persists nothing, so the prompt reappears on the next launch while the
 * bundle is still missing. Closing the prompt mid-download does not cancel the
 * backend transfer, so the outcome is still applied: activation and a green
 * confirmation on success, a notification carrying the backend message on
 * failure. The event listener therefore lives for the component's lifetime
 * instead of only while the dialog is open.
 */
export function BundlePrompt({ opened, engine, onClose }: BundlePromptProps) {
  const tt = useT();
  const prompt = engine === null ? null : PROMPTS[engine];
  const [download, setDownload] = useState<BundleProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Engine of the download this prompt started, or `null` when none is in
  // flight. A ref, not state: the listener below stays subscribed for the
  // component's lifetime, so it must read the current value without the
  // subscription churning when `opened` goes false.
  const downloadEngineRef = useRef<BundlePromptEngine | null>(null);
  // True between bundle_download_started and bundle_download_finished so a
  // rejected command promise can tell a pre-start failure from a mid-download
  // one (the latter already surfaced via the finished event).
  const downloadActiveRef = useRef(false);
  // The handlers pick the inline error or a notification by whether the prompt
  // is still open. A ref because the subscription does not re-run when
  // `opened` changes.
  const openedRef = useRef(opened);

  useEffect(() => {
    openedRef.current = opened;
  }, [opened]);

  // A closed prompt must start clean when it is raised again: an error from a
  // previous attempt would otherwise greet the next opening. The download path
  // manages its own progress state.
  useEffect(() => {
    if (!opened) setError(null);
  }, [opened]);

  // Live bundle-download progress, driven by the backend's
  // bundle_download_* events. Subscribed for the component's lifetime:
  // closing the prompt does not stop the backend download, so its outcome
  // must still be delivered — otherwise the bundle lands on disk silently,
  // the engine is never activated for this session and the failure message is
  // lost. Filtered to the engine this prompt started (`downloadEngineRef`):
  // Settings can start its own download and its events must not close this
  // prompt or activate the wrong engine.
  useEffect(() => {
    const unlisteners = [
      events.bundleDownloadStarted((p) => {
        if (p.engine !== downloadEngineRef.current) return;
        downloadActiveRef.current = true;
        setDownload({ file: 'manifest.json', percent: 0 });
      }),
      events.bundleDownloadProgress((p) => {
        if (p.engine !== downloadEngineRef.current) return;
        setDownload({ file: p.file, percent: bundleDownloadPercent(p) });
      }),
      events.bundleDownloadFinished((p) => {
        const target = downloadEngineRef.current;
        if (target === null || p.engine !== target) return;
        downloadEngineRef.current = null;
        downloadActiveRef.current = false;
        setDownload(null);
        if (p.ok) {
          // The persisted engine already names this engine; the update only
          // makes the EngineSwitcher rebuild onto it for this session now
          // that the bundle exists. Runs after the prompt was closed too.
          commands
            .updateConfig({ engine: target })
            .then(() => {
              notifications.show({
                title: tt(PROMPTS[target].readyTitle),
                message: tt(PROMPTS[target].readyMessage),
                color: 'green',
              });
            })
            .catch((err) => {
              // The bundle is on disk but the engine failed to start — do
              // not claim activation; the next launch picks it up.
              notifications.show({
                title: tt('bundle.downloaded_not_started.title'),
                message: formatError(err),
                color: 'red',
              });
            });
          onClose();
        } else {
          const message = p.message ?? tt('bundle.unknown_error');
          if (openedRef.current) {
            setError(message);
          } else {
            // The prompt is already closed, so an inline alert would be
            // invisible: surface the backend message as a notification.
            notifications.show({
              title: tt('bundle.prompt.error_title'),
              message,
              color: 'red',
            });
          }
        }
      }),
    ];
    return () => {
      unlisteners.forEach((u) => {
        u.then((fn) => fn()).catch(() => {});
      });
    };
    // onClose is a per-render inline closure; including it would resubscribe
    // the listener every render. `tt` re-subscribes on locale switch so
    // notifications use the active catalog.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tt]);

  const handleDownload = () => {
    if (prompt === null || engine === null) return;
    setError(null);
    // Switch to the progress view immediately — the started event lands one
    // IPC round-trip later, and without this the button stays clickable and
    // the user can queue a second download.
    setDownload({ file: 'manifest.json', percent: 0 });
    downloadEngineRef.current = engine;
    prompt.start().catch((err) => {
      // A finished event already settled this download (it clears the ref
      // before the rejected promise arrives): keep the backend's message
      // instead of overwriting it with the generic IPC one.
      if (downloadEngineRef.current === null) return;
      // Mid-download failures are reported by the finished event; only a
      // command that failed before starting needs the error here.
      if (downloadActiveRef.current) return;
      downloadEngineRef.current = null;
      setDownload(null);
      const message = formatError(err);
      setError((prev) => prev ?? message);
      if (!openedRef.current) {
        // The download could not even start and the prompt is already gone:
        // the inline alert would be invisible.
        notifications.show({
          title: tt('bundle.prompt.error_title'),
          message,
          color: 'red',
        });
      }
    });
  };

  return (
    <Modal
      opened={opened && prompt !== null}
      onClose={onClose}
      title={prompt === null ? '' : tt(prompt.title)}
      centered
    >
      <Stack gap="md">
        {download ? (
          <>
            <Text size="sm">{tt('bundle.prompt.downloading', [download.file])}</Text>
            <Progress value={download.percent} />
          </>
        ) : (
          <Text size="sm">{prompt === null ? '' : tt(prompt.body)}</Text>
        )}
        {error && (
          <Alert color="red" title={tt('bundle.prompt.error_title')}>
            {error}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {tt('bundle.prompt.stay_on_piper')}
          </Button>
          <Button
            onClick={handleDownload}
            loading={download !== null}
            disabled={download !== null}
          >
            {prompt === null ? '' : tt(prompt.downloadLabel)}
          </Button>
        </Group>
        {prompt?.alternative && (
          <Text size="xs" c="dimmed">
            {tt(prompt.alternative)}
          </Text>
        )}
      </Stack>
    </Modal>
  );
}
