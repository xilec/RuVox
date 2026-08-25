import { describe, expect, it } from 'vitest';
import { formatError } from './errors';

interface CommandErrorShape {
  type: string;
  code: string;
  params?: string[];
  message?: string;
}

describe('formatError priority chain', () => {
  it('localizes a known code and interpolates its params', () => {
    const err: CommandErrorShape = {
      type: 'not_found',
      code: 'entry.not_found',
      params: ['entry-3'],
    };
    expect(formatError(err)).toBe('Запись entry-3 не найдена');
  });

  it('falls back to the raw message for an unknown code', () => {
    const err: CommandErrorShape = {
      type: 'internal',
      code: 'mystery.failure',
      message: 'backend exploded',
    };
    expect(formatError(err)).toBe('backend exploded');
  });

  it('falls back to the generic per-type string when there is no code or message', () => {
    expect(formatError({ type: 'playback_error' })).toBe(
      'Ошибка воспроизведения',
    );
    expect(formatError({ type: 'storage_error' })).toBe('Ошибка хранилища');
    expect(formatError({ type: 'config_error', code: '' })).toBe(
      'Недопустимое значение настройки',
    );
  });

  it('uses the HTTP variant of image.fetch_failed when params are present', () => {
    expect(
      formatError({
        type: 'internal',
        code: 'image.fetch_failed',
        params: ['502'],
      }),
    ).toBe('Не удалось скачать изображение (HTTP 502)');
  });

  it('uses the plain image.fetch_failed variant when params are absent', () => {
    expect(formatError({ type: 'internal', code: 'image.fetch_failed' })).toBe(
      'Не удалось скачать изображение',
    );
  });

  it('passes Error and string inputs through unchanged', () => {
    expect(formatError(new Error('plain'))).toBe('plain');
    expect(formatError('raw string')).toBe('raw string');
  });
});
