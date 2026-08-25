/**
 * RU catalog — the source of truth for message keys.
 *
 * Every user-visible string lives here under a stable dotted id.
 * `en.ts` must define every key of this catalog (enforced at compile time).
 * Params are interpolated positionally with `{0}`-style placeholders.
 */
export const ru = {
  // ── Generic ─────────────────────────────────────────────────────────────
  'common.ok': 'ОК',
  'common.cancel': 'Отмена',
  'common.delete': 'Удалить',
  'common.save': 'Сохранить',
  'common.error': 'Ошибка',

  // ── Error title / generic fallbacks ─────────────────────────────────────
  'errors.title': 'Ошибка',
  'errors.generic.not_found': 'Объект не найден',
  'errors.generic.storage_error': 'Ошибка хранилища',
  'errors.generic.synthesis_error': 'Ошибка синтеза речи',
  'errors.generic.playback_error': 'Ошибка воспроизведения',
  'errors.generic.config_error': 'Недопустимое значение настройки',
  'errors.generic.internal': 'Внутренняя ошибка',

  // ── Backend error codes (CommandError.code) ─────────────────────────────
  'errors.entry.not_found': 'Запись {0} не найдена',
  'errors.entry.id_invalid': 'Некорректный идентификатор записи: {0}',
  'errors.entry.not_ready': 'Запись {0} не готова к воспроизведению (статус: {1})',
  'errors.entry.cannot_cancel': 'Запись {0} не может быть отменена (статус: {1})',
  'errors.input.empty': 'Нет текста для добавления',
  'errors.input.too_long':
    'Текст слишком длинный для движка {0} (максимум {1} символов); сократите текст или переключитесь на Silero в настройках',
  'errors.clipboard.unavailable': 'Не удалось открыть буфер обмена',
  'errors.clipboard.empty': 'В буфере обмена нет текста',
  'errors.clipboard.task_panicked': 'Внутренняя ошибка при чтении буфера обмена',
  'errors.audio.missing': 'Аудиофайл для записи {0} не найден',
  'errors.playback.load_failed': 'Ошибка при загрузке аудио',
  'errors.playback.play_failed': 'Ошибка запуска воспроизведения',
  'errors.playback.pause_failed': 'Ошибка паузы воспроизведения',
  'errors.playback.resume_failed': 'Ошибка возобновления воспроизведения',
  'errors.playback.stop_failed': 'Ошибка остановки воспроизведения',
  'errors.playback.seek_failed': 'Ошибка перемотки',
  'errors.playback.set_speed_failed': 'Не удалось изменить скорость воспроизведения',
  'errors.playback.set_volume_failed': 'Не удалось изменить громкость',
  'errors.speed.out_of_range': 'Скорость {0} вне допустимого диапазона [0,5–3,0]',
  'errors.volume.out_of_range': 'Громкость {0} вне допустимого диапазона [0,0–1,0]',
  'errors.config.engine_switch_failed': 'Не удалось переключить движок синтеза',
  'errors.synthesis.failed': 'Ошибка синтеза речи',
  'errors.synthesis.in_progress': 'Запись {0} уже синтезируется',
  'errors.storage.failure': 'Ошибка хранилища',
  'errors.pipeline.panicked': 'Внутренняя ошибка нормализации текста',
  'errors.engines.probe_panicked': 'Внутренняя ошибка проверки доступных движков',
  'errors.cache.task_panicked': 'Внутренняя ошибка очистки кэша',
  'errors.logs.dir_resolve_failed': 'Не удалось определить папку логов',
  'errors.logs.dir_create_failed': 'Не удалось создать папку логов',
  'errors.image.url_invalid': 'Некорректный URL изображения: {0}',
  'errors.image.url_scheme_unsupported': 'Схема {0} не поддерживается для изображений',
  // Network variant has no params; the HTTP-status variant appends " (HTTP …)"
  // via the `errors.image.fetch_failed.http` form (see formatError).
  'errors.image.fetch_failed': 'Не удалось скачать изображение',
  'errors.image.fetch_failed.http': 'Не удалось скачать изображение (HTTP {0})',
  'errors.image.read_failed': 'Не удалось прочитать изображение',
  'errors.image.not_image': 'Сервер вернул не изображение ({0})',
  'errors.image.no_content_type': 'Сервер не указал тип содержимого',
} as const;

export type MessageKey = keyof typeof ru;
