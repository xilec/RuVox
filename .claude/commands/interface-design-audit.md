---
name: interface-design:audit
description: Check existing PyQt6 code against your design system for spacing, depth, color, and pattern violations.
---

# interface-design audit

Check existing code against your design system.

## Usage

```
/interface-design:audit <path>     # Audit specific file/directory
/interface-design:audit            # Audit src/ruvox/ui/
```

## What to Check

**If `.interface-design/system.md` exists:**

1. **Spacing violations**
   - Find `setContentsMargins()` and `setSpacing()` values not on defined grid
   - Find hardcoded pixel values in QSS not matching the scale
   - Example: `setSpacing(14)` when base is 4px

2. **Depth violations**
   - Borders-only system → flag `QGraphicsDropShadowEffect` usage
   - Shadow system → flag mixed approaches

3. **Color violations**
   - If palette defined → flag hex colors not in palette (in QSS and Python)
   - Check `QColor()`, `setStyleSheet()`, and `.qss` files

4. **Pattern drift**
   - Find QPushButton styles not matching Button pattern
   - Find QGroupBox/QFrame not matching Card pattern
   - Find QFont() calls not using defined typography constants

**Report format:**
```
Audit Results: src/ruvox/ui/

Violations:
  widgets/player.py:45 - setSpacing(14) (grid: 4px, nearest: 12 or 16)
  widgets/queue_list.py:88 - QGraphicsDropShadowEffect (system: borders-only)
  main_window.py:120 - Hardcoded #3a3a3a not in palette

Suggestions:
  - Adjust spacing to grid multiple
  - Replace shadow with border
  - Use Colors.BG_ELEVATED constant
```

**If no system.md:**

```
No design system to audit against.

Create a system first:
1. Build UI → establish system automatically
2. Run /interface-design:extract → create system from existing code
```

## Implementation

1. Check for system.md
2. Parse system rules
3. Read target files (.py, .qss)
4. Search for: setContentsMargins, setSpacing, QFont, setStyleSheet, QColor, QGraphicsDropShadowEffect, border, background, color in QSS
5. Compare against rules
6. Report violations with suggestions
