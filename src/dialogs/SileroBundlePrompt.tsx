import { useEffect, useRef, useState } from 'react';
import { Alert, Button, Group, Modal, Progress, Stack, Text } from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { commands, events } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import { bundleDownloadPercent } from '../lib/bundleDownload';

interface SileroBundlePromptProps {
  opened: boolean;
  onClose: () => void;
}

interface BundleProgress {
  file: string;
  percent: number;
}

/**
 * First-run prompt shown when the persisted engine is silero_native but the
 * model bundle is not downloaded: offer the one-time ~230 MB download or
 * keep running on the built-in Piper engine for this run (ui spec:
 * first-run bundle prompt). Declining persists nothing, so the prompt
 * reappears on the next launch while the bundle is still missing.
 */
export function SileroBundlePrompt({ opened, onClose }: SileroBundlePromptProps) {
  const tt = useT();
  const [download, setDownload] = useState<BundleProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  // True between bundle_download_started and bundle_download_finished so a
  // rejected command promise can tell a pre-start failure from a mid-download
  // one (the latter already surfaced via the finished event).
  const downloadActiveRef = useRef(false);

  // Live bundle-download progress, driven by the backend's
  // bundle_download_* events. Subscribed only while the prompt is open.
  useEffect(() => {
    if (!opened) return;
    const unlisteners = [
      events.bundleDownloadStarted(() => {
        downloadActiveRef.current = true;
        setDownload({ file: 'manifest.json', percent: 0 });
      }),
      events.bundleDownloadProgress((p) => {
        setDownload({ file: p.file, percent: bundleDownloadPercent(p) });
      }),
      events.bundleDownloadFinished((p) => {
        downloadActiveRef.current = false;
        setDownload(null);
        if (p.ok) {
          // The persisted engine is already silero_native; the update only
          // makes the EngineSwitcher rebuild onto the native engine for this
          // session now that the bundle exists.
          commands
            .updateConfig({ engine: 'silero_native' })
            .then(() => {
              notifications.show({
                title: tt('bundle.ready.title'),
                message: tt('bundle.ready.message'),
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
          setError(p.message ?? tt('bundle.unknown_error'));
        }
      }),
    ];
    return () => {
      unlisteners.forEach((u) => {
        u.then((fn) => fn()).catch(() => {});
      });
    };
    // onClose is a per-render inline closure; resubscribing on it would churn
    // listeners every render — subscribing on `opened` is sufficient (`tt`
    // re-subscribes on locale switch so toasts use the active catalog).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened, tt]);

  const handleDownload = () => {
    setError(null);
    // Switch to the progress view immediately — the started event lands one
    // IPC round-trip later, and without this the button stays clickable and
    // the user can queue a second download.
    setDownload({ file: 'manifest.json', percent: 0 });
    commands.downloadSileroNativeBundle().catch((err) => {
      // Mid-download failures are already reported by the
      // bundle_download_finished { ok: false } event; only a command that
      // failed before starting needs the inline error here.
      if (!downloadActiveRef.current) {
        setDownload(null);
        setError(formatError(err));
      }
    });
  };

  return (
    <Modal opened={opened} onClose={onClose} title={tt('bundle.prompt.title')} centered>
      <Stack gap="md">
        {download ? (
          <>
            <Text size="sm">{tt('bundle.prompt.downloading', [download.file])}</Text>
            <Progress value={download.percent} />
          </>
        ) : (
          <Text size="sm">
            {tt('bundle.prompt.body')}
          </Text>
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
            {tt('bundle.prompt.download')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
