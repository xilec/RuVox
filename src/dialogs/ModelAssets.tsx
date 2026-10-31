import { useEffect, useState } from 'react';
import {
  Alert,
  Badge,
  Button,
  Collapse,
  Group,
  Loader,
  Modal,
  Progress,
  Stack,
  Table,
  Text,
  Tooltip,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { commands, events } from '../lib/tauri';
import type {
  ModelAssetId,
  ModelAssetStatus,
  ModelStatus,
  VoiceDownloadProgressPayload,
} from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';
import { bundleDownloadPercent } from '../lib/bundleDownload';
import { PIPER_VOICES } from '../lib/piperVoices';

interface Props {
  opened: boolean;
  onClose: () => void;
  /** Called after install/delete so Settings can re-probe engine availability. */
  onAvailabilityChanged: () => void;
}

interface DownloadState {
  asset: string;
  file: string;
  percent: number;
}

// Coarse 20-point quality guide from the internal TTS research. It combines
// UTMOS/DNSMOS with the practical caveat from REPORT.md: Piper scores high on
// UTMOS but is capped at 22.05 kHz and sounds buzzy; Silero has a full band but
// more hiss and a synthetic timbre; TeraTTSv2 is the cleanest full-band
// compromise. The exact numbers stay out of the UI by design.
const QUALITY_SCORES = {
  silero_native: 15,
  teratts: 18,
  piper: 12,
} as const;

function EngineQualityBar({ score, label }: { score: number; label: string }) {
  return (
    <Tooltip label={label} withArrow zIndex={600}>
      <Progress.Root
        size="sm"
        radius="xl"
        w={150}
        role="img"
        aria-label={label}
        data-quality-bar
      >
        {Array.from({ length: 20 }, (_, index) => (
          <Progress.Section
            key={index}
            value={5}
            color={index < score ? 'teal.5' : 'dark.4'}
          />
        ))}
      </Progress.Root>
    </Tooltip>
  );
}

/** Filled radio marker for the asset the current engine is using. */
function ActiveAssetIcon({ hint }: { hint: string }) {
  return (
    <Tooltip label={hint} withArrow zIndex={600}>
      <span
        role="img"
        aria-label={hint}
        tabIndex={0}
        data-active-icon
        style={{
          display: 'inline-flex',
          cursor: 'help',
          color: 'var(--mantine-color-teal-4)',
          verticalAlign: 'middle',
        }}
      >
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
          <circle cx="9" cy="9" r="8" fill="currentColor" opacity="0.16" />
          <circle cx="9" cy="9" r="7.25" stroke="currentColor" strokeWidth="1" />
          <circle cx="9" cy="9" r="2.6" fill="currentColor" />
        </svg>
      </span>
    </Tooltip>
  );
}

function formatBytes(bytes: number, mb: string, gb: string): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} ${gb}`;
  return `${Math.max(0, Math.round(bytes / 1024 ** 2))} ${mb}`;
}

function voiceId(asset: ModelAssetId): string {
  return asset.startsWith('piper:') ? asset.slice('piper:'.length) : asset;
}

function voicePercent(p: VoiceDownloadProgressPayload): number {
  const fileProgress = p.total_bytes ? Math.min(1, p.downloaded_bytes / p.total_bytes) : 0;
  return Math.min(100, ((p.file_idx + fileProgress) / Math.max(1, p.total_files)) * 100);
}

function assetLabel(asset: ModelAssetStatus, t: ReturnType<typeof useT>): string {
  if (asset.id === 'silero_native') return t('settings.models.silero_native');
  if (asset.id === 'teratts') return t('settings.models.teratts');
  const id = voiceId(asset.id);
  const voice = PIPER_VOICES.find((v) => v.id === id);
  return voice ? t(voice.key) : id;
}

/**
 * Model inventory as one table per model family: the first column owns the
 * framework/bundle and its actions, the second explains the voices (bundled
 * for Silero/TeraTTS, expandable for Piper), the third shows disk usage.
 */
export function ModelAssetsDialog({ opened, onClose, onAvailabilityChanged }: Props) {
  const t = useT();
  const formatSize = (bytes: number) => formatBytes(bytes, t('common.mb'), t('common.gb'));
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checkMessage, setCheckMessage] = useState<string | null>(null);
  const [download, setDownload] = useState<DownloadState | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ModelAssetStatus | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [piperOpen, setPiperOpen] = useState(false);

  const load = (force = false) => {
    setLoading(true);
    commands
      .getModelStatus(force)
      .then(setStatus)
      .catch(() => setStatus(null))
      .finally(() => setLoading(false));
  };

  const checkUpdates = () => {
    setChecking(true);
    setCheckMessage(null);
    commands
      .getModelStatus(true)
      .then((next) => {
        setStatus(next);
        if (next.channel_error) {
          setCheckMessage(t('settings.models.check_failed'));
          return;
        }
        const updates = next.assets.filter(
          (asset) =>
            asset.kind === 'bundle' &&
            asset.installed &&
            !asset.skipped &&
            asset.latest_version != null &&
            asset.latest_version !== asset.installed_version,
        ).length;
        setCheckMessage(
          updates > 0
            ? t('settings.models.check_updates_found', [String(updates)])
            : t('settings.models.check_no_updates'),
        );
      })
      .catch(() => setCheckMessage(t('settings.models.check_failed')))
      .finally(() => setChecking(false));
  };

  useEffect(() => {
    if (opened) {
      setCheckMessage(null);
      load(false);
    }
    // `load` is intentionally not a dependency: it only re-runs on open.
  }, [opened]);

  useEffect(() => {
    if (!opened) return;
    const unlisteners = [
      events.bundleDownloadStarted((p) => {
        setDownload({ asset: p.engine, file: 'manifest.json', percent: 0 });
      }),
      events.bundleDownloadProgress((p) => {
        setDownload({ asset: p.engine, file: p.file, percent: bundleDownloadPercent(p) });
      }),
      events.bundleDownloadFinished((p) => {
        setDownload(null);
        load(false);
        onAvailabilityChanged();
        if (p.ok) {
          notifications.show({
            title: t('settings.models.installed.title'),
            message: t('settings.models.installed.message'),
            color: 'green',
          });
        } else {
          notifications.show({
            title: t('settings.models.failed.title'),
            message: p.message ?? t('bundle.unknown_error'),
            color: 'red',
          });
        }
      }),
      // Voice downloads share the global notification bridge for the final
      // toast; here they only drive the in-dialog progress bar and refresh.
      events.voiceDownloadStarted((p) => {
        setDownload({ asset: `piper:${p.voice}`, file: 'config', percent: 0 });
      }),
      events.voiceDownloadProgress((p) => {
        setDownload({
          asset: `piper:${p.voice}`,
          file: p.file_kind,
          percent: voicePercent(p),
        });
      }),
      events.voiceDownloadFinished(() => {
        setDownload(null);
        load(false);
        onAvailabilityChanged();
      }),
    ];
    return () => {
      unlisteners.forEach((u) => {
        u.then((fn) => fn()).catch(() => {});
      });
    };
  }, [opened, t, onAvailabilityChanged]);

  const install = (asset: ModelAssetStatus) => {
    setDownload({ asset: asset.id, file: 'manifest.json', percent: 0 });
    const start =
      asset.kind === 'voice'
        ? commands.downloadPiperVoice(voiceId(asset.id))
        : commands.installModelAsset(asset.id);
    start.catch((err) => {
      setDownload(null);
      notifications.show({
        title: t('settings.models.failed.title'),
        message: formatError(err),
        color: 'red',
      });
    });
  };

  const remove = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      const freed = await commands.deleteModelAsset(deleteTarget.id);
      setDeleteTarget(null);
      load(false);
      onAvailabilityChanged();
      notifications.show({
        title: t('settings.models.deleted.title'),
        message: t('settings.models.deleted.message', [formatSize(freed)]),
        color: 'green',
      });
    } catch (err) {
      notifications.show({
        title: t('settings.models.delete_failed.title'),
        message: formatError(err),
        color: 'red',
      });
    } finally {
      setDeleting(false);
    }
  };

  const skip = (asset: ModelAssetStatus) => {
    if (!asset.latest_version) return;
    commands
      .skipModelUpdate(asset.id, asset.latest_version)
      .then(() => load(false))
      .catch((err) => {
        notifications.show({
          title: t('settings.models.skip_failed.title'),
          message: formatError(err),
          color: 'red',
        });
      });
  };

  const bundleAssets = status?.assets.filter((a) => a.kind === 'bundle') ?? [];
  const voiceAssets = status?.assets.filter((a) => a.kind === 'voice') ?? [];
  const installedVoices = voiceAssets.filter((a) => a.installed).length;
  const piperBytes = voiceAssets.reduce((sum, a) => sum + a.installed_bytes, 0);

  const progress = (asset: ModelAssetStatus) => {
    if (download?.asset !== asset.id) return null;
    return (
      <Stack gap={2} mt="xs">
        <Text size="xs" c="dimmed">
          {t('settings.models.downloading', [download.file, Math.round(download.percent)])}
        </Text>
        <Progress value={download.percent} animated />
      </Stack>
    );
  };

  const renderBundleRow = (asset: ModelAssetStatus) => {
    const hasNewer =
      asset.installed &&
      asset.latest_version != null &&
      asset.latest_version !== asset.installed_version;
    const voices = asset.bundled_voices ?? [];
    const version = asset.installed
      ? (asset.installed_version ?? t('settings.models.version_unknown'))
      : '—';
    const model = asset.model_id
      ? `${asset.model_id}${asset.upstream_revision ? ` · ${asset.upstream_revision.slice(0, 8)}` : ''}`
      : '—';
    const update = asset.latest_version
      ? asset.installed && !hasNewer
        ? t('settings.models.up_to_date')
        : `${asset.latest_version}${asset.latest_bytes != null ? ` (${formatSize(asset.latest_bytes)})` : ''}`
      : asset.installed
        ? t('settings.models.update_unknown')
        : '—';

    return (
      <Table.Tr key={asset.id}>
        <Table.Td>
          <Group gap={6} wrap="nowrap">
            <Text size="sm" fw={500}>
              {assetLabel(asset, t)}
            </Text>
            {asset.active && <ActiveAssetIcon hint={t('settings.models.active_hint')} />}
            {hasNewer && !asset.skipped && (
              <Badge size="xs" variant="light" color="orange">
                {t('settings.models.update_badge')}
              </Badge>
            )}
            {asset.skipped && (
              <Badge size="xs" variant="light" color="gray">
                {t('settings.models.skipped')}
              </Badge>
            )}
          </Group>
          <EngineQualityBar
            score={asset.id === 'silero_native' ? QUALITY_SCORES.silero_native : QUALITY_SCORES.teratts}
            label={t('settings.models.quality_hint')}
          />
          <Text size="xs" c="dimmed" mt={8}>
            {t('settings.models.version_label')}: {version}
          </Text>
          <Text size="xs" c="dimmed">
            {t('settings.models.model_label')}: {model}
          </Text>
          <Text size="xs" c="dimmed">
            {t('settings.models.update_label')}: {update}
          </Text>
          {asset.skipped && (
            <Text size="xs" c="dimmed">
              {t('settings.models.skipped_hint')}
            </Text>
          )}
          {progress(asset) ?? (
            <Group gap="xs" mt="xs">
              {!asset.installed && (
                <Button
                  size="xs"
                  variant="default"
                  disabled={download !== null}
                  onClick={() => install(asset)}
                >
                  {t('settings.models.download_button')}
                </Button>
              )}
              {hasNewer && !asset.active && (
                <Button
                  size="xs"
                  variant="default"
                  disabled={download !== null}
                  onClick={() => install(asset)}
                >
                  {t('settings.models.update_button')}
                </Button>
              )}
              {hasNewer && !asset.skipped && (
                <Button size="xs" variant="subtle" onClick={() => skip(asset)}>
                  {t('settings.models.skip_button')}
                </Button>
              )}
              {asset.installed && (
                <Button
                  size="xs"
                  variant="default"
                  color="gray"
                  disabled={!asset.deletable || deleting}
                  onClick={() => setDeleteTarget(asset)}
                  styles={{ root: { '&:hover': { color: 'var(--mantine-color-red-6)' } } }}
                >
                  {t('settings.models.delete_button')}
                </Button>
              )}
            </Group>
          )}
        </Table.Td>
        <Table.Td>
          <Text size="xs">
            {voices.length > 0
              ? t('settings.models.bundled_voices', [voices.join(', ')])
              : t('settings.models.bundled_unknown')}
          </Text>
          <Text size="xs" c="dimmed">
            {t('settings.models.bundled_note')}
          </Text>
        </Table.Td>
        <Table.Td ta="right">
          <Text size="xs" c="dimmed">
            {asset.installed ? formatSize(asset.installed_bytes) : '—'}
          </Text>
        </Table.Td>
      </Table.Tr>
    );
  };

  const renderVoiceRow = (asset: ModelAssetStatus) => (
    <Table.Tr key={asset.id}>
      <Table.Td>
        <Group gap={6}>
          <Text size="sm">{assetLabel(asset, t)}</Text>
          {asset.active && <ActiveAssetIcon hint={t('settings.models.voice_active_hint')} />}
        </Group>
      </Table.Td>
      <Table.Td>
        {progress(asset) ?? (
          <Group gap="xs">
            {!asset.installed && (
              <Button
                size="xs"
                variant="default"
                disabled={download !== null}
                onClick={() => install(asset)}
              >
                {t('settings.models.download_button')}
              </Button>
            )}
            {asset.installed && (
              <Text size="xs" c="dimmed">
                {t('settings.models.voice_installed')}
              </Text>
            )}
            {asset.installed && (
              <Button
                size="xs"
                variant="default"
                color="gray"
                disabled={!asset.deletable || deleting}
                onClick={() => setDeleteTarget(asset)}
                styles={{ root: { '&:hover': { color: 'var(--mantine-color-red-6)' } } }}
              >
                {t('settings.models.delete_button')}
              </Button>
            )}
          </Group>
        )}
      </Table.Td>
      <Table.Td ta="right">
        <Text size="xs" c="dimmed">
          {asset.installed ? formatSize(asset.installed_bytes) : '—'}
        </Text>
      </Table.Td>
    </Table.Tr>
  );

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={t('settings.models.dialog_title')}
      size="xl"
      zIndex={450}
    >
      <Stack gap="sm">
        <Group justify="space-between" align="center">
          <Text size="xs" c="dimmed">
            {status ? t('settings.models.total', [formatSize(status.total_bytes)]) : ''}
          </Text>
          <Group gap="sm" align="center" wrap="nowrap">
            {checkMessage && (
              <Text size="xs" c="dimmed">
                {checkMessage}
              </Text>
            )}
            <Button size="xs" variant="subtle" loading={checking} onClick={checkUpdates}>
              {t('settings.models.check_updates')}
            </Button>
          </Group>
        </Group>
        {status?.channel_error && (
          <Alert color="yellow" variant="light" title={t('settings.models.channel_error_title')}>
            <Text size="xs">{t('settings.models.channel_error_hint')}</Text>
            <Text size="xs" c="dimmed" mt={4} style={{ wordBreak: 'break-all' }}>
              {status.channel_error}
            </Text>
          </Alert>
        )}
        {loading && !status ? (
          <Loader size="xs" />
        ) : (
          <Table verticalSpacing="sm" horizontalSpacing="sm">
            <Table.Thead>
              <Table.Tr>
                <Table.Th>{t('settings.models.column_framework')}</Table.Th>
                <Table.Th>{t('settings.models.column_voices')}</Table.Th>
                <Table.Th ta="right">{t('settings.models.column_size')}</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {bundleAssets.map(renderBundleRow)}
              <Table.Tr style={piperOpen ? { borderBottom: 'none' } : undefined}>
                <Table.Td style={piperOpen ? { borderBottom: 'none' } : undefined}>
                  <Text size="sm" fw={500}>
                    Piper
                  </Text>
                  <EngineQualityBar
                    score={QUALITY_SCORES.piper}
                    label={t('settings.models.quality_hint')}
                  />
                  <Text size="xs" c="dimmed" mt={6}>
                    {t('settings.models.piper_builtin')}
                  </Text>
                </Table.Td>
                <Table.Td style={piperOpen ? { borderBottom: 'none' } : undefined}>
                  <Button
                    size="xs"
                    variant="subtle"
                    onClick={() => setPiperOpen((open) => !open)}
                  >
                    {piperOpen ? '▾' : '▸'}{' '}
                    {t('settings.models.piper_voices_count', [
                      installedVoices,
                      voiceAssets.length,
                    ])}
                  </Button>
                </Table.Td>
                <Table.Td ta="right" style={piperOpen ? { borderBottom: 'none' } : undefined}>
                  <Text size="xs" c="dimmed">
                    {formatSize(piperBytes)}
                  </Text>
                </Table.Td>
              </Table.Tr>
              {piperOpen && (
                <Table.Tr style={{ borderTop: 'none', borderBottom: 'none' }}>
                  <Table.Td
                    colSpan={3}
                    p={0}
                    style={{ borderTop: 'none', borderBottom: 'none' }}
                  >
                    <Collapse in={piperOpen}>
                      <Table verticalSpacing="xs" horizontalSpacing="sm" withRowBorders={false}>
                        <Table.Tbody>{voiceAssets.map(renderVoiceRow)}</Table.Tbody>
                      </Table>
                      <Text size="xs" c="dimmed" px="sm" pb="xs">
                        {t('settings.models.voice_update_note')}
                      </Text>
                    </Collapse>
                  </Table.Td>
                </Table.Tr>
              )}
            </Table.Tbody>
          </Table>
        )}

        <Modal
          opened={deleteTarget !== null}
          onClose={() => !deleting && setDeleteTarget(null)}
          title={t('settings.models.delete_title')}
          zIndex={500}
        >
          <Stack gap="sm">
            <Text size="sm">
              {t('settings.models.delete_message', [
                deleteTarget ? assetLabel(deleteTarget, t) : '',
              ])}
            </Text>
            <Group justify="flex-end">
              <Button variant="default" onClick={() => setDeleteTarget(null)} disabled={deleting}>
                {t('settings.models.cancel')}
              </Button>
              <Button color="red" loading={deleting} onClick={remove}>
                {t('settings.models.delete_button')}
              </Button>
            </Group>
          </Stack>
        </Modal>
      </Stack>
    </Modal>
  );
}
