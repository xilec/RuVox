import { useEffect, useRef, useState } from 'react';
import { Alert, Button, Group, Modal, Progress, Stack, Text } from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { commands, events } from '../lib/tauri';
import { formatError } from '../lib/errors';

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
        const fileFraction = p.skipped
          ? 1
          : p.total_bytes > 0
            ? p.downloaded_bytes / p.total_bytes
            : 0;
        const percent = Math.min(
          100,
          ((p.file_idx + fileFraction) / Math.max(1, p.total_files)) * 100,
        );
        setDownload({ file: p.file, percent });
      }),
      events.bundleDownloadFinished((p) => {
        downloadActiveRef.current = false;
        setDownload(null);
        if (p.ok) {
          // The persisted engine is already silero_native; the update only
          // makes the EngineSwitcher rebuild onto the native engine for this
          // session now that the bundle exists.
          commands.updateConfig({ engine: 'silero_native' }).catch(() => {});
          notifications.show({
            title: 'Движок Silero готов',
            message: 'Бандл моделей скачан, движок «Silero (нативный)» активирован.',
            color: 'green',
          });
          onClose();
        } else {
          setError(p.message ?? 'неизвестная ошибка');
        }
      }),
    ];
    return () => {
      unlisteners.forEach((u) => {
        u.then((fn) => fn()).catch(() => {});
      });
    };
    // onClose is a per-render inline closure; resubscribing on it would churn
    // listeners every render — subscribing on `opened` is sufficient.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);

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
    <Modal opened={opened} onClose={onClose} title="Скачать движок Silero?" centered>
      <Stack gap="md">
        {download ? (
          <>
            <Text size="sm">Скачивается: {download.file}</Text>
            <Progress value={download.percent} />
          </>
        ) : (
          <Text size="sm">
            Движок по умолчанию — «Silero (нативный)» — звучит заметно лучше, но требует
            одноразового скачивания моделей (~230 МБ). Пока модели не скачаны, приложение
            работает на встроенном движке Piper.
          </Text>
        )}
        {error && (
          <Alert color="red" title="Не удалось скачать бандл">
            {error}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Остаться на Piper
          </Button>
          <Button
            onClick={handleDownload}
            loading={download !== null}
            disabled={download !== null}
          >
            Скачать (~230 МБ)
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
