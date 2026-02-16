# Craft in Action (PyQt6)

This shows how the subtle layering principle translates to real decisions in PyQt6. Learn the thinking, not the code. Your values will differ — the approach won't.

---

## The Subtle Layering Mindset

Before looking at any example, internalize this: **you should barely notice the system working.**

The craft is invisible — that's how you know it's working.

---

## Example: Main Window with Sidebar and Popup Menu

### The Surface Decisions

**Why so subtle?** Each elevation jump should be only a few percentage points of lightness. You can barely see the difference in isolation. But when surfaces stack, the hierarchy emerges — whisper-quiet shifts that you feel rather than see.

```css
/* QSS — Dark theme surface hierarchy */
QMainWindow { background: #1a1a1a; }                 /* Level 0: base */
QFrame#sidebar { background: #1a1a1a; }              /* Same as base! */
QGroupBox { background: #1f1f1f; }                   /* Level 1: +5% */
QMenu { background: #252525; }                       /* Level 2: +10% */
QToolTip { background: #2a2a2a; }                    /* Level 3: +15% */
```

**What NOT to do:** Don't make dramatic jumps between elevations. Don't use different hues for different levels. Keep the same hue, shift only lightness.

### The Border Decisions

**Why rgba, not solid colors?** Low opacity borders blend with their background. `rgba(255, 255, 255, 0.06)` is barely there — it defines the edge without demanding attention.

```css
QFrame#sidebar {
    border-right: 1px solid rgba(255, 255, 255, 0.06);
}
QGroupBox {
    border: 1px solid rgba(255, 255, 255, 0.04);
}
QMenu {
    border: 1px solid rgba(255, 255, 255, 0.10);  /* overlays need slightly more */
}
```

### The Sidebar Decision

**Why same background as main, not different?**

Many apps make the sidebar a different color. This fragments the visual space. Better: Same background, subtle border separation. The sidebar is part of the app, not a separate region.

### The Menu/Popup Decision

**Why Level 2, not Level 1?**

A QMenu floats above the widget it emerged from. If both share the same background, the menu blends in — you lose the sense of layering. Level 2 is just light enough to feel "above" without being dramatically different.

---

## Example: Form Controls in PyQt6

### Input Background Decision

**Why darker, not lighter?**

Inputs are "inset" — they receive content. A slightly darker background signals "type here" without needing heavy borders:

```css
QLineEdit, QTextEdit, QPlainTextEdit {
    background: rgba(0, 0, 0, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 4px;
    padding: 6px 8px;
    color: #e0e0e0;
    selection-background-color: rgba(74, 158, 255, 0.3);
}
```

### Focus State Decision

**Why subtle focus states?**

Focus needs to be visible, but not dramatic. A noticeable increase in border opacity is enough:

```css
QLineEdit:focus, QTextEdit:focus {
    border: 1px solid rgba(74, 158, 255, 0.4);
}
```

---

## Example: Complete Widget in Python

```python
class StatusCard(QFrame):
    """A card showing a status metric with craft."""

    def __init__(self, title: str, value: str, parent=None):
        super().__init__(parent)
        self.setObjectName("statusCard")

        layout = QVBoxLayout(self)
        layout.setContentsMargins(16, 16, 16, 16)
        layout.setSpacing(4)

        title_label = QLabel(title)
        title_label.setFont(QFont("Your Font", 11, QFont.Weight.Medium))
        title_label.setStyleSheet("color: #808080;")  # secondary text

        value_label = QLabel(value)
        value_label.setFont(QFont("Your Font", 24, QFont.Weight.Bold))
        value_label.setStyleSheet("color: #e0e0e0;")  # primary text

        layout.addWidget(title_label)
        layout.addWidget(value_label)

# QSS for the card
STATUS_CARD_QSS = """
QFrame#statusCard {
    background: #1f1f1f;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
}
"""
```

---

## Adapt to Context

Your product might need:
- Warmer hues (slight yellow/orange tint to backgrounds)
- Cooler hues (blue-gray base)
- Different lightness progression
- Light mode (principles invert — higher elevation = shadow, not lightness)

**The principle is constant:** barely different, still distinguishable. The values adapt to context.

---

## The Craft Check

Apply the squint test to your work:

1. Blur your eyes or step back
2. Can you still perceive hierarchy?
3. Is anything jumping out at you?
4. Can you tell where regions begin and end?

If hierarchy is visible and nothing is harsh — the subtle layering is working.
