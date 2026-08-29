// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { MantineProvider } from '@mantine/core';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { GenerationParams, TextEntry } from '../lib/tauri';
import { useLocaleStore } from '../stores/locale';

import { GenerationParamsDialog } from './GenerationParamsDialog';

const SNAPSHOT: GenerationParams = {
  engine: 'silero_native',
  voice: 'ruslan',
  sample_rate: 24000,
  model: { name: 'silero_v5_ru', sha256: 'ab12'.repeat(8) },
  app_version: '0.5.0',
  code_block_mode: 'read',
  read_operators: true,
  normalized_text_sha256: 'cd34'.repeat(8),
  audio_codec: 'Ogg Opus',
  audio_bytes: 1536 * 1024,
};

function makeEntry(overrides: Partial<TextEntry> = {}): TextEntry {
  return {
    id: 'entry-1',
    original_text: 'тест',
    normalized_text: null,
    status: 'ready',
    format: null,
    html_source: null,
    source: null,
    created_at: '2026-08-01T10:00:00',
    audio_generated_at: '2026-08-01T10:00:05',
    audio_path: 'entry-1.opus',
    timestamps_path: null,
    duration_sec: 75,
    was_regenerated: false,
    generation_count: 1,
    generation: SNAPSHOT,
    error_message: null,
    ...overrides,
  };
}

describe('GenerationParamsDialog', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    useLocaleStore.setState({ locale: 'ru' });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
  });

  afterEach(() => {
    act(() => root.unmount());
    host.remove();
  });

  function render(entry: TextEntry | null): void {
    act(() => {
      root.render(
        <MantineProvider>
          <GenerationParamsDialog entry={entry} opened onClose={() => {}} />
        </MantineProvider>,
      );
    });
  }

  it('renders the snapshot with localized names and audio facts', () => {
    render(makeEntry({ source: 'url' }));

    const body = document.body.textContent ?? "";
    expect(body).toContain('Параметры записи');
    expect(body).toContain('Ссылка');
    expect(body).toContain('Silero (нативный)');
    expect(body).toContain('Руслан (мужской)');
    expect(body).toContain('24000 Гц');
    expect(body).toContain('silero_v5_ru');
    expect(body).toContain('0.5.0');
    expect(body).toContain('читать');
    expect(body).toContain('Да');
    // sha256 display is shortened to 12 chars with an ellipsis.
    expect(body).toContain('ab12ab12ab12…');
    expect(body).toContain('Ogg Opus, 1.5 МБ');
    // 75 s → 1:15.
    expect(body).toContain('1:15');
  });

  it('renders the clipboard source localized', () => {
    render(makeEntry({ source: 'clipboard' }));
    expect(document.body.textContent ?? '').toContain('Буфер обмена');
  });

  it('renders absent values as a dash', () => {
    render(
      makeEntry({
        generation: {
          ...SNAPSHOT,
          model: null,
          sample_rate: null,
          audio_codec: null,
          audio_bytes: null,
          read_operators: null,
          code_block_mode: null,
          normalized_text_sha256: null,
        },
      }),
    );

    const body = document.body.textContent ?? '';
    expect(body).not.toContain('silero_v5_ru');
    // Every nulled row shows the placeholder: source, sample rate, model,
    // code-block mode, operator reading, text checksum, and the audio row.
    expect(body.split('—').length - 1).toBe(7);
  });

  it('shows the legacy line for an entry without a snapshot', () => {
    render(makeEntry({ generation: null, generation_count: 0 }));

    const body = document.body.textContent ?? "";
    expect(body).toContain(
      'Параметры не записаны: аудио создано в более старой версии приложения.',
    );
    expect(body).toContain('—');
  });

  it('renders nothing without an entry', () => {
    render(null);
    expect(document.body.textContent).not.toContain('Параметры записи');
  });
});
