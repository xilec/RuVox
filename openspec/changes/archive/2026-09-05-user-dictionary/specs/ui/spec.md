## ADDED Requirements

### Requirement: Dictionary editor

The Settings dialog SHALL contain a "Словарь" section showing the entry
count and a button that opens a dedicated dictionary editor modal (the
nested-modal pattern of the cache cleanup dialog). The editor SHALL provide:
an alphabetical, case-insensitive-substring searchable table of entries
(`from` as typed, `to`, a "переопределяет встроенное" badge on entries
flagged by the backend); add / edit / delete with delete confirmed via the
standard confirm modal; form validation per the user-dictionary spec, where
submitting a `from` that already exists updates that entry's spoken form
with the typed value instead of creating a duplicate. Every completed action saves immediately (no save
button); the modal footer SHALL carry a quiet status line — "Все изменения
сохранены" / "Сохранение…" / "Не сохранено — повторить" (with a retry
action) — and CRUD success SHALL NOT raise toast notifications. Import and
export buttons SHALL use the standard file dialogs; import additionally
accepts a dropped file and, before applying, asks the user to choose
"Объединить" (merge) or "Заменить список" (replace); the import result is
reported as a notification with added / updated / skipped counts. All UI
strings are Russian.

#### Scenario: Open the editor from Settings

- GIVEN the Settings dialog is open
- WHEN the user clicks the dictionary section button
- THEN the dictionary editor modal opens above Settings

#### Scenario: Search filters the list

- GIVEN entries "docker", "kubectl", "nginx"
- WHEN the user types "ku" in the search field
- THEN only the "kubectl" row remains

#### Scenario: Duplicate from updates the existing entry

- GIVEN the entry "docker → докер" exists
- WHEN the user submits a new entry with from "Docker" and spoken form
  "докка"
- THEN the existing entry becomes "docker → докка" and no second entry is
  created

#### Scenario: Delete requires confirmation

- GIVEN the entry list is shown
- WHEN the user invokes delete on a row
- THEN a confirmation modal appears and the entry is removed only after
  confirmation

#### Scenario: Status line reflects saving

- GIVEN the user adds a valid entry
- WHEN the save command is in flight
- THEN the footer reads "Сохранение…" and returns to "Все изменения
  сохранены" on completion — with no toast

#### Scenario: Import asks for the mode

- GIVEN the user drops a dictionary TOML file onto the editor
- WHEN the drop is accepted
- THEN a choice between "Объединить" and "Заменить список" appears, and the
  applied result is reported with added / updated / skipped counts
