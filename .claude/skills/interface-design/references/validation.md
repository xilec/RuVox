# Memory Management

When and how to update `.interface-design/system.md`.

## When to Add Patterns

Add to system.md when:
- Widget pattern used 2+ times
- Pattern is reusable across the project
- Has specific measurements worth remembering (margins, sizes, fonts)

## Pattern Format

```markdown
### QPushButton Primary
- Height: 36px (minimum)
- Padding: 8px 16px (via QSS)
- Radius: 4px
- Font: 13px, Medium weight
- QSS: background: #4a9eff; color: #ffffff;
```

## Don't Document

- One-off widgets
- Temporary experiments
- Variations better handled with parameters/properties

## Pattern Reuse

Before creating a widget, check system.md:
- Pattern exists? Use it.
- Need variation? Extend, don't create new.

Memory compounds: each pattern saved makes future work faster and more consistent.

---

# Validation Checks

If system.md defines specific values, check consistency:

**Spacing** — All `setContentsMargins()` and `setSpacing()` values multiples of the defined base?

**Depth** — Using the declared strategy throughout? (borders-only means no QGraphicsDropShadowEffect)

**Colors** — Using defined palette constants, not random hex codes in QSS?

**Patterns** — Reusing documented widget styles instead of creating new?

**Typography** — Using defined QFont factories, not ad-hoc QFont() calls?
