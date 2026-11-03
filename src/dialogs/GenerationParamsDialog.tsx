import { Button, Group, Modal, Stack, Text } from '@mantine/core';
import type { GenerationParams, ModelParams, TextEntry } from '../lib/tauri';
import { formatDuration, formatMb } from '../lib/format';
import { useT } from '../lib/i18n';
import type { MessageKey } from '../i18n/ru';
import { useLocaleStore } from '../stores/locale';
import {
  ABSENT,
  displayEngine,
  displayVoice,
} from '../lib/generationDisplay';
import { formatNaiveUtcDateTime } from '../lib/datetime';

export interface GenerationParamsDialogProps {
  entry: TextEntry | null;
  opened: boolean;
  onClose: () => void;
}

type T = (key: MessageKey, params?: readonly (string | number)[]) => string;

/** Short display form of a sha256 checksum. */
const SHA_DISPLAY_LEN = 12;

function displayCodeBlockMode(mode: string, tt: T): string {
  switch (mode) {
    case 'read':
      return tt('generation.code_block.read');
    // Snapshots written before the rename may carry the legacy value; it
    // meant the same as brief.
    case 'skip':
    case 'brief':
      return tt('generation.code_block.brief');
    default:
      return mode;
  }
}

function displayModel(model: ModelParams): string {
  return model.sha256
    ? `${model.name} (sha256: ${model.sha256.slice(0, SHA_DISPLAY_LEN)}…)`
    : model.name;
}

function shortSha(sha: string): string {
  return `${sha.slice(0, SHA_DISPLAY_LEN)}…`;
}


function displayAudio(g: GenerationParams, tt: T): string {
  const parts: Array<string | null> = [g.audio_codec];
  if (g.audio_bytes != null) {
    // Short clips sit far below a megabyte — show KiB rather than «0.0 МБ».
    parts.push(
      g.audio_bytes >= 512 * 1024
        ? `${formatMb(g.audio_bytes)} ${tt('common.mb')}`
        : `${Math.max(1, Math.round(g.audio_bytes / 1024))} ${tt('common.kb')}`,
    );
  }
  return parts.filter((p) => p != null).join(', ') || ABSENT;
}

function displaySource(source: NonNullable<TextEntry['source']>, tt: T): string {
  switch (source) {
    case 'clipboard':
      return tt('generation.source.clipboard');
    case 'file':
      return tt('generation.source.file');
    case 'url':
      return tt('generation.source.url');
  }
}

/**
 * Read-only details of an entry's generation snapshot (spec `ui`,
 * "Voiceover parameters dialog"). Values absent from the snapshot render as
 * a dash; entries with audio but no snapshot (synthesized by older builds)
 * show an explanatory line.
 */
export function GenerationParamsDialog({ entry, opened, onClose }: GenerationParamsDialogProps) {
  const tt = useT();
  const locale = useLocaleStore((s) => s.locale);

  if (!entry) return null;

  const g = entry.generation;
  const legacy = g === null && entry.audio_generated_at !== null;

  const rows: Array<[label: string, value: string]> = [
    [tt('generation.source'), entry.source ? displaySource(entry.source, tt) : ABSENT],
    [tt('generation.engine'), g ? displayEngine(g.engine, tt) : ABSENT],
    [tt('generation.voice'), g ? displayVoice(g.engine, g.voice, tt) : ABSENT],
    [
      tt('generation.sample_rate'),
      g?.sample_rate != null ? tt('generation.sample_rate.value', [g.sample_rate]) : ABSENT,
    ],
    [tt('generation.model'), g?.model ? displayModel(g.model) : ABSENT],
    [tt('generation.app_version'), g?.app_version ?? ABSENT],
    [
      tt('generation.code_block'),
      g?.code_block_mode != null ? displayCodeBlockMode(g.code_block_mode, tt) : ABSENT,
    ],
    [
      tt('generation.normalized_sha'),
      g?.normalized_text_sha256 ? shortSha(g.normalized_text_sha256) : ABSENT,
    ],
    [tt('generation.audio'), g ? displayAudio(g, tt) : ABSENT],
    [tt('generation.duration'), entry.duration_sec != null ? formatDuration(entry.duration_sec) : ABSENT],
    [
      tt('generation.generated_at'),
      entry.audio_generated_at
        ? formatNaiveUtcDateTime(entry.audio_generated_at, locale)
        : ABSENT,
    ],
    [tt('generation.count'), String(entry.generation_count)],
  ];

  return (
    <Modal opened={opened} onClose={onClose} title={tt('generation.title')} centered>
      <Stack gap="sm">
        {legacy && (
          <Text size="sm" c="dimmed">
            {tt('generation.legacy')}
          </Text>
        )}
        {rows.map(([label, value]) => (
          <Group key={label} justify="space-between" gap="md" wrap="nowrap">
            <Text size="sm" c="dimmed" style={{ flexShrink: 0 }}>
              {label}
            </Text>
            <Text size="sm" ta="right" style={{ wordBreak: 'break-word' }}>
              {value}
            </Text>
          </Group>
        ))}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {tt('common.close')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
