---
name: interface-design:extract
description: Extract design patterns from existing PyQt6 code to create a system.md file.
---

# interface-design extract

Extract design patterns from existing code to create a system.

## Usage

```
/interface-design:extract          # Extract from src/ruvox/ui/
/interface-design:extract <path>   # Extract from specific directory
```

## What to Extract

**Scan Python and QSS files for:**

1. **Repeated spacing values**
   ```
   Found in setSpacing/setContentsMargins: 4 (5x), 8 (12x), 12 (3x), 16 (18x), 24 (4x)
   → Suggests: Base 4px, Scale: 4, 8, 12, 16, 24
   ```

2. **Repeated radius values**
   ```
   Found in QSS border-radius: 4px (15x), 6px (3x), 8px (2x)
   → Suggests: Radius scale: 4px, 6px, 8px
   ```

3. **Button patterns**
   ```
   Found QPushButton styles:
   - Heights: 32px (8x), 36px (3x)
   - Padding: 8px 16px (7x)
   → Suggests: Button pattern: 32px h, 8px 16px padding
   ```

4. **Color palette**
   ```
   Found unique colors: #1a1a1a (base), #222222 (elevated), #4a9eff (accent)...
   → Suggests: Dark theme with blue accent
   ```

5. **Depth strategy**
   ```
   QGraphicsDropShadowEffect found: 1x
   border in QSS found: 28x
   → Suggests: Borders-only depth
   ```

6. **Typography**
   ```
   Found QFont calls: "Segoe UI" 13pt (12x), "JetBrains Mono" 12pt (3x)
   → Suggests: Body: Segoe UI 13pt, Data: JetBrains Mono 12pt
   ```

**Then prompt:**
```
Extracted patterns:

Spacing:
  Base: 4px
  Scale: 4, 8, 12, 16, 24

Depth: Borders-only (28 borders, 1 shadow)

Typography:
  Body: Segoe UI, 13pt
  Data: JetBrains Mono, 12pt

Colors: [list]

Widget Patterns:
  QPushButton: 32px h, 8px 16px pad, 4px radius
  QGroupBox: 1px border, 16px pad

Create .interface-design/system.md with these? (y/n/customize)
```

## Implementation

1. Glob for .py and .qss files in UI directory
2. Parse for setContentsMargins, setSpacing, QFont, setStyleSheet, border-radius, background, color, QGraphicsDropShadowEffect
3. Count frequencies of repeated values
4. Identify common patterns
5. Suggest system based on frequency
6. Offer to create system.md
7. Let user customize before saving
