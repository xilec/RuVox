## MODIFIED Requirements

### Requirement: Normalization explainer

The dialog SHALL show, on every open, a short explainer line (one–two
sentences, localized) between the header and the panes stating in user terms
what normalization is and what the two panes are: RuVox rewrites technical
text (English identifiers, abbreviations, numbers, URLs, operators, code) so
the speech engine can narrate it in Russian; the left pane is the source and
the right pane is what will actually be spoken. The line SHALL NOT displace
or obscure any existing control; the dialog's minimum size and layout stay
as specified elsewhere.

The header SHALL also carry a small help affordance (an icon button) with a
click-toggled popover containing the fuller explanation: what categories get
rewritten, what the source-format selector (Авто / Обычный текст / Markdown / HTML) controls, and that fenced code block narration follows the code
block narration setting from Settings — «Кратко» replaces each block with a
brief marker sentence («далее следует пример кода на <язык>»), «Читать
полностью» reads identifiers and operators out loud; Mermaid blocks always
become the «Тут мермэйд диаграмма» marker. The popover copy SHALL NOT
mention any in-text directives. The popover SHALL include a link that opens
the README's normalization section in the system browser. The affordance
MUST NOT intercept the header's drag behavior: dragging by the icon area is
not required, but clicking it SHALL toggle the popover, not move or resize
the window, and the icon SHALL expose an accessible name (aria-label).

Both the explainer line and the tooltip copy SHALL come from the i18n
dictionaries (`preview.explain.*` keys), Russian and English.

#### Scenario: Explainer is visible on open

- GIVEN `preview_dialog_enabled` is `true` and the user opens the Add flow
- WHEN the preview dialog appears
- THEN a short explainer line about what normalization does is visible
  between the header and the panes, and the existing controls are all
  reachable

#### Scenario: Help affordance shows details and opens the README

- GIVEN the preview dialog is open
- WHEN the user activates the header help icon and then activates the link in
  its popover
- THEN a fuller explanation (rewritten categories, source-format selector,
  code-block narration) is shown, and the system browser opens the README's
  normalization section; the dialog state is unchanged

#### Scenario: Copy is localized

- GIVEN the UI language is `ru` or `en`
- WHEN the dialog is open
- THEN both the explainer line and the popover text are rendered in the
  active language from the `preview.explain.*` i18n keys
