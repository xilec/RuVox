# RuVox Design System

## Direction

**"Терминал чтеца"** — техничный, текстоцентричный, ненавязчивый.
Интерфейс отступает, текст выходит вперёд.
Ближе к IDE, чем к медиаплееру. Минимум визуального шума.

## Depth Strategy

**Borders only.** Без теней. Плоский, техничный стиль.
Иерархия поверхностей через едва заметные сдвиги яркости + тонкие границы с низкой прозрачностью.

## Цвета

Все цвета живут в `src/ruvox/ui/themes.py` — `ThemeDefinition` + QSS + palette.
Здесь фиксируется только **логика** токенов, не конкретные значения.

### Токены (семантические роли)

| Роль | Где используется |
|------|-----------------|
| `accent` | Playing bar, slider handle/fill, hover border, selection |
| `highlight_bg` | Фон подсвеченного слова при чтении |
| `secondary_text` | Info-метки в очереди, caption |
| `icon_color` | Цвет иконок кнопок плеера (enabled) |
| `icon_disabled_color` | Цвет иконок кнопок плеера (disabled) |
| `markdown_css` | CSS для HTML-рендера в QTextBrowser |

### Правило для новых тем

Для каждой темы все цвета должны быть из **одной температуры**:
- Тёмные темы: поверхности отличаются только lightness (не hue)
- Акцент — единственный цветовой акцент, всё остальное — оттенки серого/нейтрального
- `highlight_bg` должен быть заметен, но не кричать — приглушённый оттенок акцента

---

## Spacing

**Base unit:** 8px

| Name | Value | Использование |
|------|-------|---------------|
| `xs` | 2px | Микро-отступы (иконка-текст) |
| `sm` | 4px | Внутренние отступы мелких элементов, item padding |
| `md` | 8px | Стандартный spacing в layout |
| `lg` | 12px | Padding кнопок, контролов |
| `xl` | 16px | Padding панелей, секций |
| `2xl` | 24px | Разделение секций |

## Border Radius

| Name | Value | Использование |
|------|-------|---------------|
| `sm` | 4px | Инпуты, комбобоксы, мелкие элементы |
| `md` | 6px | Кнопки |
| `lg` | 8px | Диалоги |

## Typography

**UI шрифт:** системный sans-serif
**Текст чтения:** пропорциональный, системный
**Метаданные:** моноширинный для timestamp/duration

| Level | Size | Роль |
|-------|------|------|
| `body` | 14px | Основной UI текст |
| `body-reading` | 15px | Текст в TextViewer |
| `caption` | 12px | Метки, info |
| `small` | 11px | Вторичные метки (info_label) |
| `micro` | 7px | Speed-кнопки (objectName `speed_btn`) |

## Layout

### MainWindow

```
[PlayerWidget          ]  ← фиксированная высота
[QueueList | TextViewer]  ← QSplitter (1:2)
[QStatusBar            ]
```

### QueueItemWidget

```
[3px accent bar] [status 20px] [text_preview — primary text]
                               [info_label — secondary, small size]
```

### PlayerWidget

```
[progress slider                                              ]
[time_current] [prev|seek-|play|seek+|next] [speed ▲▼] [vol] [time_total]
```

- Все иконки: QPainterPath (`src/ruvox/ui/icons.py`), QIcon с Normal + Disabled pixmaps
- Основные кнопки: 32x32px (play — 48x32px), filled shapes
- Speed-кнопки: 24x13px, filled треугольники вверх/вниз (icon_arrow_up/down)
- Volume slider: 80px

## States

Каждый интерактивный элемент:
- **Default:** базовый стиль
- **Hover:** border-color → accent, или осветление фона
- **Pressed:** затемнение от hover
- **Focus:** border-color → accent (полупрозрачный)
- **Disabled:** muted text, пониженная opacity
