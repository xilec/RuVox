# Contributing

Руководство по внесению вклада в RuVox.

## Способы помочь

1. **Расширение словарей** — добавление IT-терминов, аббревиатур, операторов.
2. **Исправление произношения** — корректировка существующих транслитераций.
3. **Баг-репорты** — конкретные неисправности с минимальным воспроизведением.
4. **Новые функции** — обсуждение через issue, затем PR.
5. **Документация** — улучшение описаний, актуализация после изменений.

## Расширение словарей

Pipeline нормализации живёт в `src-tauri/src/pipeline/normalizers/`. Каждый нормалайзер — отдельный Rust-файл с собственной таблицей подстановок. При расширении словаря **обязателен** golden-test.

### Добавление IT-термина

**1. Файл:** `src-tauri/src/pipeline/normalizers/english.rs`

Найти таблицу `IT_TERMS` и добавить запись (поддерживай алфавитный порядок внутри секции):

```rust
pub static IT_TERMS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    // ... существующие термины ...
    "kubernetes" => "кубернетис",
    "terraform"  => "терраформ",
};
```

**2. Golden-фикстура:** `src-tauri/tests/fixtures/pipeline/`

```bash
# Создать вход и эталонный выход
echo "Используем Kubernetes и Terraform" > src-tauri/tests/fixtures/pipeline/it_kubernetes.input.txt
echo "Используем кубернетис и терраформ" > src-tauri/tests/fixtures/pipeline/it_kubernetes.expected.txt

# Сгенерировать char_map.json через legacy-pipeline (для регрессии)
nix-shell --run "PYTHONPATH=legacy/src python3 scripts/generate_golden.py --case it_kubernetes"
```

**3. Прогон:**

```bash
nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml --test golden -- it_kubernetes"
```

**4. Коммит:**

```bash
git checkout -b feat/it-term-kubernetes
git add src-tauri/src/pipeline/normalizers/english.rs src-tauri/tests/fixtures/pipeline/it_kubernetes.*
git commit -m "feat(pipeline): add 'kubernetes' to IT_TERMS dictionary"
```

### Добавление аббревиатуры

Файл: `src-tauri/src/pipeline/normalizers/abbreviations.rs`.

- **Как слово** — таблица `AS_WORD` (`"json" → "джейсон"`).
- **По буквам** — добавлять не нужно: всё, что не в `AS_WORD`, по умолчанию читается побуквенно через `LETTER_MAP`.

### Добавление оператора / символа

Файл: `src-tauri/src/pipeline/normalizers/symbols.rs` или `src-tauri/src/pipeline/constants.rs` (для GREEK_LETTERS, MATH_SYMBOLS, ARROW_SYMBOLS).

Многосимвольные операторы (`===`, `=>`, `>=`) обрабатываются в `pipeline/mod.rs::TRACKED_OPERATOR_KEYS` — порядок «длинный раньше короткого» обязателен.

## Баг-репорты

### Что включить

1. **Входной текст** — что обрабатывали (минимальный пример).
2. **Ожидаемый результат** — как должно звучать.
3. **Фактический результат** — что получилось (`normalized_text` из `~/.cache/ruvox/history.json`).
4. **Версия** — `git log -1 --oneline`.
5. **Окружение** — ОС, NixOS/Nix-shell или ручная установка.

### Пример

```markdown
**Входной текст:**
`Версия >= 2.0`

**Ожидаемый результат:**
"Версия больше или равно два точка ноль"

**Фактический результат:**
"Версия >= два точка ноль"

**Версия:** abc1234
**Окружение:** NixOS 24.11, nix develop
```

## Code Style

См. полные правила в [development.md](development.md#правила-кода). Кратко:

- Rust: edition 2021, `tracing` + `thiserror`, без `unwrap` в production-путях, `cargo fmt` + `cargo clippy`.
- TypeScript: `strict: true`, без `React.FC`, без `any`, функциональные компоненты + hooks.
- Mantine 8: CSS Modules + prop `classNames`. Запрет `sx`, `createStyles`, `emotion`, легаси Mantine 6/7.
- Python (ttsd): 3.12, uv-managed, `ruff check` чистый, JSON через stdin/stdout, логи на stderr.

### Коммиты

```
feat(pipeline): add 'kubernetes' term to IT_TERMS
fix(player): mask mpv seek latency to prevent slider snap-back
docs(ipc-contract): add add_text_entry command
test(pipeline): golden fixture for size_units
refactor(commands): extract spawn_synthesis helper
chore(deps): bump tauri to 2.10
```

- Никаких emoji.
- **Запрещено:** «Co-Authored-By: Claude …».
- `git push` в `main` — только по явному согласованию (см. CLAUDE.md).

## Pull Request

### Чеклист

- [ ] `nix-shell --run "cargo test --manifest-path src-tauri/Cargo.toml"` зелёный.
- [ ] `nix-shell --run "pnpm typecheck"` зелёный.
- [ ] `nix-shell --run "pnpm lint"` зелёный.
- [ ] Если pipeline трогали — golden-фикстура добавлена/обновлена.
- [ ] Если ttsd трогали — `cd ttsd && uv run python -m pytest` зелёный.
- [ ] Документация (`docs/`) актуализирована, если поведение изменилось.
- [ ] Коммит-сообщения в формате `<type>(<module>): <desc>`.

### Описание PR

```markdown
## Что сделано

Добавлен термин "kubernetes" в IT_TERMS словарь нормализации английского.

## Зачем

Часто встречается в технической документации; раньше транслитерировался как
"кьюбернетес" через fallback, что неприемлемо.

## Тестирование

- Добавлен golden-кейс `it_kubernetes`.
- Все существующие golden-тесты проходят.
- Smoke в `pnpm tauri dev`: вставить в буфер строку с Kubernetes, синтез корректный.
```

## Вопросы

Если что-то непонятно — открой issue с тегом `question`.
