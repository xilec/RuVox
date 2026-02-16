# Core Craft Principles (PyQt6)

These apply regardless of design direction. This is the quality floor.

---

## Surface & Token Architecture

Professional interfaces don't pick colors randomly — they build systems. Understanding this architecture is the difference between "looks okay" and "feels like a real product."

### The Primitive Foundation

Every color in your interface should trace back to a small set of primitives:

- **Foreground** — text colors (primary, secondary, muted)
- **Background** — surface colors (base, elevated, overlay)
- **Border** — edge colors (default, subtle, strong)
- **Brand** — your primary accent
- **Semantic** — functional colors (destructive, warning, success)

Don't invent new colors. Map everything to these primitives. In PyQt6, define them as constants:

```python
class Colors:
    # Foreground
    TEXT_PRIMARY = "#e0e0e0"
    TEXT_SECONDARY = "#a0a0a0"
    TEXT_MUTED = "#606060"

    # Background
    BG_BASE = "#1a1a1a"
    BG_ELEVATED = "#222222"
    BG_OVERLAY = "#2a2a2a"

    # Border
    BORDER_DEFAULT = "rgba(255, 255, 255, 0.08)"
    BORDER_SUBTLE = "rgba(255, 255, 255, 0.05)"
    BORDER_STRONG = "rgba(255, 255, 255, 0.15)"

    # Brand
    ACCENT = "#4a9eff"

    # Semantic
    DESTRUCTIVE = "#e55555"
    WARNING = "#e5a555"
    SUCCESS = "#55b577"
```

### Surface Elevation Hierarchy

Surfaces stack. A QMenu sits above a QGroupBox which sits above the QMainWindow. Build a numbered system:

```
Level 0: Base background (the app canvas — QMainWindow)
Level 1: Panels, group boxes (same visual plane as base)
Level 2: Popups, menus, combo dropdowns (floating above)
Level 3: Nested menus, tooltips
```

In dark mode, higher elevation = slightly lighter. The principle: **elevated surfaces need visual distinction from what's beneath them.**

### The Subtlety Principle

The difference between elevation levels should be subtle — a few percentage points of lightness, not dramatic jumps.

**For surfaces:** Surface-1 might be 7% lighter than base, surface-2 might be 9%, surface-3 might be 12%. You can barely see it, but you feel it.

**For borders:** Use low opacity (`rgba(255, 255, 255, 0.06)` for dark mode). The border should disappear when you're not looking for it, but be findable when you need to understand the structure.

**Common mistakes to avoid:**
- Borders that are too visible (solid `#555` instead of subtle `rgba`)
- Surface jumps that are too dramatic
- Using different hues for different surfaces (gray panel on blue background)
- Harsh separators where subtle borders would do

### Text Hierarchy via Constants

Build four levels:

- **Primary** — default text, highest contrast
- **Secondary** — supporting text, slightly muted
- **Tertiary** — metadata, timestamps, less important
- **Muted** — disabled, placeholder, lowest contrast

Apply via QSS:

```css
QLabel { color: #e0e0e0; }                    /* primary */
QLabel[role="secondary"] { color: #a0a0a0; }  /* secondary */
QLabel[role="tertiary"] { color: #707070; }    /* tertiary */
QLabel:disabled { color: #505050; }            /* muted */
```

Or via `setProperty("role", "secondary")` in Python code.

### Dedicated Control Tokens

Form controls (QLineEdit, QComboBox, QSpinBox) have specific needs. Create dedicated styles:

```css
QLineEdit {
    background: rgba(0, 0, 0, 0.2);        /* slightly darker = inset */
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    padding: 6px 8px;
    color: #e0e0e0;
}
QLineEdit:focus {
    border: 1px solid rgba(74, 158, 255, 0.5);  /* accent on focus */
}
```

---

## Spacing System

Pick a base unit (4px is common for Qt) and use multiples throughout:

```python
class Spacing:
    XS = 4    # micro: icon gaps, tight pairs
    SM = 8    # component: within buttons, inputs
    MD = 12   # between related items
    LG = 16   # section spacing
    XL = 24   # major separation
    XXL = 32  # between distinct areas
```

Apply consistently:

```python
layout.setSpacing(Spacing.SM)
layout.setContentsMargins(Spacing.LG, Spacing.LG, Spacing.LG, Spacing.LG)
```

## Symmetrical Padding

`setContentsMargins(16, 16, 16, 16)` — all sides match. Exception: when content naturally creates visual balance (e.g., horizontal padding slightly larger than vertical).

## Border Radius Consistency

Sharper corners feel technical, rounder corners feel friendly. Build a scale:

```python
class Radius:
    SM = 4    # inputs, buttons
    MD = 6    # cards, group boxes
    LG = 8    # dialogs, large containers
```

## Depth & Elevation Strategy

Choose ONE and commit:

**Borders-only (flat)** — Clean, technical, dense:
```css
QFrame { border: 1px solid rgba(255, 255, 255, 0.06); }
```

**Subtle shadows** — Soft lift:
```python
shadow = QGraphicsDropShadowEffect()
shadow.setBlurRadius(8)
shadow.setOffset(0, 2)
shadow.setColor(QColor(0, 0, 0, 30))
widget.setGraphicsEffect(shadow)
```

**Surface color shifts** — Background tints without shadows:
```css
QGroupBox { background: #222222; }  /* slightly lighter than #1a1a1a base */
```

## Typography Hierarchy

```python
class Fonts:
    @staticmethod
    def headline():
        f = QFont("Your Font", 16)
        f.setWeight(QFont.Weight.Bold)
        f.setLetterSpacing(QFont.SpacingType.PercentageSpacing, 98)
        return f

    @staticmethod
    def body():
        return QFont("Your Font", 13, QFont.Weight.Normal)

    @staticmethod
    def label():
        return QFont("Your Font", 11, QFont.Weight.Medium)

    @staticmethod
    def data():
        f = QFont("JetBrains Mono", 12)
        f.setStyleStrategy(QFont.StyleStrategy.PreferAntialias)
        return f
```

## Animation

```python
anim = QPropertyAnimation(widget, b"geometry")
anim.setDuration(200)
anim.setEasingCurve(QEasingCurve.Type.OutCubic)
```

Keep durations 150-250ms. Avoid spring/bounce in professional interfaces.

## States via QSS Pseudo-States

```css
QPushButton {
    background: #2a2a2a;
    border: 1px solid rgba(255, 255, 255, 0.08);
}
QPushButton:hover {
    background: #333333;
    border: 1px solid rgba(255, 255, 255, 0.12);
}
QPushButton:pressed {
    background: #1a1a1a;
}
QPushButton:disabled {
    background: #1e1e1e;
    color: #505050;
}
```

## Dark Mode

In PyQt6, implement via QPalette or QSS theme switching:

```python
def apply_dark_theme(app: QApplication):
    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor("#1a1a1a"))
    palette.setColor(QPalette.ColorRole.WindowText, QColor("#e0e0e0"))
    palette.setColor(QPalette.ColorRole.Base, QColor("#151515"))
    palette.setColor(QPalette.ColorRole.AlternateBase, QColor("#222222"))
    palette.setColor(QPalette.ColorRole.Text, QColor("#e0e0e0"))
    palette.setColor(QPalette.ColorRole.Button, QColor("#2a2a2a"))
    palette.setColor(QPalette.ColorRole.ButtonText, QColor("#e0e0e0"))
    palette.setColor(QPalette.ColorRole.Highlight, QColor("#4a9eff"))
    palette.setColor(QPalette.ColorRole.HighlightedText, QColor("#ffffff"))
    app.setPalette(palette)
```

Or load a QSS stylesheet:

```python
app.setStyleSheet(Path("theme.qss").read_text())
```
