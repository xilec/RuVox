import { Button, Group, Modal, Stack, Text } from '@mantine/core';
import type { GenerationParams, ModelParams, TextEntry } from '../lib/tauri';
import { formatDuration, formatMb } from '../lib/format';
import { useT } from '../lib/i18n';
import type { MessageKey } from '../i18n/ru';
import { PIPER_VOICES } from '../lib/piperVoices';
import { useLocaleStore } from '../stores/locale';

export interface GenerationParamsDialogProps {
  entry: TextEntry | null;
  opened: boolean;
  onClose: () => void;
}

type T = (key: MessageKey, params?: readonly (string | number)[]) => string;

/** Placeholder for values the snapshot does not carry — absent, never guessed
 *  (spec `ui`, "Voiceover parameters dialog"). */
const ABSENT = '—';
/** Short display form of a sha256 checksum. */
const SHA_DISPLAY_LEN = 12;

function displayEngine(engine: string, tt: T): string {
  switch (engine) {
    case 'silero_native':
      return tt('generation.engine.silero_native');
    case 'piper':
      return tt('generation.engine.piper');
    case 'silero':
      return tt('generation.engine.silero');
    default:
      return engine;
  }
}

function displayVoice(voice: string, tt: T): string {
  const known = PIPER_VOICES.find((v) => v.id === voice);
  return known ? tt(known.key) : voice;
}

function displayCodeBlockMode(mode: string, tt: T): string {
  switch (mode) {
    case 'read':
      return tt('generation.code_block.read');
    case 'skip':
      return tt('generation.code_block.skip');
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

/** The stored naive timestamps are UTC (storage spec); without the suffix
 *  JS would parse them as local time. Rendered in the app language, not the
 *  webview's system locale. */
function formatGeneratedAt(naiveUtc: string, locale: 'ru' | 'en'): string {
  const tag = locale === 'ru' ? 'ru-RU' : 'en-US';
  return new Date(`${naiveUtc}Z`).toLocaleString(tag);
}

function displayAudio(g: GenerationParams, tt: T): string {
  const parts = [g.audio_codec];
  if (g.audio_bytes != null) parts.push(`${formatMb(g.audio_bytes)} ${tt('common.mb')}`);
  return parts.filter((p) => p != null).join(', ') || ABSENT;
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
    [tt('generation.engine'), g ? displayEngine(g.engine, tt) : ABSENT],
    [tt('generation.voice'), g ? displayVoice(g.voice, tt) : ABSENT],
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
      tt('generation.read_operators'),
      g?.read_operators != null ? (g.read_operators ? tt('common.yes') : tt('common.no')) : ABSENT,
    ],
    [
      tt('generation.normalized_sha'),
      g?.normalized_text_sha256 ? shortSha(g.normalized_text_sha256) : ABSENT,
    ],
    [tt('generation.audio'), g ? displayAudio(g, tt) : ABSENT],
    [tt('generation.duration'), entry.duration_sec != null ? formatDuration(entry.duration_sec) : ABSENT],
    [
      tt('generation.generated_at'),
      entry.audio_generated_at ? formatGeneratedAt(entry.audio_generated_at, locale) : ABSENT,
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
