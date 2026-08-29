## MODIFIED Requirements

### Requirement: Queue list behavior

The system SHALL load entries on mount via `commands.getEntries()` sorted by `created_at` descending, and SHALL keep the list in sync by listening to `events.entryUpdated` (prepend new entries, replace existing ones in place) and `events.entryRemoved` (remove and clear the selection if the selected entry was deleted).

Each queue item SHALL show a 60-character preview of `original_text`, a status badge (Ожидание/Обработка/Готово/Играет/Ошибка), the duration when available, and a Play action enabled only for `ready`/`playing` entries. Clicking an item SHALL store it as the selected entry in the Zustand `selectedEntry` store.

Right-clicking an item SHALL open a context menu with "Воспроизвести", "Сохранить аудио как…", "Перегенерировать аудио", "Параметры записи…", "Отменить синтез" (for entries being synthesized) and "Удалить"; deletion MUST be confirmed via `modals.openConfirmModal` before calling `commands.deleteEntry`.

The navbar search input SHALL filter entries case-insensitively by `original_text` substring; when the playing entry is scrolled out of view, a floating "К читаемому" button SHALL appear that selects and scrolls to the playing entry.

#### Scenario: New entry appears at the top

- GIVEN the queue shows existing entries
- WHEN an `entry_updated` event arrives with an entry id not in the list
- THEN the entry is prepended and the list remains sorted by `created_at` descending

#### Scenario: Delete requires confirmation

- GIVEN a queue item's context menu is open
- WHEN the user clicks "Удалить"
- THEN a confirmation modal appears, and only after confirming does the system call `commands.deleteEntry` and remove the item

#### Scenario: Search filters the list

- GIVEN the queue contains entries
- WHEN the user types into "Поиск по записям"
- THEN only entries whose `original_text` contains the query (case-insensitive) remain visible, or "Ничего не найдено" is shown when none match

## ADDED Requirements

### Requirement: Recording parameters dialog

The queue context menu item "Параметры записи…" SHALL open a read-only dialog showing the entry's ingestion source ("Источник": буфер обмена / файл / ссылка) as the first row, followed by the generation snapshot: engine, voice, sample rate, model identity (name and checksum when available), app version, normalization settings used (code-block mode, operator reading), normalized-text checksum, audio codec and size, duration, generation timestamp, and the generation number (`generation_count`). The item SHALL be disabled for entries that have neither a snapshot nor a generation timestamp (never synthesized).

Values absent from the snapshot SHALL render as a placeholder dash rather than guessed values. For entries with audio but no snapshot (synthesized by older builds), the dialog SHALL show an explanatory line that parameters were not recorded. Engine and Piper voice names SHALL be shown with their localized display names; the dialog and menu item SHALL be localized in Russian and English.

#### Scenario: Dialog shows the snapshot

- GIVEN a ready entry whose snapshot records engine `silero_native`, speaker `xenia`, and an Ogg Opus file
- WHEN the user opens "Параметры записи…" for the entry
- THEN the dialog shows the ingestion source first, then the localized engine name, the voice, the sample rate, the audio codec and size, and the generation number

#### Scenario: Absent values render as a dash

- GIVEN a ready entry whose snapshot has no model identity
- WHEN the parameters dialog is open
- THEN the model row renders a placeholder dash, not an invented value

#### Scenario: Legacy entry without a snapshot

- GIVEN a ready entry synthesized by an older build (`generation` is null, `audio_generated_at` is set)
- WHEN the user opens "Параметры записи…" for the entry
- THEN the dialog opens, shows the generation timestamp and duration where known, an explanatory line that parameters were not recorded, and dashes for unknown values

#### Scenario: Item disabled for never-synthesized entries

- GIVEN a pending entry with no audio
- WHEN the user opens the context menu for the entry
- THEN "Параметры записи…" is disabled
